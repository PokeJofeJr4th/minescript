mod command;
mod nbt;
mod selector;
mod syntax;
mod token;

pub use prelude::*;

pub mod prelude {
    use core::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    use std::rc::Rc;

    pub use super::command::{Command, ExecuteOption};
    pub use super::nbt::Nbt;
    pub use super::selector::{Selector, SelectorType};
    pub use super::syntax::{BlockType, OpLeft, Operation, Syntax};
    pub use super::token::Token;
    pub use crate::nbt;

    pub type SResult<T> = Result<T, String>;
    pub type RStr = Rc<str>;

    pub fn get_hash<T: Hash>(obj: T) -> u64 {
        let mut hasher = DefaultHasher::new();
        obj.hash(&mut hasher);
        hasher.finish()
    }
}
