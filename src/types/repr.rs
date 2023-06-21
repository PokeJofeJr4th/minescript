use std::collections::BTreeMap;

use crate::types::prelude::*;

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
pub struct IntermediateRepr {
    pub items: Vec<Item>,
    pub objectives: BTreeMap<RStr, RStr>,
    pub functions: Vec<(RStr, Vec<Command>)>,
    pub recipes: BTreeMap<RStr, String>,
}

impl IntermediateRepr {
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            objectives: BTreeMap::new(),
            functions: Vec::new(),
            recipes: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompiledRepr {
    pub functions: BTreeMap<RStr, String>,
    pub advancements: BTreeMap<RStr, String>,
    pub recipes: BTreeMap<RStr, String>,
    pub mcmeta: String,
}
