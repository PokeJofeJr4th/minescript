mod command;
mod nbt;
mod selector;
mod syntax;
mod token;

pub use prelude::*;

pub mod prelude {
    use std::rc::Rc;
    pub type RStr = Rc<str>;
    pub use super::command::{Command, ExecuteOption};
    pub use super::nbt::Nbt;
    pub use super::selector::{Selector, SelectorType};
    pub use super::syntax::{BlockType, OpLeft, Operation, Syntax};
    pub use super::token::Token;
}
