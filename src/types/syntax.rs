use std::{collections::BTreeMap, fmt::Display, hash::Hash, rc::Rc};

use super::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Syntax {
    Identifier(RStr),
    Macro(RStr, Box<Syntax>),
    Object(BTreeMap<RStr, Syntax>),
    Array(Rc<[Syntax]>),
    Function(RStr, Box<Syntax>),
    Selector(Selector<Syntax>),
    DottedSelector(Selector<Syntax>, RStr),
    BinaryOp(OpLeft, Operation, Box<Syntax>),
    Block(BlockType, OpLeft, Operation, Box<Syntax>, Box<Syntax>),
    String(RStr),
    Integer(i32),
    Range(Option<i32>, Option<i32>),
    Float(f32),
    Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockType {
    If,
    For,
    DoWhile,
    While,
}

// this is fine because hash is deterministic and follows the relevant equality except for NaNs and I don't care about them
#[allow(clippy::derive_hash_xor_eq)]
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
            Self::DottedSelector(sel, ident) => {
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
            Self::Integer(int) => int.hash(state),
            Self::Range(left, right) => {
                left.hash(state);
                right.hash(state);
            }
            // allow float to hash. NaNs are non-deterministic
            Self::Float(float) => unsafe { &*(float as *const f32).cast::<u32>() }.hash(state),
            Self::Unit => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum OpLeft {
    Ident(RStr),
    Colon(RStr, RStr),
    Selector(Selector<Syntax>),
    SelectorColon(Selector<Syntax>, RStr),
}

impl OpLeft {
    pub fn stringify_scoreboard_target(&self) -> Result<RStr, String> {
        match self {
            Self::Ident(id) | Self::Colon(id, _) => Ok(format!("%{id}").into()),
            Self::Selector(selector) | Self::SelectorColon(selector, _) => {
                Ok(format!("{}", selector.stringify()?).into())
            }
        }
    }

    pub fn stringify_scoreboard_objective(&self) -> RStr {
        match self {
            Self::Ident(_) | Self::Selector(_) => "dummy".into(),
            Self::Colon(_, score) | Self::SelectorColon(_, score) => score.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    Colon,
    Dot,
    Equal,
    LCaret,
    LCaretEq,
    RCaret,
    RCaretEq,
    BangEq,
    AddEq,
    SubEq,
    MulEq,
    DivEq,
    ModEq,
    Swap,
    In,
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Colon => ":",
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
    type Error = ();

    fn try_from(value: &Syntax) -> Result<Self, Self::Error> {
        match value {
            Syntax::Identifier(str) | Syntax::String(str) => Ok(Self::from(&**str)),
            Syntax::Integer(num) => Ok(format!("{num}")),
            Syntax::Float(float) => Ok(format!("{float}")),
            _ => Err(()),
        }
    }
}

impl TryFrom<&Syntax> for RStr {
    type Error = ();

    fn try_from(value: &Syntax) -> Result<Self, Self::Error> {
        match value {
            Syntax::Identifier(str) | Syntax::String(str) => Ok(str.clone()),
            Syntax::Integer(num) => Ok(format!("{num}").into()),
            Syntax::Float(float) => Ok(format!("{float}").into()),
            _ => Err(()),
        }
    }
}
