use super::InterRepr;
use crate::types::prelude::*;

/// interpret an operation, like `x += 1`
///
/// ## Panics
/// If passed a `target` with a double colon or nbt
#[allow(clippy::too_many_lines)]
pub(super) fn operation(
    target: &OpLeft,
    op: Operation,
    syn: &Syntax,
    state: &mut InterRepr,
) -> SResult<Vec<Command>> {
    let target_objective = target.stringify_scoreboard_objective()?;
    let target_name = target.stringify_scoreboard_target()?;
    if !state.objectives.contains_key(&target_objective) {
        state
            .objectives
            .insert(target_objective.clone(), "dummy".into());
    }
    match (op, syn) {
        (_, Syntax::BinaryOp(OpLeft::Selector(sel), Operation::DoubleColon, syn)) => {
            let Syntax::Identifier(ident) = &**syn else {
                return Err(format!(
                    "A selector can only be `::` indexed with `lvl` or `xp`, not `{syn:?}`"
                ));
            };
            let ident = &**ident;
            let levels = if ident == "lvl" || ident == "level" {
                true
            } else if ident == "xp" || ident == "experience" {
                false
            } else {
                return Err(format!(
                    "A selector can only be `::` indexed with `lvl` or `xp`, not `{ident}`"
                ));
            };
            // get experience into variable
            let mut vec = vec![Command::Execute {
                options: vec![ExecuteOption::StoreScore {
                    target: "%".into(),
                    objective: "dummy".into(),
                }],
                cmd: Box::new(Command::XpGet {
                    target: sel.stringify()?,
                    levels,
                }),
            }];
            // operate on the variable
            vec.extend(operation(
                target,
                op,
                &Syntax::Identifier("".into()),
                state,
            )?);
            Ok(vec)
        }
        (Operation::Equal, Syntax::SelectorNbt(selector, nbt)) => Ok(vec![Command::execute(
            vec![ExecuteOption::StoreScore {
                target: target_name,
                objective: target_objective,
            }],
            vec![Command::DataGet {
                target_type: "entity".into(),
                target: selector.stringify()?.to_string().into(),
                target_path: nbt.clone(),
            }],
            "",
            state,
        )]),
        (op, Syntax::SelectorNbt(selector, nbt)) => {
            let hash = get_hash(syn);
            let mut cmd_buf = vec![Command::Execute {
                options: vec![ExecuteOption::StoreScore {
                    target: format!("%{hash:x}").into(),
                    objective: "dummy".into(),
                }],
                cmd: Box::new(Command::DataGet {
                    target_type: "entity".into(),
                    target: selector.stringify()?.to_string().into(),
                    target_path: nbt.clone(),
                }),
            }];
            cmd_buf.extend(operation(
                target,
                op,
                &Syntax::Identifier(format!("{hash:x}").into()),
                state,
            )?);
            cmd_buf.push(Command::ScoreSet {
                target: format!("%{hash:x}").into(),
                objective: "dummy".into(),
                value: 0,
            });
            Ok(cmd_buf)
        }
        // x = y
        (op, Syntax::Identifier(ident)) => {
            if !state.objectives.contains_key(&target_objective) {
                state
                    .objectives
                    .insert(target_objective.clone(), "dummy".into());
            }
            Ok(vec![Command::ScoreOperation {
                target: target_name,
                target_objective,
                operation: op,
                source: format!("%{ident}").into(),
                source_objective: "dummy".into(),
            }])
        }
        // x = @r.y
        (op, Syntax::SelectorColon(selector, ident)) => Ok(vec![Command::ScoreOperation {
            target: target_name,
            target_objective,
            operation: op,
            source: format!("{}", selector.stringify()?).into(),
            source_objective: ident.clone(),
        }]),
        // x *= 0
        (Operation::MulEq, Syntax::Integer(0)) => Ok(vec![Command::ScoreSet {
            target: target_name,
            objective: target_objective,
            value: 0,
        }]),
        // x /= 0
        (Operation::DivEq | Operation::ModEq, Syntax::Integer(0)) => {
            Err(String::from("Can't divide by zero"))
        }
        // x = 2
        (Operation::Equal, Syntax::Integer(int)) => Ok(vec![Command::ScoreSet {
            target: target_name,
            objective: target_objective,
            value: *int,
        }]),
        // x *= 1 => nop
        (Operation::MulEq | Operation::DivEq | Operation::ModEq, Syntax::Integer(1))
        | (Operation::AddEq | Operation::SubEq, Syntax::Integer(0)) => Ok(Vec::new()),
        // x += 2
        (Operation::AddEq, Syntax::Integer(int)) => Ok(vec![Command::ScoreAdd {
            target: target_name,
            objective: target_objective,
            value: *int,
        }]),
        // x -= 2
        (Operation::SubEq, Syntax::Integer(int)) => Ok(vec![Command::ScoreAdd {
            target: target_name,
            objective: target_objective,
            value: -int,
        }]),
        // x *= 2 => x += x
        (Operation::MulEq, Syntax::Integer(2)) => Ok(vec![Command::ScoreOperation {
            source: target_name.clone(),
            source_objective: target_objective.clone(),
            target: target_name,
            target_objective,
            operation: Operation::MulEq,
        }]),
        // x %= 2
        (op, Syntax::Integer(int)) => {
            state.objectives.insert("dummy".into(), "dummy".into());
            Ok(vec![
                Command::ScoreSet {
                    target: "%".into(),
                    objective: "dummy".into(),
                    value: *int,
                },
                Command::ScoreOperation {
                    target: target_name,
                    target_objective,
                    operation: op,
                    source: "%".into(),
                    source_objective: "dummy".into(),
                },
            ])
        }
        (Operation::MulEq | Operation::DivEq, Syntax::Float(float)) => {
            let approx = farey_approximation(
                if op == Operation::MulEq {
                    *float
                } else {
                    1.0 / *float
                },
                100,
            );
            Ok(vec![
                Command::ScoreSet {
                    target: "%".into(),
                    objective: "dummy".into(),
                    value: approx.0,
                },
                Command::ScoreOperation {
                    target: target_name.clone(),
                    target_objective: target_objective.clone(),
                    operation: Operation::MulEq,
                    source: "%".into(),
                    source_objective: "dummy".into(),
                },
                Command::ScoreSet {
                    target: "%".into(),
                    objective: "dummy".into(),
                    value: approx.1,
                },
                Command::ScoreOperation {
                    target: target_name,
                    target_objective,
                    operation: Operation::DivEq,
                    source: "%".into(),
                    source_objective: "dummy".into(),
                },
            ])
        }
        _ => Err(format!("Unsupported operation: {target:?} {op} {syn:?}")),
    }
}

