use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use super::{inner_interpret, InterRepr};
use crate::types::prelude::*;

/// interpret an operation, like `x += 1`
pub(super) fn operation(
    lhs: &OpLeft,
    op: Operation,
    rhs: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
) -> SResult<Vec<Command>> {
    match (lhs, op, rhs) {
        // @s::xp
        (OpLeft::SelectorDoubleColon(sel, ident), _, _) => double_colon(sel, ident, op, rhs),
        // @s.name
        (OpLeft::SelectorNbt(sel, nbt), _, _) => {
            nbt_op(NbtLocation::Entity(sel.stringify()?, nbt.clone()), op, rhs)
        }
        (OpLeft::NbtStorage(nbt), _, _) => nbt_op(NbtLocation::Storage(nbt.clone()), op, rhs),
        // x | @s:score | var:x
        _ => simple_operation(lhs, op, rhs, state, path, src_files),
    }
}

/// Interpret an operation with a score on the left
///
/// ## Panics
/// If passed a `target` with a double colon or nbt
#[allow(clippy::too_many_lines)]
fn simple_operation(
    target: &OpLeft,
    op: Operation,
    syn: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
) -> SResult<Vec<Command>> {
    let target_objective = target.stringify_scoreboard_objective()?;
    let target_name = target.stringify_scoreboard_target()?;
    if !state.objectives.contains_key(&target_objective) {
        state
            .objectives
            .insert(target_objective.clone(), "dummy".into());
    }
    match (op, syn) {
        (_, Syntax::Integer(value)) => integer_operation(target_name, target_objective, op, *value, state),
        (_, Syntax::SelectorDoubleColon(sel, ident)) => {
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
            vec.extend(simple_operation(
                target,
                op,
                &Syntax::Identifier("".into()),
                state,
                path,
                src_files,
            )?);
            Ok(vec)
        }
        (Operation::Equal, Syntax::SelectorNbt(selector, nbt)) => Ok(vec![Command::Execute {
            options: vec![ExecuteOption::StoreScore {
                target: target_name,
                objective: target_objective,
            }],
            cmd: Box::new(Command::DataGet {
                target: NbtLocation::Entity(selector.stringify()?, nbt.clone()),
            }),
        }]),
        (op, Syntax::SelectorNbt(selector, nbt)) => {
            let mut cmd_buf = vec![Command::Execute {
                options: vec![ExecuteOption::StoreScore {
                    target: "%".into(),
                    objective: "dummy".into(),
                }],
                cmd: Box::new(Command::DataGet {
                    target: NbtLocation::Entity(selector.stringify()?, nbt.clone()),
                }),
            }];
            cmd_buf.extend(simple_operation(
                target,
                op,
                &Syntax::Identifier("".into()),
                state,
                path,
                src_files,
            )?);
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
        // x = @r:y
        (op, Syntax::SelectorColon(selector, ident)) => Ok(vec![Command::ScoreOperation {
            target: target_name,
            target_objective,
            operation: op,
            source: format!("{}", selector.stringify()?).into(),
            source_objective: ident.clone(),
        }]),
        // x = @rand ...
        (Operation::Equal, Syntax::Macro(mac, bound)) => {
            if !matches!(&**mac, "rand" | "random") {
                return Err(format!(
                    "The only macro allowed in an operation is `rand`; got `{mac}`"
                ));
            }
            inner_interpret(
                &Syntax::Macro(
                    mac.clone(),
                    Box::new(Syntax::BinaryOp(
                        target.clone(),
                        Operation::In,
                        bound.clone(),
                    )),
                ),
                state,
                path,
                src_files,
            )
        }
        // x += @rand ...
        (op, Syntax::Macro(mac, bound)) => {
            if !matches!(&**mac, "rand" | "random") {
                return Err(format!(
                    "The only macro allowed in an operation is `rand`; got `{mac}`"
                ));
            }
            // set an intermediate score to the random value
            let mut cmd_buf = inner_interpret(
                &Syntax::Macro(
                    mac.clone(),
                    Box::new(Syntax::BinaryOp(
                        OpLeft::Ident("%".into()),
                        Operation::In,
                        bound.clone(),
                    )),
                ),
                state,
                path,
                src_files,
            )?;
            // operate the random value into the target
            cmd_buf.push(Command::ScoreOperation {
                target: target.stringify_scoreboard_target()?,
                target_objective: target.stringify_scoreboard_objective()?,
                operation: op,
                source: "%%".into(),
                source_objective: "dummy".into(),
            });
            Ok(cmd_buf)
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
            state.constants.insert(approx.0);
            state.constants.insert(approx.1);
            Ok(vec![
                Command::ScoreOperation {
                    target: target_name.clone(),
                    target_objective: target_objective.clone(),
                    operation: Operation::MulEq,
                    source: format!("%const_{:x}", approx.0).into(),
                    source_objective: "dummy".into(),
                },
                Command::ScoreOperation {
                    target: target_name,
                    target_objective,
                    operation: Operation::DivEq,
                    source: format!("%const_{:x}", approx.1).into(),
                    source_objective: "dummy".into(),
                },
            ])
        }
        // x += 0.1 => complain
        (_, Syntax::Float(_)) => Err(format!("Can't apply operation `{op}` with a float; floats can only be used in multiplication and division.")),
        _ => Err(format!("Unsupported operation: `{target:?} {op} {syn:?}`")),
    }
}

/// an operation with a variable and literal integer
fn integer_operation(
    target_name: RStr,
    target_objective: RStr,
    op: Operation,
    value: i32,
    state: &mut InterRepr,
) -> SResult<Vec<Command>> {
    match (op, value) {
        // x *= 0 => set to 0
        (Operation::MulEq, 0) => Ok(vec![Command::ScoreSet {
            target: target_name,
            objective: target_objective,
            value: 0,
        }]),
        // x /= 0
        (Operation::DivEq | Operation::ModEq, 0) => Err(String::from("Can't divide by zero")),
        // x = 2
        (Operation::Equal, _) => Ok(vec![Command::ScoreSet {
            target: target_name,
            objective: target_objective,
            value,
        }]),
        // x *= 1 => nop
        (Operation::MulEq | Operation::DivEq | Operation::ModEq, 1)
        | (Operation::AddEq | Operation::SubEq, 0) => Ok(Vec::new()),
        // x += 2
        (Operation::AddEq, _) => Ok(vec![Command::ScoreAdd {
            target: target_name,
            objective: target_objective,
            value,
        }]),
        // x >< 1 => complain
        (Operation::Swap, _) => Err(String::from(
            "Can't apply `><` (the swap operator) to an integer; did you mean `=`, `>`, or `<`?",
        )),
        // x -= 2
        (Operation::SubEq, _) => Ok(vec![Command::ScoreAdd {
            target: target_name,
            objective: target_objective,
            value: -value,
        }]),
        // x *= 2 => x += x
        (Operation::MulEq, 2) => Ok(vec![Command::ScoreOperation {
            source: target_name.clone(),
            source_objective: target_objective.clone(),
            target: target_name,
            target_objective,
            operation: Operation::AddEq,
        }]),
        // x %= 2
        (op, _) => {
            state.objectives.insert("dummy".into(), "dummy".into());
            state.constants.insert(value);
            Ok(vec![Command::ScoreOperation {
                target: target_name,
                target_objective,
                operation: op,
                source: format!("%const_{value:x}").into(),
                source_objective: "dummy".into(),
            }])
        }
    }
}

/// apply an operation on a selector indexed by double colon
fn double_colon(
    selector: &Selector<Syntax>,
    ident: &str,
    op: Operation,
    right: &Syntax,
) -> SResult<Vec<Command>> {
    let levels = match ident {
        "level" | "lvl" => true,
        "xp" | "experience" => false,
        _ => {
            return Err(format!(
                "A selector can only be `::` indexed with `lvl` or `xp`, not `{ident}`"
            ))
        }
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
        _ => Err(format!("Can't operate `{{XP}} {op} {right:?}`")),
    }
}

/// apply an operation where the left is a selector with an nbt path
fn nbt_op(lhs: NbtLocation, operation: Operation, rhs: &Syntax) -> SResult<Vec<Command>> {
    match (operation, rhs) {
        (
            Operation::Equal,
            Syntax::Array(_)
            | Syntax::Object(_)
            | Syntax::String(_)
            | Syntax::Integer(_)
            | Syntax::Float(_),
        ) => Ok(vec![Command::DataSetValue {
            target: lhs,
            value: Nbt::try_from(rhs)?.to_string().into(),
        }]),
        (Operation::Equal, Syntax::SelectorNbt(rhs_sel, rhs_nbt)) => {
            Ok(vec![Command::DataSetFrom {
                target: lhs,
                src: NbtLocation::Entity(rhs_sel.stringify()?, rhs_nbt.clone()),
            }])
        }
        (Operation::Equal, Syntax::NbtStorage(rhs_nbt)) => Ok(vec![Command::DataSetFrom {
            target: lhs,
            src: NbtLocation::Storage(rhs_nbt.clone()),
        }]),
        (Operation::Swap, Syntax::NbtStorage(rhs_nbt)) => {
            Ok(swap_nbt(lhs, NbtLocation::Storage(rhs_nbt.clone())))
        }
        (Operation::Swap, Syntax::SelectorNbt(sel, rhs_nbt)) => Ok(swap_nbt(
            lhs,
            NbtLocation::Entity(sel.stringify()?, rhs_nbt.clone()),
        )),
        _ => Err(format!("Can't operate `{{NBT}} {operation} {rhs:?}`")),
    }
}

fn swap_nbt(lhs: NbtLocation, rhs: NbtLocation) -> Vec<Command> {
    vec![
        Command::DataSetFrom {
            target: NbtLocation::Storage(vec![NbtPathPart::Ident("%".into())]),
            src: lhs.clone(),
        },
        Command::DataSetFrom {
            target: lhs,
            src: rhs.clone(),
        },
        Command::DataSetFrom {
            target: rhs,
            src: NbtLocation::Storage(vec![NbtPathPart::Ident("%".into())]),
        },
    ]
}
