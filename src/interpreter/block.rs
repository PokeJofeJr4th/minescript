use super::{inner_interpret, InterRepr};
use crate::types::prelude::*;

pub(super) fn block(
    block_type: BlockType,
    left: &OpLeft,
    op: Operation,
    right: &Syntax,
    block: &Syntax,
    state: &mut InterRepr,
) -> SResult<Vec<Command>> {
    assert_ne!(block_type, BlockType::If);
    let fn_name: RStr = format!("closure/{:x}", get_hash(block)).into();
    // for _ in .. => replace `_` with hash
    let left = if block_type == BlockType::For && left == &OpLeft::Ident("_".into()) {
        OpLeft::Ident(get_hash(block).to_string().into())
    } else {
        left.clone()
    };
    let [goto_fn] = &interpret_if(
            &left,
            op,
            right,
            &[Command::Function {
                func: fn_name.clone(),
            }],
            "",
            state,
        )?[..] else {
            return Err(format!("Internal compiler error - please report this to the devs. {}{}", file!(), line!()))
        };
    let mut body = inner_interpret(block, state)?;
    if block_type == BlockType::For {
        let &Syntax::Range(start, _) = right else {
                return Err(format!("Expected a range in for loop; got `{right:?}`"))
            };
        body.push(Command::ScoreAdd {
            target: left.stringify_scoreboard_target()?,
            objective: left.stringify_scoreboard_objective(),
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
                objective: left.stringify_scoreboard_objective(),
                value: 0,
            },
        ]
    } else {
        vec![goto_fn.clone()]
    })
}

#[allow(clippy::too_many_lines)]
pub(super) fn interpret_if(
    left: &OpLeft,
    op: Operation,
    right: &Syntax,
    content: &[Command],
    hash: &str,
    state: &mut InterRepr,
) -> SResult<Vec<Command>> {
    if content.is_empty() {
        return Err(String::from("`if` body cannot be empty"));
    }
    let cmd: Command = if let [cmd] = content {
        cmd.clone()
    } else {
        let func_name: RStr = format!("closure/{hash}").into();
        state.functions.push((func_name.clone(), content.to_vec()));
        Command::Function { func: func_name }
    };
    let target_player = left.stringify_scoreboard_target()?;
    let target_objective = left.stringify_scoreboard_objective();
    let options = match right {
        Syntax::Identifier(_) | Syntax::BinaryOp(_, _, _) | Syntax::ColonSelector(_, _) => {
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
                Syntax::ColonSelector(selector, right) => {
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
                        invert: false,
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
                        invert: true,
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
                Operation::Equal => (false, Some(*num), Some(*num)),
                // x >= 1 => if x matches 1..
                Operation::RCaretEq => (false, Some(*num), None),
                // x <= 1 => if x matches ..1
                Operation::LCaretEq => (false, None, Some(*num)),
                // x != 1 => unless x matches 1
                Operation::BangEq => (true, Some(*num), Some(*num)),
                // x > 1 => unless x matches ..1
                Operation::RCaret => (true, None, Some(*num)),
                // x < 1 => unless x matches 1..
                Operation::LCaret => (true, Some(*num), None),
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
                invert: false,
                target: target_player,
                objective: target_objective,
                lower: *left,
                upper: *right,
            }]
        }
        _ => return Err(format!("Can't end an if statement with {right:?}")),
    };
    Ok(vec![Command::execute(options, cmd)])
}
