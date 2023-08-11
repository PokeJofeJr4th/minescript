use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    hash::Hash,
    rc::Rc,
};
use strum_macros::{EnumIs, EnumString};

use crate::Config;

use super::prelude::*;

#[derive(Clone, PartialEq, EnumIs)]
pub enum Syntax {
    /// A floating piece of text
    Identifier(RStr),
    /// An annotation invocation with the name and body of the annotation
    Annotation(RStr, Box<Syntax>),
    /// A list of key-value pairs
    Object(BTreeMap<RStr, Syntax>),
    /// A list of syntax elements
    Array(Rc<[Syntax]>),
    /// A selector
    Selector(Selector<Syntax>),
    /// A selector with a colon and a score name
    SelectorColon(Selector<Syntax>, RStr),
    /// A selector with a double colon and a special identifier
    SelectorDoubleColon(Selector<Syntax>, RStr),
    /// A selector with an nbt path in the form of `@s.Inventory[42].tag`
    SelectorNbt(Selector<Syntax>, NbtPath),
    /// An identifier with an NBT path on the end
    NbtStorage(NbtPath),
    /// A binary operation like x += 2
    BinaryOp {
        lhs: OpLeft,
        operation: Operation,
        rhs: Box<Syntax>,
    },
    /// A block of the form `positioned @s { ... }`
    Block(BlockType, Box<Syntax>, Box<Syntax>),
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

impl Debug for Syntax {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Identifier(ident) => write!(f, "{ident}"),
            Self::Annotation(name, body) => write!(f, "@{name} {body:?}"),
            Self::Object(obj) => f.debug_map().entries(obj).finish(),
            Self::Array(arr) => f.debug_list().entries(arr.iter()).finish(),
            Self::Selector(sel) => write!(f, "{sel:?}"),
            Self::SelectorColon(sel, ident) => write!(f, "{sel:?}:{ident}"),
            Self::SelectorDoubleColon(sel, ident) => write!(f, "{sel:?}::{ident}"),
            Self::SelectorNbt(sel, nbt) => write!(f, "{sel:?}.{}", fmt_nbt_path(nbt)),
            Self::NbtStorage(nbt) => write!(f, "{}", fmt_nbt_path(nbt)),
            Self::BinaryOp {
                lhs,
                operation: op,
                rhs,
            } => write!(f, "{lhs:?} {op} {rhs:?}"),
            Self::Block(block_type, lhs, rhs) => write!(f, "{block_type} ({lhs:?}) {rhs:?}"),
            Self::String(str) => write!(f, "\"{str}\""),
            Self::Integer(int) => write!(f, "{int}"),
            Self::Range(Some(lhs), Some(rhs)) => write!(f, "{lhs}..{rhs}"),
            Self::Range(Some(lhs), None) => write!(f, "{lhs}.."),
            Self::Range(None, Some(rhs)) => write!(f, "..{rhs}"),
            Self::Range(None, None) => write!(f, ".."),
            Self::WooglyCoord(coord) => write!(f, "~{coord}"),
            Self::CaretCoord(coord) => write!(f, "^{coord}"),
            Self::Float(float) => write!(f, "{float}"),
            Self::Unit => write!(f, "()"),
        }
    }
}

