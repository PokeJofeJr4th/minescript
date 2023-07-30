use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use crate::types::prelude::*;

/// handles blocks of the form `if <condition> {...}`
mod block;
/// handles macros like `@item {...}`
mod macros;
/// handles operations like `counter += 1;`
mod operation;
/// handles selector blocks like `as @s {...}`
mod selector_block;

pub fn interpret(
    src: &Syntax,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
) -> SResult<InterRepr> {
    let mut state = InterRepr::new();
    inner_interpret(src, &mut state, path, src_files)?;
    Ok(state)
}

fn inner_interpret(
    src: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
) -> SResult<Vec<Command>> {
    match src {
        // []
        Syntax::Array(statements) => {
            let mut commands_buf = Vec::new();
            for statement in statements.iter() {
                commands_buf.extend(inner_interpret(statement, state, path, src_files)?);
            }
            return Ok(commands_buf);
        }
        Syntax::BinaryOp {
            lhs,
            operation: op,
            rhs,
        } => return operation::operation(lhs, *op, rhs, state, path, src_files),
        Syntax::Block(block_type, lhs, rhs) => {
            return block::block(*block_type, lhs, rhs, state, path, src_files)
        }
        // @function x
        Syntax::Macro(name, properties) => {
            return macros::macros(name, properties, state, path, src_files)
        }
        Syntax::Unit => {}
        other => return Err(format!("Unexpected item `{other:?}`")),
    }
    Ok(Vec::new())
}

/// ## Testing Only
/// This function allows a test to expose `inner_interpret` without interacting with `IntermediateRepr`
///
/// It should normally not be used, since the side effects on the state are vital to the project's function
#[cfg(test)]
pub fn test_interpret(src: &Syntax) -> SResult<Vec<Command>> {
    inner_interpret(
        src,
        &mut InterRepr::new(),
        Path::new(""),
        &mut BTreeSet::new(),
    )
}
