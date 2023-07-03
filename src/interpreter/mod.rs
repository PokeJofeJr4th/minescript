use std::path::Path;

use crate::types::prelude::*;

mod block;
mod macros;
mod operation;
mod selector_block;

pub fn interpret(src: &Syntax, path: &Path) -> SResult<InterRepr> {
    let mut state = InterRepr::new();
    inner_interpret(src, &mut state, path)?;
    Ok(state)
}

fn inner_interpret(src: &Syntax, state: &mut InterRepr, path: &Path) -> SResult<Vec<Command>> {
    match src {
        // []
        Syntax::Array(statements) => {
            let mut commands_buf = Vec::new();
            for statement in statements.iter() {
                commands_buf.extend(inner_interpret(statement, state, path)?);
            }
            return Ok(commands_buf);
        }
        // function x {}
        Syntax::Function(func, content) => {
            let inner = inner_interpret(content, state, path)?;
            state.functions.push((func.clone(), inner));
        }
        // if x=1 {}
        Syntax::Block(BlockType::If, left, op, right, block) => {
            return block::interpret_if(
                false,
                left,
                *op,
                right,
                inner_interpret(block, state, path)?,
                &format!("{:x}", get_hash(block)),
                state,
            )
        }
        // unless x=1 {}
        Syntax::Block(BlockType::Unless, left, op, right, block) => {
            return block::interpret_if(
                true,
                left,
                *op,
                right,
                inner_interpret(block, state, path)?,
                &format!("{:x}", get_hash(block)),
                state,
            )
        }
        // for _ in 1..10 {}
        Syntax::Block(block_type, left, op, right, block) => {
            return block::block(*block_type, left, *op, right, block, state, path)
        }
        // @function x
        Syntax::Macro(name, properties) => return macros::macros(name, properties, state, path),
        // @s::xp += 1
        Syntax::BinaryOp(OpLeft::SelectorDoubleColon(sel, ident), op, right) => {
            return operation::double_colon(sel, ident, *op, right)
        }
        // x += 1
        Syntax::BinaryOp(target, op, syn) => return operation::operation(target, *op, syn, state),
        // tp @s (~ ~10 ~)
        Syntax::SelectorBlock(block_type, selector, body) => {
            return selector_block::block(*block_type, selector, body, state, path)
        }
        // on owner {...}
        Syntax::IdentBlock(block_type, ident, body) => {
            return block::ident_block(*block_type, ident.clone(), body, state, path)
        }
        // Syntax::Identifier(_) => todo!(),
        Syntax::Unit => {}
        other => return Err(format!("Unexpected item `{other:?}`")),
    }
    Ok(Vec::new())
}

/// This function allows a test to expose `inner_interpret` without interacting with `IntermediateRepr`
///
/// It should not be used for any real application, since the side effects on the state are vital to the project's function
#[cfg(test)]
pub fn test_interpret(src: &Syntax) -> SResult<Vec<Command>> {
    inner_interpret(src, &mut InterRepr::new(), Path::new(""))
}
