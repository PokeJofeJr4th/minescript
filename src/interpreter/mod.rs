use crate::types::prelude::*;

mod block;
mod macros;
mod operation;
mod selector_block;

pub fn interpret(src: &Syntax) -> SResult<IntermediateRepr> {
    let mut state = IntermediateRepr::new();
    inner_interpret(src, &mut state)?;
    Ok(state)
}

fn inner_interpret(src: &Syntax, state: &mut IntermediateRepr) -> SResult<Vec<Command>> {
    match src {
        // []
        Syntax::Array(statements) => {
            let mut commands_buf = Vec::new();
            for statement in statements.iter() {
                commands_buf.extend(inner_interpret(statement, state)?);
            }
            return Ok(commands_buf);
        }
        // function x {}
        Syntax::Function(func, content) => {
            let inner = inner_interpret(content, state)?;
            state.functions.push((func.clone(), inner));
        }
        // if x=1 {}
        Syntax::Block(BlockType::If, left, op, right, block) => {
            return block::interpret_if(
                left,
                *op,
                right,
                &inner_interpret(block, state)?,
                &format!("{:x}", get_hash(block)),
                state,
            )
        }
        // for _ in 1..10 {}
        Syntax::Block(block_type, left, op, right, block) => {
            return block::block(*block_type, left, *op, right, block, state)
        }
        // @function x
        Syntax::Macro(name, properties) => return macros::macros(name, properties, state),
        // x += 1
        Syntax::BinaryOp(target, op, syn) => return operation::operation(target, *op, syn, state),
        // tp @s (~ ~10 ~)
        Syntax::SelectorBlock(block_type, selector, body) => {
            return selector_block::block(*block_type, selector, body, state)
        }
        // Syntax::Identifier(_) => todo!(),
        Syntax::Unit => {}
        other => return Err(format!("Unexpected item `{other:?}`")),
    }
    Ok(Vec::new())
}

/// This function allows a test to expose `inner_interpret` without interacting with `InterRep`
#[cfg(test)]
pub fn test_interpret(src: &Syntax) -> SResult<Vec<Command>> {
    inner_interpret(src, &mut IntermediateRepr::new())
}
