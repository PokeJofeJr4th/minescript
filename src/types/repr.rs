use std::collections::BTreeMap;

use crate::types::prelude::*;

#[derive(Debug, Clone)]
pub struct Item {
    pub name: RStr,
    pub base: RStr,
    pub nbt: Nbt,
    /// function that runs when the item is consumed
    pub on_consume: Vec<Command>,
    /// function that runs when the item is used
    pub on_use: Vec<Command>,
    /// function that runs every tick while the item is being used
    pub while_using: Vec<Command>,
}

/// intermediate representation of most items and functions
#[derive(Debug)]
pub struct InterRepr {
    pub items: Vec<Item>,
    pub objectives: BTreeMap<RStr, RStr>,
    pub functions: Vec<(RStr, Vec<Command>)>,
    pub recipes: BTreeMap<RStr, String>,
    // /// all of the standard library functions it uses
    // pub std_imports: BTreeSet<RStr>,
}

impl InterRepr {
    /// Create a new, empty Intermediate Representation
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            objectives: BTreeMap::new(),
            functions: Vec::new(),
            recipes: BTreeMap::new(),
            // std_imports: BTreeSet::new(),
        }
    }
}

/// finished representation containing all of the data that should go into the file structure
#[derive(Debug, Clone, Default)]
pub struct CompiledRepr {
    pub functions: BTreeMap<RStr, String>,
    pub advancements: BTreeMap<RStr, String>,
    pub recipes: BTreeMap<RStr, String>,
    pub mcmeta: String,
}

impl CompiledRepr {
    /// writes the .mcmeta file
    pub fn new(namespace: &str) -> Self {
        Self {
            mcmeta: nbt!({
                pack: nbt!({
                    pack_format: 15,
                    description: format!("{namespace}, made with MineScript")
                })
            })
            .to_json(),
            functions: BTreeMap::new(),
            advancements: BTreeMap::new(),
            recipes: BTreeMap::new(),
        }
    }
}
