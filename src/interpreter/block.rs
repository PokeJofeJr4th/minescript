use std::path::Path;

use super::{inner_interpret, InterRepr};
use crate::types::prelude::*;

/// ## Panics
/// If `block_type` is `If` or `Unless`. Use `interpret_if` for these cases
pub(super) fn block(
    block_type: BlockType,
    left: &OpLeft,
    op: Operation,
    right: &Syntax,
    block: &Syntax,
    state: &mut InterRepr,
    path: &Path,
) -> SResult<Vec<Command>> {
    let invert = match block_type {
        BlockType::For | BlockType::DoWhile | BlockType::While => false,
        BlockType::DoUntil | BlockType::Until => true,
        BlockType::If | BlockType::Unless => unreachable!(),
    };
    let fn_name: RStr = format!("closure/{:x}", get_hash(block)).into();
    // for _ in .. => replace `_` with hash
    let left = if block_type == BlockType::For && left == &OpLeft::Ident("_".into()) {
        OpLeft::Ident(get_hash(block).to_string().into())
    } else {
        left.clone()
    };
    let [goto_fn] = &interpret_if(
        invert,
            &left,
            op,
            right,
            vec![Command::Function {
                func: fn_name.clone(),
            }],
            "",
            state,
        )?[..] else {
            return Err(format!("Internal compiler error - please report this to the devs. {}{}", file!(), line!()))
        };
    let mut body = inner_interpret(block, state, path)?;
    if block_type == BlockType::For {
        let &Syntax::Range(start, _) = right else {
                return Err(format!("Expected a range in for loop; got `{right:?}`"))
            };
        body.push(Command::ScoreAdd {
            target: left.stringify_scoreboard_target()?,
            objective: left.stringify_scoreboard_objective()?,
            value: start.unwrap_or(0),
        });
    }
    // don't perform the initial check for do-while or for loops
    if block_type == BlockType::DoWhile || block_type == BlockType::For {
        body.push(Command::Function {
            func: fn_name.clone(),
        });
    } else {
        body.push(goto_fn.clone());
    }
    state.functions.push((fn_name, body));
    Ok(if block_type == BlockType::For {
        // reset the value at the end of a for loop
        vec![
            goto_fn.clone(),
            Command::ScoreSet {
                target: left.stringify_scoreboard_target()?,
                objective: left.stringify_scoreboard_objective()?,
                value: 0,
            },
        ]
    } else {
        vec![goto_fn.clone()]
    })
}

/// get the command for an `if|unless` block
#[allow(clippy::too_many_lines)]
pub(super) fn interpret_if(
    invert: bool,
    left: &OpLeft,
    op: Operation,
    right: &Syntax,
    content: Vec<Command>,
    hash: &str,
    state: &mut InterRepr,
) -> SResult<Vec<Command>> {
    if content.is_empty() {
        return Err(String::from("`if` body cannot be empty"));
    }
    let target_player = left.stringify_scoreboard_target()?;
    let target_objective = left.stringify_scoreboard_objective()?;
    let options = match right {
        Syntax::Identifier(_) | Syntax::BinaryOp(_, _, _) | Syntax::SelectorColon(_, _) => {
            let (source, source_objective) = match right {
                Syntax::Identifier(ident) => (ident.clone(), "dummy".into()),
                Syntax::BinaryOp(left, Operation::Colon, right) => match &**right {
                    Syntax::Identifier(ident) => {
                        (left.stringify_scoreboard_target()?, ident.clone())
                    }
                    _ => {
                        return Err(format!(
                            "Scoreboard must be indexed by an identifier; got {right:?}"
                        ))
                    }
                },
                Syntax::SelectorColon(selector, right) => {
                    (selector.stringify()?.to_string().into(), right.clone())
                }
                _ => return Err(format!("Can't compare to `{right:?}`")),
            };
            match op {
                // x = var
                Operation::LCaret
                | Operation::LCaretEq
                | Operation::Equal
                | Operation::RCaretEq
                | Operation::RCaret => {
                    vec![ExecuteOption::ScoreSource {
                        invert,
                        target: target_player,
                        target_objective,
                        operation: op,
                        source,
                        source_objective,
                    }]
                }
                // x != var
                Operation::BangEq => {
                    vec![ExecuteOption::ScoreSource {
                        invert: !invert,
                        target: target_player,
                        target_objective,
                        operation: Operation::Equal,
                        source,
                        source_objective,
                    }]
                }
                _ => return Err(format!("Can't compare using `{op}`")),
            }
        }
        Syntax::Integer(num) => {
            let (invert, lower, upper): (bool, Option<i32>, Option<i32>) = match op {
                // x = 1 => if x matches 1
                Operation::Equal => (invert, Some(*num), Some(*num)),
                // x >= 1 => if x matches 1..
                Operation::RCaretEq => (invert, Some(*num), None),
                // x <= 1 => if x matches ..1
                Operation::LCaretEq => (invert, None, Some(*num)),
                // x != 1 => unless x matches 1
                Operation::BangEq => (!invert, Some(*num), Some(*num)),
                // x > 1 => unless x matches ..1
                Operation::RCaret => (!invert, None, Some(*num)),
                // x < 1 => unless x matches 1..
                Operation::LCaret => (!invert, Some(*num), None),
                _ => return Err(format!("Can't evaluate `if <variable> {op} <number>`")),
            };
            vec![ExecuteOption::ScoreMatches {
                invert,
                target: target_player,
                objective: target_objective,
                lower,
                upper,
            }]
        }
        Syntax::Range(left, right) => {
            if op != Operation::In {
                return Err(format!(
                    "The only available operation for a range like `{right:?}` is `in`; not `{op}`"
                ));
            };
            vec![ExecuteOption::ScoreMatches {
                invert,
                target: target_player,
                objective: target_objective,
                lower: *left,
                upper: *right,
            }]
        }
        _ => return Err(format!("Can't end an if statement with {right:?}")),
    };
    Ok(vec![Command::execute(options, content, hash, state)])
}

/// interpret a block of the form `on attacker {...}`
pub(super) fn ident_block(
    block_type: IdentBlockType,
    ident: RStr,
    body: &Syntax,
    state: &mut InterRepr,
    path: &Path,
) -> SResult<Vec<Command>> {
    if block_type == IdentBlockType::On
        && !matches!(
            &*ident,
            "attacker"
                | "controller"
                | "leasher"
                | "origin"
                | "owner"
                | "passengers"
                | "target"
                | "vehicle"
        )
    {
        return Err(format!("Invalid `on` identifier: {ident}"));
    }

    let content = inner_interpret(body, state, path)?;

    match block_type {
        IdentBlockType::On => Ok(vec![Command::execute(
            vec![ExecuteOption::On { ident }],
            content,
            &format!("{:x}", get_hash(body)),
            state,
        )]),
        IdentBlockType::Summon => Ok(vec![Command::execute(
            vec![ExecuteOption::Summon { ident }],
            content,
            &format!("{:x}", get_hash(body)),
            state,
        )]),
        IdentBlockType::Anchored => Ok(vec![Command::execute(
            vec![ExecuteOption::Anchored { ident }],
            content,
            &format!("{:x}", get_hash(body)),
            state,
        )]),
    }
}