pub(super) fn double_colon(
    selector: &Selector<Syntax>,
    ident: &str,
    op: Operation,
    right: &Syntax,
) -> SResult<Vec<Command>> {
    let levels = if ident == "lvl" || ident == "level" {
        true
    } else if ident == "xp" || ident == "experience" {
        false
    } else {
        return Err(format!(
            "A selector can only be `::` indexed with `lvl` or `xp`, not `{ident}`"
        ));
    };
    match (op, right) {
        (Operation::AddEq | Operation::SubEq, Syntax::Integer(int)) => {
            let amount = if op == Operation::AddEq { *int } else { -int };
            Ok(vec![Command::XpAdd {
                target: selector.stringify()?,
                amount,
                levels,
            }])
        }
        (Operation::Equal, Syntax::Integer(amount)) => Ok(vec![Command::XpSet {
            target: selector.stringify()?,
            amount: *amount,
            levels,
        }]),
        _ => Err(format!("Can't operate on XP with `{op}` `{right:?}`")),
    }
}

/// apply an operation where the left is a selector with an nbt path
pub(super) fn nbt(
    selector: &Selector<Syntax>,
    nbt: NbtPath,
    operation: Operation,
    right: &Syntax,
) -> SResult<Vec<Command>> {
    if operation != Operation::Equal {
        return Err(format!(
            "NBT operations only support the `=` operation, not `{operation}`"
        ));
    }
    match right {
        Syntax::Array(_)
        | Syntax::Object(_)
        | Syntax::String(_)
        | Syntax::Integer(_)
        | Syntax::Float(_) => Ok(vec![Command::DataSetValue {
            target_type: "entity".into(),
            target: selector.stringify()?.to_string().into(),
            target_path: nbt,
            value: Nbt::try_from(right)?.to_string().into(),
        }]),
        Syntax::SelectorNbt(rhs_sel, rhs_nbt) => Ok(vec![Command::DataSetFrom {
            target_type: "entity".into(),
            target: selector.stringify()?.to_string().into(),
            target_path: nbt,
            src_type: "entity".into(),
            src: rhs_sel.stringify()?.to_string().into(),
            src_path: rhs_nbt.clone(),
        }]),
        _ => todo!(),
    }
}