impl From<OpLeft> for Syntax {
    fn from(value: OpLeft) -> Self {
        match value {
            OpLeft::Ident(id) => Self::Identifier(id),
            OpLeft::Colon(lhs, rhs) => Self::BinaryOp {
                lhs: OpLeft::Ident(lhs),
                operation: Operation::Equal,
                rhs: Box::new(Self::Identifier(rhs)),
            },
            OpLeft::Selector(sel) => Self::Selector(sel),
            OpLeft::SelectorColon(sel, id) => Self::SelectorColon(sel, id),
            OpLeft::SelectorDoubleColon(sel, id) => Self::SelectorDoubleColon(sel, id),
            OpLeft::SelectorNbt(sel, nbt) => Self::SelectorNbt(sel, nbt),
            OpLeft::NbtStorage(storage) => Self::NbtStorage(storage),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum BlockType {
    Advancement,
    Anchored,
    As,
    AsAt,
    Async,
    At,
    Case,
    Damage,
    Do,
    DoUntil,
    DoWhile,
    Facing,
    For,
    Function,
    If,
    On,
    Positioned,
    Rotated,
    Summon,
    Switch,
    Tellraw,
    // allow both versions to work
    #[strum(serialize = "tp", serialize = "teleport")]
    Tp,
    Unless,
    Until,
    While,
}

impl Display for BlockType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}

// this is fine because hash is deterministic and follows the relevant equality except for NaNs and I don't care about them
#[allow(clippy::derived_hash_with_manual_eq)]
impl Hash for Syntax {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // using the discriminant means that multiple enum variants can have the same hash body and still hash differently
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Identifier(str) | Self::String(str) => str.hash(state),
            Self::Annotation(name, syn) => {
                name.hash(state);
                syn.hash(state);
            }
            Self::Object(map) => map.hash(state),
            Self::Array(arr) => arr.hash(state),
            Self::Selector(sel) => sel.hash(state),
            Self::SelectorColon(sel, ident) | Self::SelectorDoubleColon(sel, ident) => {
                sel.hash(state);
                ident.hash(state);
            }
            Self::SelectorNbt(sel, nbt) => {
                sel.hash(state);
                nbt.hash(state);
            }
            Self::NbtStorage(nbt) => {
                nbt.hash(state);
            }
            Self::BinaryOp {
                lhs: left,
                operation: op,
                rhs: right,
            } => {
                left.hash(state);
                op.hash(state);
                right.hash(state);
            }
            Self::Block(block_block_type, lhs, rhs) => {
                block_block_type.hash(state);
                lhs.hash(state);
                rhs.hash(state);
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
    /// A specified entity with an NBT path
    SelectorNbt(Selector<Syntax>, NbtPath),
    /// A storage space with an NBT path
    NbtStorage(NbtPath),
}

impl OpLeft {
    pub fn stringify_scoreboard_target(&self) -> SResult<RStr> {
        match self {
            Self::Ident(id) | Self::Colon(id, _) => Ok(format!("%{id}").into()),
            Self::Selector(selector) | Self::SelectorColon(selector, _) => {
                Ok(format!("{}", selector.stringify()?).into())
            }
            Self::SelectorDoubleColon(_, _) | Self::SelectorNbt(_, _) | Self::NbtStorage(_) => {
                Err(format!(
                "{self:?} isn't a score. This is a compiler error. Please notify the developers"
            ))
            }
        }
    }

    pub fn stringify_scoreboard_objective(&self, config: &Config) -> SResult<RStr> {
        match self {
            Self::Ident(_) | Self::Selector(_) => Ok(config.dummy_objective.clone()),
            Self::Colon(_, score) | Self::SelectorColon(_, score) => Ok(score.clone()),
            Self::SelectorDoubleColon(_, _) | Self::SelectorNbt(_, _) | Self::NbtStorage(_) => {
                Err(format!(
                "{self:?} isn't a score. This is a compiler error. Please notify the developers"
            ))
            }
        }
    }
}

impl TryFrom<Syntax> for OpLeft {
    type Error = ();
    fn try_from(value: Syntax) -> Result<Self, Self::Error> {
        match value {
            Syntax::Identifier(ident) => Ok(Self::Ident(ident)),
            Syntax::BinaryOp {
                lhs: Self::Ident(lhs),
                operation: Operation::Equal,
                rhs,
            } => {
                let Syntax::Identifier(rhs) = *rhs else {
                    return Err(())
                };
                Ok(Self::Colon(lhs, rhs))
            }
            Syntax::Selector(sel) => Ok(Self::Selector(sel)),
            Syntax::SelectorColon(sel, ident) => Ok(Self::SelectorColon(sel, ident)),
            Syntax::SelectorDoubleColon(sel, ident) => Ok(Self::SelectorDoubleColon(sel, ident)),
            Syntax::SelectorNbt(sel, nbt) => Ok(Self::SelectorNbt(sel, nbt)),
            Syntax::NbtStorage(storage) => Ok(Self::NbtStorage(storage)),
            _ => Err(()),
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
    /// execute store result
    ColonEq,
    /// execute store success
    QuestionEq,
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
                Self::ColonEq => ":=",
                Self::QuestionEq => "?=",
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
            _ => Err(format!("Can't get a string from {value:?}")),
        }
    }
}

impl Syntax {
    pub fn to_selector_body(&self) -> SResult<String> {
        match self {
            Self::Identifier(str) | Self::String(str) => Ok(String::from(&**str)),
            Self::Integer(num) => Ok(format!("{num}")),
            Self::Float(float) => Ok(format!("{float}")),
            Self::Range(None, Some(rhs)) => Ok(format!("..{rhs}")),
            Self::Range(Some(lhs), None) => Ok(format!("{lhs}..")),
            Self::Range(Some(lhs), Some(rhs)) => Ok(format!("{lhs}..{rhs}")),
            Self::Object(obj) => {
                let mut buf = String::from('{');
                for (k, v) in obj {
                    buf.push_str(k);
                    buf.push('=');
                    buf.push_str(&v.to_selector_body()?);
                }
                if !obj.is_empty() {
                    buf.pop();
                }
                Ok(buf)
            }
            _ => Err(format!("Can't get a string from {self:?}")),
        }
    }
}

impl TryFrom<&Syntax> for RStr {
    type Error = String;

    fn try_from(value: &Syntax) -> SResult<Self> {
        match value {
            Syntax::Identifier(str) | Syntax::String(str) => Ok(str.clone()),
            Syntax::Integer(num) => Ok(format!("{num}").into()),
            _ => Err(format!("Can't get a string from {value:?}")),
        }
    }
}
