use super::InterRepr;
use crate::{types::prelude::*, Config};

/// interpret an operation, like `x += 1`
pub(super) fn operation(
    lhs: &OpLeft,
    op: Operation,
    rhs: &Syntax,
    state: &mut InterRepr,
    config: &Config,
) -> SResult<VecCmd> {
    match (lhs, op, rhs) {
        // @s::xp
        (OpLeft::SelectorDoubleColon(sel, ident), _, _) => double_colon(sel, ident, op, rhs),
        // @s.name
        (OpLeft::SelectorNbt(sel, nbt), _, _) => nbt_op(
            NbtLocation::Entity(sel.stringify()?, nbt.clone()),
            op,
            rhs,
            state,
            config,
        ),
        (OpLeft::NbtStorage(nbt), _, _) => {
            nbt_op(NbtLocation::Storage(nbt.clone()), op, rhs, state, config)
        }
        // x | @s:score | var:x
        _ => simple_operation(lhs, op, rhs, state, config),
    }
}

/// Interpret an operation with a score on the left
#[allow(clippy::too_many_lines)]
fn simple_operation(
    target: &OpLeft,
    op: Operation,
    syn: &Syntax,
    state: &mut InterRepr,
    config: &Config,
) -> SResult<VecCmd> {
    let target_objective = target.stringify_scoreboard_objective(config)?;
    let target_name = target.stringify_scoreboard_target()?;
    if !state.objectives.contains_key(&target_objective) {
        state
            .objectives
            .insert(target_objective.clone(), config.dummy_objective.clone());
    }
    match (op, syn) {
        (_, Syntax::Integer(value)) => integer_operation(target_name, target_objective, op, *value, state, config),
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
            let (xp_target, xp_objective) = if op == Operation::Equal {
                (target.stringify_scoreboard_target()?, target.stringify_scoreboard_objective(config)?)
            } else {
                ("%__xp__".into(), config.dummy_objective.clone())
            };
            // get experience into variable
            let mut vec: VecCmd = vec![Command::Execute {
                options: vec![ExecuteOption::StoreScore {
                    target: xp_target,
                    objective: xp_objective,
                    is_success: false,
                }],
                cmd: Box::new(Command::XpGet {
                    target: sel.stringify()?,
                    levels,
                }),
            }].into();
            if op != Operation::Equal {
                // operate on the variable
                vec.extend(score_operation(
                    target_name,
                    target_objective,
                    op,
                    "%__xp__".into(),
                    config.dummy_objective.clone(),
                    state,
                    config
                )?);
            }
            Ok(vec)
        }
        (Operation::Equal, Syntax::SelectorNbt(selector, nbt)) => Ok(vec![Command::Execute {
            options: vec![ExecuteOption::StoreScore {
                target: target_name,
                objective: target_objective,
                is_success: false,
            }],
            cmd: Box::new(Command::DataGet(NbtLocation::Entity(selector.stringify()?, nbt.clone()))),
        }].into()),
        (op, Syntax::SelectorNbt(selector, nbt)) => {
            let mut cmd_buf: VecCmd = vec![Command::Execute {
                options: vec![ExecuteOption::StoreScore {
                    target: "%".into(),
                    objective: config.dummy_objective.clone(),
                    is_success: false
                }],
                cmd: Box::new(Command::DataGet(NbtLocation::Entity(selector.stringify()?, nbt.clone()))),
            }].into();
            cmd_buf.extend(simple_operation(
                target,
                op,
                &Syntax::Identifier("".into()),
                state,config
            )?);
            Ok(cmd_buf)
        }
        // x = y
        (op, Syntax::Identifier(ident)) => {
            score_operation(target_name, target_objective, op, format!("%{ident}").into(), config.dummy_objective.clone(), state, config)
        }
        // x = @r:y
        (op, Syntax::SelectorColon(selector, ident)) => score_operation(target_name, target_objective, op, selector.stringify()?.to_string().into(), ident.clone(), state, config),
        // x = @rand ...
        (Operation::Equal, Syntax::Annotation(mac, bound)) => {
            if !matches!(&**mac, "rand" | "random") {
                return Err(format!(
                    "The only annotation allowed in an operation is `rand`; got `{mac}`"
                ));
            }
            super::annotations::random(&Syntax::BinaryOp { lhs: target.clone(), operation: Operation::In, rhs: bound.clone() }, state, config)
        }
        // x += @rand ...
        (op, Syntax::Annotation(mac, bound)) => {
            if !matches!(&**mac, "rand" | "random") {
                return Err(format!(
                    "The only annotation allowed in an operation is `rand`; got `{mac}`"
                ));
            }
            // set an intermediate score to the random value
            let mut cmd_buf = super::annotations::random(&Syntax::BinaryOp { lhs: OpLeft::Ident("__rand__".into()), operation: Operation::In, rhs: bound.clone() }, state, config)?;
            // operate the random value into the target
            cmd_buf.extend(score_operation(target_name, target_objective, op, "%__rand__".into(), config.dummy_objective.clone(), state, config)?);
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
                    source: format!("%__const__{:x}", approx.0).into(),
                    source_objective: config.dummy_objective.clone(),
                },
                Command::ScoreOperation {
                    target: target_name,
                    target_objective,
                    operation: Operation::DivEq,
                    source: format!("%__const__{:x}", approx.1).into(),
                    source_objective: config.dummy_objective.clone(),
                },
            ].into())
        }
        // x .+= 0.1
        (op @ (Operation::FpAddEq | Operation::FpSubEq), Syntax::Float(float)) => {
            #[allow(clippy::cast_possible_truncation)]
            let actual_add_value = (*float * config.fixed_point_accuracy as f32) as i32;
            Ok(vec![
                Command::ScoreAdd {
                    target:target_name,
                    objective: target_objective,
                    value: if op == Operation::FpAddEq {
                        actual_add_value
                    } else {
                        - actual_add_value
                    } }
            ].into())},
        (Operation::FpEq, Syntax::Float(float)) => {
            #[allow(clippy::cast_possible_truncation)]
            let actual_add_value = (*float * config.fixed_point_accuracy as f32) as i32;
            Ok(vec![
                Command::ScoreSet {
                    target:target_name,
                    objective: target_objective,
                    value: actual_add_value
                }
            ].into())},
        (Operation::FpMulEq | Operation::FpDivEq, Syntax::Float(_)) => Err(format!("Can't apply operation `{op}` with a float; since you can just multiply or divide by a float, `.*=` and `./=` are reserved for operating between fixed-point variables.")),
        // x %= 0.1 => complain
        (_, Syntax::Float(_)) => Err(format!("Can't apply operation `{op}` with a float; floats can only be used in multiplication, division, and dedicated fixed-point decimal operations.")),
        _ => Err(format!("Unsupported operation: `{target:?} {op} {syn:?}`")),
    }
}

