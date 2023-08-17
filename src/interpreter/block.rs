use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use lazy_regex::lazy_regex;

use super::{inner_interpret, InterRepr};
use crate::{interpreter::operation::operation, types::prelude::*, Config};

pub(super) fn block(
    block_type: BlockType,
    lhs: &Syntax,
    body: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
    config: &Config,
) -> SResult<VecCmd> {
    match (block_type, lhs, body) {
        // if x=1 {}
        (
            BlockType::If | BlockType::Unless,
            Syntax::BinaryOp {
                lhs: left,
                operation: op,
                rhs: right,
            },
            _,
        ) => interpret_if(
            block_type == BlockType::Unless,
            left,
            *op,
            right,
            inner_interpret(body, state, path, src_files, config)?,
            &format!("__internal__/if_{:x}", get_hash(body)),
            state,
            config,
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
        ) => loop_block(
            block_type, left, *op, right, body, state, path, src_files, config,
        ),
        // switch _ { case _ { ...}* }
        (BlockType::Switch, _, Syntax::Array(arr)) => {
            switch_block(lhs, arr, state, path, src_files, config)
        }
        // tp @s (~ ~10 ~)
        (_, Syntax::Selector(selector), _) => {
            super::selector_block::block(block_type, selector, body, state, path, src_files, config)
        }
        // function do_thing { ... }
        (BlockType::Function, Syntax::Identifier(ident) | Syntax::String(ident), _) => {
            if matches!(&**ident, "load" | "tick") {
                println!("\x1b[33mWARN\x1b[0m\tDid you mean to name your function `__{ident}__` instead of `{ident}`?");
            } else if matches!(&**ident, "__load__" | "__tick__") {
                // nothing happens here
            } else if lazy_regex!("__[a-zA-Z0-9-_]+__").is_match(ident) {
                println!("\x1b[33mWARN\x1b[0m\tFunctions of the form `{ident}` may lead to undefined behavior; double-underscores are reserved for use by Minescript");
            }
            let inner = inner_interpret(body, state, path, src_files, config)?;
            state.functions.insert(ident.clone(), inner);
            Ok(VecCmd::default())
        }
        // advancement my_advancement { ... }
        (
            BlockType::Advancement,
            Syntax::Identifier(ident) | Syntax::String(ident),
            Syntax::Object(obj),
        ) => {
            let inner = advancement(ident, obj, state, path, src_files, config)?;
            state.advancements.insert(ident.clone(), inner);
            Ok(VecCmd::default())
        }
        // async do_thing { ... }
        (BlockType::Async, Syntax::Identifier(ident) | Syntax::String(ident), _) => {
            async_block(body, ident, state, path, src_files, config)
        }
        // on owner { ... }
        (
            BlockType::On | BlockType::Summon | BlockType::Anchored,
            Syntax::Identifier(ident) | Syntax::String(ident),
            _,
        ) => ident_block(
            block_type,
            ident.clone(),
            body,
            state,
            path,
            src_files,
            config,
        ),
        (BlockType::Rotated, Syntax::Array(arr), _) => {
            if let [yaw, pitch] = &arr[..] {
                rotated_block(yaw, pitch, body, state, path, src_files, config)
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
            |coord| coord_block(block_type, coord, body, state, path, src_files, config),
        ),
    }
}

/// Handle a switch statement
fn switch_block(
    lhs: &Syntax,
    arr: &[Syntax],
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
    config: &Config,
) -> SResult<VecCmd> {
    let switch_var: RStr = format!("__internal__/switch_{:x}", get_hash(&arr)).into();
    let mut cmd_buf = operation(
        &OpLeft::Ident(switch_var.clone()),
        Operation::Equal,
        lhs,
        state,
        config,
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
            inner_interpret(body, state, path, src_files, config)?,
            &format!("__internal__/case_{:x}", get_hash(body)),
            state,
            config,
        )?);
    }
    Ok(cmd_buf)
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
    config: &Config,
) -> SResult<VecCmd> {
    let invert = match block_type {
        BlockType::For | BlockType::DoWhile | BlockType::While => false,
        BlockType::DoUntil | BlockType::Until => true,
        _ => unreachable!(),
    };
    let fn_name: RStr = format!("__internal__/{:x}", get_hash(block)).into();
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
        config,
    )?;
    // this is the code that runs on each loop
    let mut body = inner_interpret(block, state, path, src_files, config)?;
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
                objective: left.stringify_scoreboard_objective(config)?,
                value: start.unwrap_or(0),
            }
            .into(),
        );
        body.push(
            Command::ScoreAdd {
                target: left.stringify_scoreboard_target()?,
                objective: left.stringify_scoreboard_objective(config)?,
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
        initial.extend(binding.clone());
    }
    // always check to restart loop at the end
    body.extend(binding);
    state.functions.insert(fn_name, body);
    Ok(initial)
}

