use super::InterRepr;
use crate::types::prelude::*;

#[allow(clippy::too_many_lines)]
pub(super) fn operation(
    target: &OpLeft,
    op: Operation,
    syn: &Syntax,
    state: &mut InterRepr,
) -> SResult<Vec<Command>> {
    let target_objective = target.stringify_scoreboard_objective();
    let target = target.stringify_scoreboard_target()?;
    if !state.objectives.contains_key(&target_objective) {
        state
            .objectives
            .insert(target_objective.clone(), "dummy".into());
    }
    match (op, syn) {
        // x = y
        (op, Syntax::Identifier(ident)) => {
            if !state.objectives.contains_key(&target_objective) {
                state
                    .objectives
                    .insert(target_objective.clone(), "dummy".into());
            }
            Ok(vec![Command::ScoreOperation {
                target,
                target_objective,
                operation: op,
                source: format!("%{ident}").into(),
                source_objective: "dummy".into(),
            }])
        }
        // x = @r.y
        (op, Syntax::ColonSelector(sel, ident)) => Ok(vec![Command::ScoreOperation {
            target,
            target_objective,
            operation: op,
            source: format!("{}", sel.stringify()?).into(),
            source_objective: ident.clone(),
        }]),
        // x *= 0
        (Operation::MulEq, Syntax::Integer(0)) => Ok(vec![Command::ScoreSet {
            target,
            objective: target_objective,
            value: 0,
        }]),
        // x /= 0
        (Operation::DivEq | Operation::ModEq, Syntax::Integer(0)) => {
            Err(String::from("Can't divide by zero"))
        }
        // x = 2
        (Operation::Equal, Syntax::Integer(int)) => Ok(vec![Command::ScoreSet {
            target,
            objective: target_objective,
            value: *int,
        }]),
        // x *= 1 => nop
        (Operation::MulEq | Operation::DivEq | Operation::ModEq, Syntax::Integer(1))
        | (Operation::AddEq | Operation::SubEq, Syntax::Integer(0)) => Ok(Vec::new()),
        // x += 2
        (Operation::AddEq, Syntax::Integer(int)) => Ok(vec![Command::ScoreAdd {
            target,
            objective: target_objective,
            value: *int,
        }]),
        // x -= 2
        (Operation::SubEq, Syntax::Integer(int)) => Ok(vec![Command::ScoreAdd {
            target,
            objective: target_objective,
            value: -int,
        }]),
        // x *= 2 => x += x
        (Operation::MulEq, Syntax::Integer(2)) => Ok(vec![Command::ScoreOperation {
            source: target.clone(),
            source_objective: target_objective.clone(),
            target,
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
                    target,
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
                    target: target.clone(),
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
                    target,
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
