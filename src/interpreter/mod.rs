use std::collections::BTreeMap;

use crate::types::prelude::*;

mod block;
mod macros;
mod operation;
mod selector_block;

#[derive(Debug)]
pub struct Item {
    pub name: RStr,
    pub base: RStr,
    pub nbt: Nbt,
    /// function that runs when the item is consumed
    pub on_consume: Option<RStr>,
    /// function that runs when the item is used
    pub on_use: Option<RStr>,
    /// function that runs every tick while the item is being used
    pub while_using: Option<RStr>,
}

#[derive(Debug)]
pub struct InterRep {
    pub items: Vec<Item>,
    pub objectives: BTreeMap<RStr, RStr>,
    pub functions: Vec<(RStr, Vec<Command>)>,
    pub recipes: BTreeMap<RStr, String>,
}

impl InterRep {
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            objectives: BTreeMap::new(),
            functions: Vec::new(),
            recipes: BTreeMap::new(),
        }
    }
}

pub fn interpret(src: &Syntax) -> SResult<InterRep> {
    let mut state = InterRep::new();
    inner_interpret(src, &mut state)?;
    Ok(state)
}

fn inner_interpret(src: &Syntax, state: &mut InterRep) -> SResult<Vec<Command>> {
    match src {
        Syntax::Array(statements) => {
            let mut commands_buf = Vec::new();
            for statement in statements.iter() {
                commands_buf.extend(inner_interpret(statement, state)?);
            }
            return Ok(commands_buf);
        }
        Syntax::Macro(name, properties) => return macros::macros(name, properties, state),
        Syntax::Function(func, content) => {
            let inner = inner_interpret(content, state)?;
            state.functions.push((func.clone(), inner));
        }
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
        Syntax::Block(block_type, left, op, right, block) => {
            return block::block(*block_type, left, *op, right, block, state)
        }
        Syntax::BinaryOp(target, op, syn) => return operation::operation(target, *op, syn, state),
        Syntax::SelectorBlock(SelectorBlockType::Tp, selector, body) => {
            return selector_block::teleport(selector, body)
        }
        Syntax::SelectorBlock(SelectorBlockType::Damage, selector, body) => {
            return selector_block::damage(selector, body)
        }
        Syntax::SelectorBlock(SelectorBlockType::TellRaw, selector, body) => {
            return selector_block::tellraw(selector, body)
        }
        Syntax::SelectorBlock(block_type, selector, body) => {
            return selector_block::selector_block(*block_type, selector, body, state)
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
    inner_interpret(src, &mut InterRep::new())
}
