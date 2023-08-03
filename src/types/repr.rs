use std::collections::{BTreeMap, BTreeSet};

use crate::types::prelude::*;

#[derive(Debug, Clone)]
pub struct Item {
    pub name: RStr,
    pub base: RStr,
    pub nbt: Nbt,
    /// function that runs when the item is consumed
    pub on_consume: VecCmd,
    /// function that runs when the item is used
    pub on_use: VecCmd,
    /// function that runs every tick while the item is being used
    pub while_using: VecCmd,
    // function that runs every tick while the item is in the given slot
    pub slot_checks: Vec<(i32, VecCmd)>,
}

impl Default for Item {
    fn default() -> Self {
        Self {
            name: String::new().into(),
            base: String::new().into(),
            nbt: Nbt::default(),
            on_consume: VecCmd::default(),
            on_use: VecCmd::default(),
            while_using: VecCmd::default(),
            slot_checks: Vec::new(),
        }
    }
}

/// intermediate representation of most items and functions
#[derive(Debug)]
pub struct InterRepr {
    pub items: Vec<Item>,
    pub objectives: BTreeMap<RStr, RStr>,
    pub functions: Vec<(RStr, VecCmd)>,
    pub recipes: BTreeMap<RStr, (String, RStr)>,
    pub loot_tables: BTreeMap<RStr, RStr>,
    pub constants: BTreeSet<i32>,
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
            loot_tables: BTreeMap::new(),
            constants: BTreeSet::new(),
            // std_imports: BTreeSet::new(),
        }
    }
}

/// finished representation containing all of the data that should go into the file structure
#[derive(Debug, Clone, Default)]
pub struct CompiledRepr {
    pub functions: BTreeMap<RStr, Versioned<String>>,
    pub advancements: BTreeMap<RStr, String>,
    pub recipes: BTreeMap<RStr, String>,
    pub loot_tables: BTreeMap<RStr, RStr>,
}

impl CompiledRepr {
    /// writes the .mcmeta file
    pub fn new(loot_tables: BTreeMap<RStr, RStr>) -> Self {
        Self {
            functions: BTreeMap::new(),
            advancements: BTreeMap::new(),
            recipes: BTreeMap::new(),
            loot_tables,
        }
    }

    /// insert a function into the object, adding it to the end of an existing function if necessary.
    pub fn insert_fn(&mut self, name: &str, func: Versioned<String>) {
        match self.functions.get_mut(name) {
            Some(existing) => existing.map_with(|e, f| *e += &f, func),
            None => {
                self.functions.insert(name.into(), func);
            }
        }
    }
}
