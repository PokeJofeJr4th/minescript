use std::{collections::BTreeMap, fs, path::PathBuf};

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
pub struct InterRepr {
    pub items: Vec<Item>,
    pub objectives: BTreeMap<RStr, RStr>,
    pub functions: Vec<(RStr, Vec<Command>)>,
    pub recipes: BTreeMap<RStr, String>,
    folder: PathBuf,
}

impl InterRepr {
    /// Create a new, empty Intermediate Representation
    ///
    /// Only exposed for testing, as it prevents imports from working
    fn empty() -> Self {
        Self {
            items: Vec::new(),
            objectives: BTreeMap::new(),
            functions: Vec::new(),
            recipes: BTreeMap::new(),
            folder: PathBuf::new(),
        }
    }

    #[cfg(test)]
    // This just exposes the `new` function for testing
    #[allow(non_upper_case_globals)]
    pub const new: fn() -> Self = Self::empty;

    /// Create an empty `InterRepr` that will import from the given path
    pub fn from_path<T>(path: T) -> Self
    where
        PathBuf: From<T>,
    {
        let mut new = Self::empty();
        new.folder = PathBuf::from(path);
        new
    }

    /// Import a file's contents. `filename` should be relative to the given path
    pub fn import(&self, filename: &str) -> SResult<String> {
        let path_buf = self.folder.join(filename);
        fs::read_to_string(path_buf).map_err(|err| format!("Error opening file: {err}"))
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompiledRepr {
    pub functions: BTreeMap<RStr, String>,
    pub advancements: BTreeMap<RStr, String>,
    pub recipes: BTreeMap<RStr, String>,
    pub mcmeta: String,
}
