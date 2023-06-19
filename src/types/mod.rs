mod command;
mod nbt;
mod selector;

pub mod prelude {
    use std::rc::Rc;
    pub type RStr = Rc<str>;
    pub use super::command::{Command, ExecuteOption};
    pub use super::selector::{Selector, SelectorType};
    pub use super::nbt::Nbt;
}
