use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use super::{inner_interpret, InterRepr};
use crate::{interpreter::operation::operation, types::prelude::*};

#[allow(clippy::too_many_lines)]
pub(super) fn block(
    block_type: BlockType,
    lhs: &Syntax,
    body: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
) -> SResult<VecCmd> {
    match (block_type, lhs, body) {
        // if x=1 {}
        (
            BlockType::If,
            Syntax::BinaryOp {
                lhs: left,
                operation: op,
                rhs: right,
            },
            _,
        ) => interpret_if(
            false,
            left,
            *op,
            right,
            inner_interpret(body, state, path, src_files)?,
            &format!("closure/if_{:x}", get_hash(body)),
            state,
        ),
        // unless x=1 {}
        (
            BlockType::Unless,
            Syntax::BinaryOp {
                lhs: left,
                operation: op,
                rhs: right,
            },
            _,
        ) => interpret_if(
            true,
            left,
            *op,
            right,
            inner_interpret(body, state, path, src_files)?,
            &format!("closure/unless_{:x}", get_hash(body)),
            state,
        ),
        // for _ in 1..10 {}
        (
            BlockType::For
            | BlockType::While
            | BlockType::Until
            | BlockType::DoWhile
            | BlockType::DoUntil,
            Syntax::BinaryOp {
                lhs: left,
                operation: op,
                rhs: right,
            },
            _,
        ) => loop_block(block_type, left, *op, right, body, state, path, src_files),
        // switch _ { case _ { ...}* }
        (BlockType::Switch, _, Syntax::Array(arr)) => {
            let switch_var: RStr = format!("closure/switch_{:x}", get_hash(body)).into();
            let mut cmd_buf = operation(
                &OpLeft::Ident(switch_var.clone()),
                Operation::Equal,
                lhs,
                state,
                path,
                src_files,
            )?;
            for syn in arr.iter() {
                let Syntax::Block(BlockType::Case, match_value, body) = syn else {
                    return Err(format!("Expected `case` statement; got `{syn:?}`"))
                };
                cmd_buf.extend(interpret_if(
                    false,
                    &OpLeft::Ident(switch_var.clone()),
                    Operation::Equal,
                    match_value,
                    inner_interpret(body, state, path, src_files)?,
                    &format!("closure/case_{:x}", get_hash(body)),
                    state,
                )?);
            }
            Ok(cmd_buf)
        }
        // tp @s (~ ~10 ~)
        (_, Syntax::Selector(selector), _) => {
            super::selector_block::block(block_type, selector, body, state, path, src_files)
        }
        // function do_thing { ... }
        (BlockType::Function, Syntax::Identifier(ident) | Syntax::String(ident), _) => {
            let inner = inner_interpret(body, state, path, src_files)?;
            state.functions.push((ident.clone(), inner));
            Ok(VecCmd::default())
        }
        // async do_thing { ... }
        (BlockType::Async, Syntax::Identifier(ident) | Syntax::String(ident), _) => {
            async_block(body, ident, state, path, src_files)
        }
        // on owner {...}
        (
            BlockType::On | BlockType::Summon | BlockType::Anchored,
            Syntax::Identifier(ident) | Syntax::String(ident),
            _,
        ) => ident_block(block_type, ident.clone(), body, state, path, src_files),
        (BlockType::Rotated, Syntax::Array(arr), _) => {
            if let [yaw, pitch] = &arr[..] {
                rotated_block(yaw, pitch, body, state, path, src_files)
            } else {
                Err(format!("Expected a `rotated [{{yaw}}, {{pitch}}]` or `rotated @[{{selector}}]`; got `rotated {arr:?}`"))
            }
        }
        _ => Coordinate::try_from(lhs).map_or_else(
            |_| {
                Err(format!(
                    "Unsupported block invocation: `{block_type:?} {lhs:?} {body:?}`"
                ))
            },
            |coord| coord_block(block_type, coord, body, state, path, src_files),
        ),
    }
}