/// compile an operation where both the left and right are scores
fn score_operation(
    target_name: RStr,
    target_objective: RStr,
    op: Operation,
    src_name: RStr,
    src_objective: RStr,
    state: &mut InterRepr,
    config: &Config,
) -> SResult<VecCmd> {
    if !state.objectives.contains_key(&target_objective) {
        state
            .objectives
            .insert(target_objective.clone(), config.dummy_objective.clone());
    }
    if target_objective != src_objective && !state.objectives.contains_key(&target_objective) {
        state
            .objectives
            .insert(target_objective.clone(), config.dummy_objective.clone());
    }
    match op {
        Operation::AddEq | Operation::SubEq | Operation::MulEq | Operation::DivEq | Operation::ModEq | Operation::LCaret | Operation::RCaret | Operation::Swap | Operation::Equal => {
            Ok(vec![Command::ScoreOperation { target: target_name, target_objective, operation: op, source: src_name, source_objective: src_objective }].into())
        }
        Operation::FpMulEq => {
            state.constants.insert(config.fixed_point_accuracy);
            Ok(vec![
                Command::ScoreOperation { target: target_name.clone(), target_objective: target_objective.clone(), operation: Operation::MulEq, source: src_name, source_objective: src_objective },
                Command::ScoreOperation { target: target_name, target_objective, operation: Operation::DivEq, source: format!("%__const__{:x}", config.fixed_point_accuracy).into(), source_objective: config.dummy_objective.clone() }
            ].into())
        }
        Operation::FpDivEq => {
            state.constants.insert(config.fixed_point_accuracy);
            Ok(vec![
                Command::ScoreOperation { target: target_name.clone(), target_objective: target_objective.clone(), operation: Operation::MulEq, source: format!("%__const__{:x}", config.fixed_point_accuracy).into(), source_objective: config.dummy_objective.clone() },
                Command::ScoreOperation { target: target_name, target_objective, operation: Operation::DivEq, source: src_name, source_objective: src_objective }
            ].into())
        }
        _ => Err(format!("Can't operate `{op}` with two scores ({target_name} {target_objective} {op} {src_name} {src_objective})"))
    }
}

