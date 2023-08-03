use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Write,
};

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
    pub functions: BTreeMap<RStr, String>,
    pub advancements: BTreeMap<RStr, String>,
    pub recipes: BTreeMap<RStr, String>,
    pub loot_tables: BTreeMap<RStr, RStr>,
    pub mcmeta: String,
}

impl CompiledRepr {
    /// writes the .mcmeta file
    pub fn new(namespace: &str, loot_tables: BTreeMap<RStr, RStr>) -> Self {
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
            loot_tables,
        }
    }

    /// insert a function into the object, adding it to the end of an existing function if necessary.
    pub fn insert_fn(&mut self, name: &str, func: &str) {
        match self.functions.get_mut(name) {
            Some(existing) => existing.push_str(func),
            None => {
                self.functions.insert(name.into(), func.into());
            }
        }
    }

    pub fn write(&self, parent: &str, nmsp: &str) -> Result<(), std::io::Error> {
        let _ = fs::remove_dir_all(&format!("{parent}{nmsp}"));
        for (path, contents) in &self.functions {
            let mut file = create_file_with_parent_dirs(&format!(
                "{parent}{nmsp}/data/{nmsp}/functions/{path}.mcfunction"
            ))?;
            write!(file, "{contents}")?;
            if &**path == "tick" {
                let mut tick = create_file_with_parent_dirs(&format!(
                    "{parent}{nmsp}/data/minecraft/tags/functions/tick.json"
                ))?;
                write!(tick, "{{\"values\":[\"{nmsp}:tick\"]}}")?;
            }
            if &**path == "load" {
                let mut load = create_file_with_parent_dirs(&format!(
                    "{parent}{nmsp}/data/minecraft/tags/functions/load.json"
                ))?;
                write!(load, "{{\"values\":[\"{nmsp}:load\"]}}")?;
            }
        }
        for (path, contents) in &self.advancements {
            let mut file = create_file_with_parent_dirs(&format!(
                "{parent}{nmsp}/data/{nmsp}/advancements/{path}.json"
            ))?;
            write!(file, "{contents}")?;
        }
        for (path, contents) in &self.recipes {
            let mut file = create_file_with_parent_dirs(&format!(
                "{parent}{nmsp}/data/{nmsp}/recipes/{path}.json"
            ))?;
            write!(file, "{contents}")?;
        }
        for (path, contents) in &self.loot_tables {
            let mut file = create_file_with_parent_dirs(&format!(
                "{parent}{nmsp}/data/{nmsp}/loot_tables/{path}.json"
            ))?;
            write!(file, "{contents}")?;
        }
        let mut mcmeta = create_file_with_parent_dirs(&format!("{parent}{nmsp}/pack.mcmeta"))?;
        write!(mcmeta, "{}", self.mcmeta)?;
        Ok(())
    }
}

fn create_file_with_parent_dirs(filename: &str) -> Result<File, std::io::Error> {
    let parent_dir = std::path::Path::new(filename).parent().unwrap();
    fs::create_dir_all(parent_dir)?;

    File::create(filename)
}