/// # Panics
/// If passed a `BlockType` other than `For`, `Until`, `DoWhile`, `While`, or `DoUntil`
#[allow(clippy::too_many_arguments)]
fn loop_block(
    block_type: BlockType,
    left: &OpLeft,
    op: Operation,
    right: &Syntax,
    block: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
) -> SResult<VecCmd> {
    let invert = match block_type {
        BlockType::For | BlockType::DoWhile | BlockType::While => false,
        BlockType::DoUntil | BlockType::Until => true,
        _ => unreachable!(),
    };
    let fn_name: RStr = format!("closure/{:x}", get_hash(block)).into();
    // for _ in .. => replace `_` with hash
    let left = if block_type == BlockType::For && left == &OpLeft::Ident("_".into()) {
        OpLeft::Ident(format!("{:x}", get_hash(block)).into())
    } else {
        left.clone()
    };
    let binding = interpret_if(
        invert,
        &left,
        op,
        right,
        vec![Command::Function(fn_name.clone())].into(),
        "",
        state,
    )?;
    let [goto_fn] = &binding.base()[..] else {
            return Err(format!("Internal compiler error - please report this to the devs, along with the following information: {}{}", file!(), line!()))
        };
    // this is the code that runs on each loop
    let mut body = inner_interpret(block, state, path, src_files)?;
    // this is the code that runs to enter the loop
    let mut initial = VecCmd::default();
    if block_type == BlockType::For {
        // reset value at start of for loop
        let &Syntax::Range(start, _) = right else {
                return Err(format!("Expected `for {{variable}} in {{range}}`; got `{right:?}`"))
            };
        initial.push(
            Command::ScoreSet {
                target: left.stringify_scoreboard_target()?,
                objective: left.stringify_scoreboard_objective()?,
                value: start.unwrap_or(0),
            }
            .into(),
        );
        body.push(
            Command::ScoreAdd {
                target: left.stringify_scoreboard_target()?,
                objective: left.stringify_scoreboard_objective()?,
                value: 1,
            }
            .into(),
        );
    }
    // don't perform the initial check for do-while, do-until, or for loops
    if matches!(
        block_type,
        BlockType::DoWhile | BlockType::DoUntil | BlockType::For
    ) {
        initial.push(Command::Function(fn_name.clone()).into());
    } else {
        initial.push(goto_fn.clone().into());
    }
    // always check to restart loop at the end
    body.push(goto_fn.clone().into());
    state.functions.push((fn_name, body));
    Ok(initial)
}

