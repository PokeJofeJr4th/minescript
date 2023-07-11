use std::path::Path;

use crate::types::prelude::*;

/// handles blocks of the form `if <condition> {...}`
mod block;
/// handles macros like `@item {...}`
mod macros;
/// handles operations like `counter += 1;`
mod operation;
/// handles selector blocks like `as @s {...}`
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
        // tp @s (~ ~10 ~)
        Syntax::SelectorBlock(block_type, selector, body) => {
            return selector_block::block(*block_type, selector, body, state, path)
        }
        Syntax::IdentBlock(IdentBlockType::Function, ident, body) => {
            let inner = inner_interpret(body, state, path)?;
            state.functions.push((ident.clone(), inner));
        }
        // async do_thing { ... }
        Syntax::IdentBlock(IdentBlockType::Async, ident, body) => {
            let Syntax::Array(arr) = &**body else {
                // just make it a normal function
                let inner = inner_interpret(body, state, path)?;
                state.functions.push((ident.clone(), inner));
                return Ok(Vec::new());
            };
            let mut func = ident.clone();
            let mut command_buf: Vec<Command> = Vec::new();
            for cmd in arr.iter() {
                if let Syntax::Macro(id, body) = cmd {
                    if &**id == "delay" {
                        let Syntax::Integer(time) = &**body else {
                            return Err(format!("Expected an integer for delay; got `{body:?}`"))
                        };
                        let next_func: RStr = format!("closure/async_{:x}", get_hash(&func)).into();
                        command_buf.push(Command::Schedule {
                            func: next_func.clone(),
                            time: *time,
                            replace: false,
                        });
                        state.functions.push((
                            core::mem::replace(&mut func, next_func),
                            core::mem::take(&mut command_buf),
                        ));
                        continue;
                    }
                }
                command_buf.extend(inner_interpret(cmd, state, path)?);
            }
            state.functions.push((func, command_buf));
        }
        // on owner {...}
        Syntax::IdentBlock(block_type, ident, body) => {
            return block::ident_block(*block_type, ident.clone(), body, state, path)
        }
        Syntax::BinaryOp(lhs, op, rhs) => return operation::operation(lhs, *op, rhs, state),
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
    inner_interpret(src, &mut InterRepr::new(), Path::new(""))
}