/// get the command for an `if|unless` block
#[allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::manual_let_else
)]
fn interpret_if(
    invert: bool,
    left: &OpLeft,
    op: Operation,
    right: &Syntax,
    content: VecCmd,
    hash: &str,
    state: &mut InterRepr,
    config: &Config,
) -> SResult<VecCmd> {
    if content.is_empty() {
        println!(
            "\x1b[33mWARN\x1b[0m\t{} statement `{hash}` is empty; `{left:?} {op} {right:?}`",
            if invert { "Unless" } else { "If" }
        );
        return Ok(VecCmd::default());
    }
    let mut setter = None;
    let (target_player, target_objective) = if let (Ok(target_player), Ok(target_objective)) = (
        left.stringify_scoreboard_target(),
        left.stringify_scoreboard_objective(config),
    ) {
        (target_player, target_objective)
    } else {
        setter = Some(operation(
            &OpLeft::Ident("__if__".into()),
            Operation::Equal,
            &left.clone().into(),
            state,
            config,
        )?);
        ("%__if__".into(), config.dummy_objective.clone())
    };
    let options = match right {
        Syntax::Identifier(_) | Syntax::BinaryOp { .. } | Syntax::SelectorColon(_, _) => {
            let (source, source_objective) = match right {
                Syntax::Identifier(ident) => (ident.clone(), config.dummy_objective.clone()),
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
            if !state.objectives.contains_key(&source_objective) {
                state
                    .objectives
                    .insert(source_objective.clone(), config.dummy_objective.clone());
            }
            match op {
                // x = var
                Operation::LCaret
                | Operation::LCaretEq
                | Operation::Equal
                | Operation::RCaretEq
                | Operation::RCaret => {
                    vec![ExecuteOption::IfScoreSource {
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
                    vec![ExecuteOption::IfScoreSource {
                        invert: !invert,
                        target: target_player,
                        target_objective,
                        operation: Operation::Equal,
                        source,
                        source_objective,
                    }]
                }
                _ => return Err(format!("Can't compare to a score using `{op}`")),
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
            vec![ExecuteOption::IfScoreMatches {
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
            vec![ExecuteOption::IfScoreMatches {
                invert,
                target: target_player,
                objective: target_objective,
                lower: *left,
                upper: *right,
            }]
        }
        _ => return Err(format!("Can't check if `{{...}} {op} {right:?}`")),
    };
    let mut ret_val = Command::execute(&options, content, hash, state).into_vec();
    if let Some(setter) = setter {
        ret_val.map_with(
            |cmds, setter| {
                cmds.splice(0..0, setter.into_iter());
            },
            setter,
        );
    }
    Ok(ret_val)
}

fn advancement(
    name: &str,
    body: &BTreeMap<RStr, Syntax>,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
    config: &Config,
) -> SResult<Nbt> {
    let mut map_buf = BTreeMap::new();
    let mut reward_fn = None;
    for (k, v) in body {
        if &**k == "reward" {
            reward_fn = Some(inner_interpret(v, state, path, src_files, config)?);
        } else if &**k == "reward_each" {
            reward_fn = Some(inner_interpret(v, state, path, src_files, config)?.map(
                |mut cmds| {
                    cmds.push(Command::Raw(
                        format!("advancement revoke @s only <NAMESPACE>:{name}").into(),
                    ));
                    cmds
                },
            ));
        } else {
            map_buf.insert(k.clone(), Nbt::try_from(v)?);
        }
    }
    if let Some(reward_fn) = reward_fn {
        let rewards = map_buf.entry("rewards".into()).or_insert(nbt!({}));
        let Nbt::Object(rewards_obj) = rewards else {
            return Err(format!("Advancement rewards should be an object; got `{rewards}`"))
        };
        let fn_name: RStr = format!("advancement/{name}_{:x}", get_hash(body)).into();
        state.functions.insert(fn_name.clone(), reward_fn);
        rewards_obj.insert("function".into(), fn_name.into());
    }
    Ok(Nbt::Object(map_buf))
}

fn ident_block(
    block_type: BlockType,
    ident: RStr,
    body: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
    config: &Config,
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

    let content = inner_interpret(body, state, path, src_files, config)?;

    let (options, hash) = match block_type {
        BlockType::On => (
            ExecuteOption::On(ident),
            format!("__internal__/on_{:x}", get_hash(body)),
        ),
        BlockType::Summon => (
            ExecuteOption::Summon(ident),
            format!("__internal__/summon_{:x}", get_hash(body)),
        ),
        BlockType::Anchored => (
            ExecuteOption::Anchored(ident),
            format!("__internal__/anchored_{:x}", get_hash(body)),
        ),
        _ => unreachable!(),
    };
    Ok(Command::execute(&[options], content, &hash, state).into_vec())
}

fn coord_block(
    block_type: BlockType,
    coord: Coordinate,
    block: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
    config: &Config,
) -> SResult<VecCmd> {
    let mut opts = Vec::new();
    match block_type {
        BlockType::Facing => opts.push(ExecuteOption::FacingPos(coord)),
        BlockType::Positioned => {
            println!(
                "\x1b[33mWARN\x1b[0m\t`positioned (~ ~ ~) {{ ... }}`; This is a non-operation."
            );
            opts.push(ExecuteOption::Positioned(coord));
        }
        _ => return Err(format!("`{block_type:?}` block does not take a coordinate")),
    }
    Ok(Command::execute(
        &opts,
        inner_interpret(block, state, path, src_files, config)?,
        &format!("__internal__/{block_type}_{:x}", get_hash(block)),
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
    config: &Config,
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
    let hash = format!("__internal__/rotated_{:x}", get_hash(body));
    Ok(Command::execute(
        &[ExecuteOption::Rotated {
            yaw_rel,
            yaw,
            pitch_rel,
            pitch,
        }],
        inner_interpret(body, state, path, src_files, config)?,
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
    config: &Config,
) -> SResult<VecCmd> {
    let Syntax::Array(arr) = body else {
                // just make it a normal function
                let inner = inner_interpret(body, state, path, src_files, config)?;
                state.functions.insert(ident.clone(), inner);
                return Ok(VecCmd::default());
            };
    let mut func = ident.clone();
    let mut command_buf = VecCmd::default();
    for cmd in arr.iter() {
        if let Syntax::Annotation(id, body) = cmd {
            if &**id == "delay" {
                let Syntax::Integer(time) = &**body else {
                            return Err(format!("Expected an integer for delay; got `{body:?}`"))
                        };
                let next_func: RStr = format!("__async__/{:x}", get_hash(&func)).into();
                command_buf.push(
                    Command::Schedule {
                        func: next_func.clone(),
                        time: *time,
                        replace: false,
                    }
                    .into(),
                );
                state.functions.insert(
                    core::mem::replace(&mut func, next_func),
                    core::mem::take(&mut command_buf),
                );
                continue;
            }
        }
        command_buf.extend(inner_interpret(cmd, state, path, src_files, config)?);
    }
    state.functions.insert(func, command_buf);
    Ok(VecCmd::default())
}