/// get the command for an `if|unless` block
fn interpret_if(
    invert: bool,
    left: &OpLeft,
    op: Operation,
    right: &Syntax,
    content: VecCmd,
    hash: &str,
    state: &mut InterRepr,
) -> SResult<VecCmd> {
    if content.is_empty() {
        return Err(String::from("`if` body cannot be empty"));
    }
    let target_player = left.stringify_scoreboard_target()?;
    let target_objective = left.stringify_scoreboard_objective()?;
    let options = match right {
        Syntax::Identifier(_)
        | Syntax::BinaryOp {
            lhs: _,
            operation: _,
            rhs: _,
        }
        | Syntax::SelectorColon(_, _) => {
            let (source, source_objective) = match right {
                Syntax::Identifier(ident) => (ident.clone(), "dummy".into()),
                Syntax::BinaryOp {
                    lhs: left,
                    operation: Operation::Colon,
                    rhs: right,
                } => match &**right {
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
                _ => return Err(format!("Can't evaluate `if {{...}} {op} {{integer}}`")),
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
                    "Can't check if `{{...}} {op} {{range}}`. Did you mean `{{...}} in {{range}}`?"
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
        _ => return Err(format!("Can't check if `{{...}} {op} {right:?}`")),
    };
    Ok(Command::execute(options, content, hash, state).into_vec())
}

fn ident_block(
    block_type: BlockType,
    ident: RStr,
    body: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
) -> SResult<VecCmd> {
    if block_type == BlockType::On
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
        return Err(format!("Invalid `on` identifier: `{ident}`; expected `attacker`, `controller`, `leasher`, `origin`, `owner`, `passengers`, `target`, or `vehicle`"));
    }

    let content = inner_interpret(body, state, path, src_files)?;

    let (options, hash) = match block_type {
        BlockType::On => (
            ExecuteOption::On(ident),
            format!("closure/on_{:x}", get_hash(body)),
        ),
        BlockType::Summon => (
            ExecuteOption::Summon(ident),
            format!("closure/summon_{:x}", get_hash(body)),
        ),
        BlockType::Anchored => (
            ExecuteOption::Anchored(ident),
            format!("closure/anchored_{:x}", get_hash(body)),
        ),
        _ => unreachable!(),
    };
    Ok(Command::execute(vec![options], content, &hash, state).into_vec())
}

fn coord_block(
    block_type: BlockType,
    coord: Coordinate,
    block: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
) -> SResult<VecCmd> {
    let mut opts = Vec::new();
    match block_type {
        BlockType::Facing => opts.push(ExecuteOption::FacingPos(coord)),
        BlockType::Positioned => opts.push(ExecuteOption::Positioned(coord)),
        _ => return Err(format!("`{block_type:?}` block does not take a coordinate")),
    }
    Ok(Command::execute(
        opts.clone(),
        inner_interpret(block, state, path, src_files)?,
        &format!("closure/{block_type}_{:x}", get_hash(block)),
        state,
    )
    .map(|c| vec![c]))
}

fn rotated_block(
    yaw: &Syntax,
    pitch: &Syntax,
    body: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
) -> SResult<VecCmd> {
    let (yaw_rel, yaw) = match yaw {
        Syntax::Integer(int) => (false, *int as f32),
        Syntax::Float(fl) => (false, *fl),
        Syntax::WooglyCoord(fl) => (true, *fl),
        other => {
            return Err(format!(
                "Expected a number or relative rotation for `rotated`; got `{other:?}`"
            ))
        }
    };
    let (pitch_rel, pitch) = match pitch {
        Syntax::Integer(int) => (false, *int as f32),
        Syntax::Float(fl) => (false, *fl),
        Syntax::WooglyCoord(fl) => (true, *fl),
        other => {
            return Err(format!(
                "Expected a number or relative rotation for `rotated`; got `{other:?}`"
            ))
        }
    };
    let hash = format!("closure/rotated_{:x}", get_hash(body));
    Ok(Command::execute(
        vec![ExecuteOption::Rotated {
            yaw_rel,
            yaw,
            pitch_rel,
            pitch,
        }],
        inner_interpret(body, state, path, src_files)?,
        &hash,
        state,
    )
    .into_vec())
}

fn async_block(
    body: &Syntax,
    ident: &RStr,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
) -> SResult<VecCmd> {
    let Syntax::Array(arr) = body else {
                // just make it a normal function
                let inner = inner_interpret(body, state, path, src_files)?;
                state.functions.push((ident.clone(), inner));
                return Ok(VecCmd::default());
            };
    let mut func = ident.clone();
    let mut command_buf = VecCmd::default();
    for cmd in arr.iter() {
        if let Syntax::Macro(id, body) = cmd {
            if &**id == "delay" {
                let Syntax::Integer(time) = &**body else {
                            return Err(format!("Expected an integer for delay; got `{body:?}`"))
                        };
                let next_func: RStr = format!("closure/async_{:x}", get_hash(&func)).into();
                command_buf.push(
                    Command::Schedule {
                        func: next_func.clone(),
                        time: *time,
                        replace: false,
                    }
                    .into(),
                );
                state.functions.push((
                    core::mem::replace(&mut func, next_func),
                    core::mem::take(&mut command_buf),
                ));
                continue;
            }
        }
        command_buf.extend(inner_interpret(cmd, state, path, src_files)?);
    }
    state.functions.push((func, command_buf));
    Ok(VecCmd::default())
}
