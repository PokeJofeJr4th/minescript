use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use crate::{types::prelude::*, Config};

/// handles annotations like `@item {...}`
mod annotations;
/// handles blocks of the form `if <condition> {...}`
mod block;
/// handles operations like `counter += 1;`
mod operation;
/// handles selector blocks like `as @s {...}`
mod selector_block;
/// handles operations like `counter := @function "get_count"` or `success ?= @function "try_something"`
mod store;

pub fn interpret(
    src: &Syntax,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
    config: &Config,
) -> SResult<InterRepr> {
    let mut state = InterRepr::new(config);
    inner_interpret(src, &mut state, path, src_files, config)?;
    Ok(state)
}

fn inner_interpret(
    src: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
    config: &Config,
) -> SResult<VecCmd> {
    match src {
        // []
        Syntax::Array(statements) => {
            let mut commands_buf = VecCmd::default();
            for statement in statements.iter() {
                commands_buf.extend(inner_interpret(statement, state, path, src_files, config)?);
            }
            return Ok(commands_buf);
        }
        Syntax::BinaryOp {
            lhs,
            operation: operation @ (Operation::ColonEq | Operation::QuestionEq),
            rhs,
        } => {
            let (mut commands, lhs) = get_data_location(lhs)?;
            commands.extend(store::storage_op(
                lhs,
                *operation == Operation::QuestionEq,
                inner_interpret(rhs, state, path, src_files, config)?,
                &format!("__internal__/{:x}", get_hash(rhs)),
                state,
                config,
            )?);
            return Ok(commands);
        }
        Syntax::BinaryOp {
            lhs,
            operation: op,
            rhs,
        } => return operation::operation(lhs, *op, rhs, state, config),
        Syntax::Block(block_type, lhs, rhs) => {
            return block::block(*block_type, lhs, rhs, state, path, src_files, config)
        }
        // @function x
        Syntax::Annotation(name, properties) => {
            return annotations::annotations(name, properties, state, path, src_files, config)
        }
        Syntax::Unit => {}
        other => return Err(format!("Unexpected item `{other:?}`")),
    }
    Ok(VecCmd::default())
}

fn get_data_location(src: &Syntax) -> SResult<(VecCmd, DataLocation)> {
    if let Ok(data) = DataLocation::try_from(src.clone()) {
        return Ok((VecCmd::default(), data));
    }
    match src {
        other => Err(format!("Can't get data location from `{other:?}`")),
    }
}

/// ## Testing Only
/// This function allows a test to expose `inner_interpret` without interacting with `IntermediateRepr`
///
/// It should normally not be used, since the side effects on the state are vital to the project's function
#[cfg(test)]
pub fn test_interpret(src: &Syntax) -> Vec<Command> {
    let config = Config {
        namespace: "test".into(),
        dummy_objective: "dummy".into(),
        fixed_point_accuracy: 100,
    };
    inner_interpret(
        src,
        &mut InterRepr::new(&config),
        Path::new(""),
        &mut BTreeSet::new(),
        &config,
    )
    .unwrap()
    .base()
    .clone()
}