/// an operation with a variable and literal integer
fn integer_operation(
    target_name: RStr,
    target_objective: RStr,
    op: Operation,
    value: i32,
    state: &mut InterRepr,
    config: &Config,
) -> SResult<VecCmd> {
    match (op, value) {
        // x *= 0 => set to 0
        (Operation::MulEq, 0) => {
            println!("\x1b[33mWARN\x1b[0m\t`{{SCORE}} *= 0`; resetting score instead.");
            Ok(vec![Command::ScoreSet {
                target: target_name,
                objective: target_objective,
                value: 0,
            }]
            .into())
        }
        // x /= 0
        (Operation::DivEq | Operation::ModEq, 0) => Err(String::from("Can't divide by zero")),
        // x = 2
        (Operation::Equal, _) => Ok(vec![Command::ScoreSet {
            target: target_name,
            objective: target_objective,
            value,
        }]
        .into()),
        // x *= 1 => nop
        (Operation::MulEq | Operation::DivEq | Operation::ModEq, 1)
        | (Operation::AddEq | Operation::SubEq | Operation::FpAddEq | Operation::FpSubEq, 0) => {
            println!("\x1b[33mWARN\x1b[0m\t`{{SCORE}} {op} {value}`; This is a non-operation.");
            Ok(VecCmd::default())
        }
        // x += 2
        (Operation::AddEq, _) => Ok(vec![Command::ScoreAdd {
            target: target_name,
            objective: target_objective,
            value,
        }]
        .into()),
        // x >< 1 => complain
        (Operation::Swap, _) => Err(String::from(
            "Can't apply `><` (the swap operator) to an integer; did you mean `=`, `>`, or `<`?",
        )),
        // x -= 2
        (Operation::SubEq, _) => Ok(vec![Command::ScoreAdd {
            target: target_name,
            objective: target_objective,
            value: -value,
        }]
        .into()),
        // x *= 2 => x += x
        (Operation::MulEq, 2) => Ok(vec![Command::ScoreOperation {
            source: target_name.clone(),
            source_objective: target_objective.clone(),
            target: target_name,
            target_objective,
            operation: Operation::AddEq,
        }]
        .into()),
        // x .= 2
        (Operation::FpEq, int) => Ok(vec![Command::ScoreSet {
            target: target_name,
            objective: target_objective,
            value: int * config.fixed_point_accuracy,
        }]
        .into()),
        // x .-= 2
        (Operation::FpSubEq, int) => Ok(vec![Command::ScoreAdd {
            target: target_name,
            objective: target_objective,
            value: -int * config.fixed_point_accuracy,
        }]
        .into()),
        // x .+= 2
        (Operation::FpAddEq, int) => Ok(vec![Command::ScoreAdd {
            target: target_name,
            objective: target_objective,
            value: int * config.fixed_point_accuracy,
        }]
        .into()),
        // x %= 2
        (op, _) => {
            state.constants.insert(value);
            Ok(vec![Command::ScoreOperation {
                target: target_name,
                target_objective,
                operation: op,
                source: format!("%__const__{value:x}").into(),
                source_objective: config.dummy_objective.clone(),
            }]
            .into())
        }
    }
}

/// apply an operation on a selector indexed by double colon
fn double_colon(
    selector: &Selector<Syntax>,
    ident: &str,
    op: Operation,
    right: &Syntax,
) -> SResult<VecCmd> {
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
            }]
            .into())
        }
        (Operation::Equal, Syntax::Integer(amount)) => Ok(vec![Command::XpSet {
            target: selector.stringify()?,
            amount: *amount,
            levels,
        }]
        .into()),
        _ => Err(format!("Can't operate `{{XP}} {op} {right:?}`")),
    }
}

/// apply an operation where the left is a selector with an nbt path
fn nbt_op(
    lhs: NbtLocation,
    operation: Operation,
    rhs: &Syntax,
    state: &mut InterRepr,
    config: &Config,
) -> SResult<VecCmd> {
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
        }]
        .into()),
        (Operation::Equal, Syntax::SelectorNbt(rhs_sel, rhs_nbt)) => {
            Ok(vec![Command::DataSetFrom {
                target: lhs,
                src: NbtLocation::Entity(rhs_sel.stringify()?, rhs_nbt.clone()),
            }]
            .into())
        }
        (Operation::Equal, Syntax::NbtStorage(rhs_nbt)) => Ok(vec![Command::DataSetFrom {
            target: lhs,
            src: NbtLocation::Storage(rhs_nbt.clone()),
        }]
        .into()),
        (Operation::Equal, syn) => {
            let cmd = match syn {
                Syntax::Identifier(ident) => Ok(vec![Command::ScoreGet {
                    target: format!("%{ident}").into(),
                    objective: config.dummy_objective.clone(),
                }]),
                Syntax::SelectorColon(sel, ident) => Ok(vec![Command::ScoreGet {
                    target: sel.stringify()?.to_string().into(),
                    objective: ident.clone(),
                }]),
                _ => Err(format!("Can't operate `{{NBT}} = {syn:?}`")),
            }?;
            let hash = format!("__internal__/{:x}", get_hash(&(&lhs, syn)));
            Ok(Command::execute(
                vec![ExecuteOption::StoreNBT {
                    location: lhs,
                    is_success: false,
                }],
                cmd.into(),
                &hash,
                state,
            )
            .into_vec())
        }
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

fn swap_nbt(lhs: NbtLocation, rhs: NbtLocation) -> VecCmd {
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
    .into()
}
