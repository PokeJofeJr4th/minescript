use std::{collections::BTreeMap, fmt::Display, hash::Hash, rc::Rc};

use super::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Syntax {
    /// A floating piece of text
    Identifier(RStr),
    /// A macro invocation with the name and body of the macro
    Macro(RStr, Box<Syntax>),
    /// A list of key-value pairs
    Object(BTreeMap<RStr, Syntax>),
    /// A list of syntax elements
    Array(Rc<[Syntax]>),
    /// A function definition with the function name and body
    Function(RStr, Box<Syntax>),
    /// A selector
    Selector(Selector<Syntax>),
    /// A selector with a colon and a score name
    ColonSelector(Selector<Syntax>, RStr),
    /// A binary operation like x += 2
    BinaryOp(OpLeft, Operation, Box<Syntax>),
    /// A block of the form `if left op right {content}`
    Block(BlockType, OpLeft, Operation, Box<Syntax>, Box<Syntax>),
    /// A block of the form `as @s {content}`
    SelectorBlock(SelectorBlockType, Selector<Syntax>, Box<Syntax>),
    /// A block of the form `summon sheep {...}`
    IdentBlock(IdentBlockType, RStr, Box<Syntax>),
    /// A string literal
    String(RStr),
    /// An integer literal
    Integer(i32),
    /// A range literal
    Range(Option<i32>, Option<i32>),
    /// A coordinate starting with ~
    WooglyCoord(f32),
    /// A coordinate starting with ^
    CaretCoord(f32),
    /// A float literal
    Float(f32),
    /// An empty object
    Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockType {
    If,
    Unless,
    For,
    DoWhile,
    While,
    DoUntil,
    Until,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectorBlockType {
    As,
    At,
    AsAt,
    Tp,
    Damage,
    TellRaw,
    IfEntity,
    FacingEntity,
    Rotated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdentBlockType {
    On,
    Summon,
    Anchored,
}

// this is fine because hash is deterministic and follows the relevant equality except for NaNs and I don't care about them
#[allow(clippy::derived_hash_with_manual_eq)]
impl Hash for Syntax {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // using the discriminant means that multiple enum variants can have the same hash body and still hash differently
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Identifier(str) | Self::String(str) => str.hash(state),
            Self::Function(name, syn) | Self::Macro(name, syn) => {
                name.hash(state);
                syn.hash(state);
            }
            Self::Object(map) => map.hash(state),
            Self::Array(arr) => arr.hash(state),
            Self::Selector(sel) => sel.hash(state),
            Self::ColonSelector(sel, ident) => {
                sel.hash(state);
                ident.hash(state);
            }
            Self::BinaryOp(left, op, right) => {
                left.hash(state);
                op.hash(state);
                right.hash(state);
            }
            Self::Block(blocktype, left, op, right, content) => {
                blocktype.hash(state);
                left.hash(state);
                op.hash(state);
                right.hash(state);
                content.hash(state);
            }
            Self::SelectorBlock(block_selector_type, selector, content) => {
                block_selector_type.hash(state);
                selector.hash(state);
                content.hash(state);
            }
            Self::IdentBlock(block_type, ident, content) => {
                block_type.hash(state);
                ident.hash(state);
                content.hash(state);
            }
            Self::Integer(int) => int.hash(state),
            Self::Range(left, right) => {
                left.hash(state);
                right.hash(state);
            }
            // allow float to hash. NaNs are non-deterministic
            Self::Float(float) | Self::WooglyCoord(float) | Self::CaretCoord(float) => {
                unsafe { &*(float as *const f32).cast::<u32>() }.hash(state);
            }
            Self::Unit => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum OpLeft {
    /// An imaginary player's dummy objective
    Ident(RStr),
    /// An imaginary player's specified objective
    Colon(RStr, RStr),
    /// A specified entity's dummy objective
    Selector(Selector<Syntax>),
    /// A specified entity's specified objective
    SelectorColon(Selector<Syntax>, RStr),
    /// A specified entity's special property
    SelectorDoubleColon(Selector<Syntax>, RStr),
}

impl OpLeft {
    pub fn stringify_scoreboard_target(&self) -> SResult<RStr> {
        match self {
            Self::Ident(id) | Self::Colon(id, _) => Ok(format!("%{id}").into()),
            Self::Selector(selector) | Self::SelectorColon(selector, _) => {
                Ok(format!("{}", selector.stringify()?).into())
            }
            Self::SelectorDoubleColon(_, _) => Err(format!(
                "{self:?} isn't a score. This is a compiler error. Please notify the developers"
            )),
        }
    }

    pub fn stringify_scoreboard_objective(&self) -> SResult<RStr> {
        match self {
            Self::Ident(_) | Self::Selector(_) => Ok("dummy".into()),
            Self::Colon(_, score) | Self::SelectorColon(_, score) => Ok(score.clone()),
            Self::SelectorDoubleColon(_, _) => Err(format!(
                "{self:?} isn't a score. This is a compiler error. Please notify the developers"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    /// key-value pair
    Colon,
    /// xp levels and points
    DoubleColon,
    /// unused
    Dot,
    /// check equality or assign value
    Equal,
    /// less than
    LCaret,
    /// less than or equal
    LCaretEq,
    /// greater than
    RCaret,
    /// greater than or equal
    RCaretEq,
    /// not equal
    BangEq,
    /// add and assign
    AddEq,
    /// subtract and assign
    SubEq,
    /// multiply and assign
    MulEq,
    /// divide and assign
    DivEq,
    /// modulo and assign
    ModEq,
    /// swap values
    Swap,
    /// check if in range
    In,
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Colon => ":",
                Self::DoubleColon => "::",
                Self::Dot => ".",
                Self::Equal => "=",
                Self::LCaretEq => "<=",
                Self::RCaretEq => ">=",
                Self::BangEq => "!=",
                Self::AddEq => "+=",
                Self::SubEq => "-=",
                Self::MulEq => "*=",
                Self::DivEq => "/=",
                Self::ModEq => "%=",
                Self::Swap => "><",
                Self::LCaret => "<",
                Self::RCaret => ">",
                Self::In => "in",
            }
        )
    }
}

impl TryFrom<&Syntax> for String {
    type Error = Self;

    fn try_from(value: &Syntax) -> SResult<Self> {
        match value {
            Syntax::Identifier(str) | Syntax::String(str) => Ok(Self::from(&**str)),
            Syntax::Integer(num) => Ok(format!("{num}")),
            Syntax::Float(float) => Ok(format!("{float}")),
            _ => Err(format!("Can't get a string from {value:?}")),
        }
    }
}

impl TryFrom<&Syntax> for RStr {
    type Error = String;

    fn try_from(value: &Syntax) -> SResult<Self> {
        match value {
            Syntax::Identifier(str) | Syntax::String(str) => Ok(str.clone()),
            Syntax::Integer(num) => Ok(format!("{num}").into()),
            Syntax::Float(float) => Ok(format!("{float}").into()),
            _ => Err(format!("Can't get a string from {value:?}")),
        }
    }
}
