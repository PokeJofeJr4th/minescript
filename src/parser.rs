use std::{collections::BTreeMap, fmt::Display, hash::Hash, iter::Peekable, rc::Rc};

use crate::{
    command::{Selector, SelectorType},
    lexer::Token,
    RStr,
};

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
    While,
    For,
}

// this is fine because hash is deterministic except for NaNs and I don't care about them
#[allow(clippy::derive_hash_xor_eq)]
impl Hash for Syntax {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
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
            Self::BinaryOp(left, op, syn) => {
                left.hash(state);
                op.hash(state);
                syn.hash(state);
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

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]
pub fn parse<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> Result<Syntax, String> {
    let first = match tokens.next() {
        Some(Token::String(str)) => Ok(Syntax::String(str)),
        Some(Token::Integer(num)) => Ok(Syntax::Integer(num)),
        Some(Token::Range(l, r)) => Ok(Syntax::Range(l, r)),
        Some(Token::Doot) => {
            let Some(Token::Integer(num)) = tokens.next() else {
                return Err(String::from("Expected number after `..`"))
            };
            Ok(Syntax::Range(None, Some(num)))
        }
        Some(Token::Tack) => match tokens.next() {
            Some(Token::Integer(num)) => Ok(Syntax::Integer(-num)),
            Some(Token::Float(num)) => Ok(Syntax::Float(num)),
            _ => Err(String::from("Expected a number or float after `-`")),
        },
        Some(Token::Identifier(id)) => parse_identifier(tokens, id),
        Some(Token::LSquirrely) => {
            let mut statements_buf = Vec::new();
            match tokens.peek() {
                Some(Token::RSquirrely) => {
                    tokens.next();
                    return Ok(Syntax::Unit);
                }
                _ => statements_buf.push(parse(tokens)?),
            }
            while let Some(tok) = tokens.peek() {
                if tok == &Token::RSquirrely {
                    tokens.next();
                    break;
                } else if tok == &Token::Comma || tok == &Token::SemiColon {
                    tokens.next();
                } else {
                    // println!("Squirrely Object");
                    statements_buf.push(parse(tokens)?);
                }
            }
            statements_buf
                .iter()
                .map(|syn| match syn {
                    Syntax::BinaryOp(OpLeft::Ident(k), Operation::Colon, v) => {
                        Some((k.clone(), *(*v).clone()))
                    }
                    _ => None,
                })
                .collect::<Option<BTreeMap<_, _>>>()
                .map_or_else(
                    || Ok(Syntax::Array(statements_buf.into())),
                    |props| Ok(Syntax::Object(props)),
                )
        }
        Some(Token::LSquare) => {
            let mut statements_buf = Vec::new();
            while let Some(tok) = tokens.peek() {
                if tok == &Token::RSquare {
                    tokens.next();
                    break;
                } else if tok == &Token::Comma || tok == &Token::SemiColon {
                    tokens.next();
                } else {
                    // println!("Square Object");
                    statements_buf.push(parse(tokens)?);
                }
            }
            Ok(Syntax::Array(statements_buf.into()))
        }
        Some(Token::At) => parse_macro(tokens),
        other => Err(format!(
            "Unexpected token `{other:?}`; {:?}",
            tokens.collect::<Vec<_>>()
        )),
    };
    match &first {
        Ok(Syntax::BinaryOp(OpLeft::Ident(first), Operation::Colon, second)) => {
            // println!("Binary Operation");
            if let Syntax::Identifier(second) = &**second {
                if let Some(op) = get_op(tokens) {
                    // println!("Secondary Operation");
                    return Ok(Syntax::BinaryOp(
                        OpLeft::Colon(first.clone(), second.clone()),
                        op,
                        Box::new(parse(tokens)?),
                    ));
                } else if tokens.peek() == Some(&Token::PlusPlus) {
                    tokens.next();
                    return Ok(Syntax::BinaryOp(
                        OpLeft::Colon(first.clone(), second.clone()),
                        Operation::AddEq,
                        Box::new(Syntax::Integer(1)),
                    ));
                } else if tokens.peek() == Some(&Token::TackTack) {
                    tokens.next();
                    return Ok(Syntax::BinaryOp(
                        OpLeft::Colon(first.clone(), second.clone()),
                        Operation::SubEq,
                        Box::new(Syntax::Integer(1)),
                    ));
                }
            }
        }
        Ok(Syntax::Selector(sel)) => {
            // println!("Selector");
            if tokens.peek() == Some(&Token::Colon) {
                tokens.next();
                let Some(Token::Identifier(ident)) = tokens.next() else {
                    return Err(String::from("Selectors can only be indexed with `:<identifier>`"))
                };
                if let Some(op) = get_op(tokens) {
                    return Ok(Syntax::BinaryOp(
                        OpLeft::SelectorColon(sel.clone(), ident),
                        op,
                        Box::new(parse(tokens)?),
                    ));
                } else if tokens.peek() == Some(&Token::PlusPlus) {
                    tokens.next();
                    return Ok(Syntax::BinaryOp(
                        OpLeft::SelectorColon(sel.clone(), ident),
                        Operation::AddEq,
                        Box::new(Syntax::Integer(1)),
                    ));
                } else if tokens.peek() == Some(&Token::TackTack) {
                    tokens.next();
                    return Ok(Syntax::BinaryOp(
                        OpLeft::SelectorColon(sel.clone(), ident),
                        Operation::SubEq,
                        Box::new(Syntax::Integer(1)),
                    ));
                }
            }
        }
        _ => {}
    }
    // println!("{first:?}");
    first
}

fn get_op<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> Option<Operation> {
    let val = match tokens.peek() {
        Some(Token::Colon) => Some(Operation::Colon),
        Some(Token::Equal) => Some(Operation::Equal),
        Some(Token::LCaretEq) => Some(Operation::LCaretEq),
        Some(Token::RCaretEq) => Some(Operation::RCaretEq),
        Some(Token::BangEq) => Some(Operation::BangEq),
        Some(Token::PlusEq) => Some(Operation::AddEq),
        Some(Token::TackEq) => Some(Operation::SubEq),
        Some(Token::StarEq) => Some(Operation::MulEq),
        Some(Token::SlashEq) => Some(Operation::DivEq),
        Some(Token::PercEq) => Some(Operation::ModEq),
        Some(Token::LCaret) => Some(Operation::LCaret),
        Some(Token::Identifier(ident)) => {
            if ident.as_ref() == "in" {
                Some(Operation::In)
            } else {
                None
            }
        }
        Some(Token::RCaret) => {
            tokens.next();
            match tokens.peek() {
                Some(Token::LCaret) => {
                    tokens.next();
                    Some(Operation::Swap)
                }
                _ => Some(Operation::RCaret),
            }
        }
        _ => None,
    };
    // the `>` already consumes the next token
    if val.is_some() && val != Some(Operation::RCaret) {
        tokens.next();
    }
    val
}

fn parse_identifier<T: Iterator<Item = Token>>(
    tokens: &mut Peekable<T>,
    id: RStr,
) -> Result<Syntax, String> {
    if let Some(op) = get_op(tokens) {
        Ok(Syntax::BinaryOp(
            OpLeft::Ident(id),
            op,
            Box::new(parse(tokens)?),
        ))
    } else if tokens.peek() == Some(&Token::PlusPlus) {
        tokens.next();
        Ok(Syntax::BinaryOp(
            OpLeft::Ident(id),
            Operation::AddEq,
            Box::new(Syntax::Integer(1)),
        ))
    } else if tokens.peek() == Some(&Token::TackTack) {
        tokens.next();
        Ok(Syntax::BinaryOp(
            OpLeft::Ident(id),
            Operation::SubEq,
            Box::new(Syntax::Integer(1)),
        ))
    } else if &*id == "function" {
        let Some(Token::Identifier(func) | Token::String(func)) = tokens.next() else {
                return Err(String::from("Expected identifier after function"))
            };
        Ok(Syntax::Function(func, Box::new(parse(tokens)?)))
    // } else if id == "effect" {
    //     todo!()
    } else if &*id == "if" {
        // println!("If Statement");
        let Syntax::BinaryOp(left, op, right) = parse(tokens)? else {
            return Err(String::from("If statement requires a check like `x = 2`"))
        };
        // println!("If Block");
        Ok(Syntax::Block(
            BlockType::If,
            left,
            op,
            right,
            Box::new(parse(tokens)?),
        ))
    } else if &*id == "while" {
        let Syntax::BinaryOp(left, op, right) = parse(tokens)? else {
            return Err(String::from("While loop requires a check like `x = 2`"))
        };
        Ok(Syntax::Block(
            BlockType::While,
            left,
            op,
            right,
            Box::new(parse(tokens)?),
        ))
    } else if &*id == "for" {
        let Syntax::BinaryOp(left, op, right) = parse(tokens)? else {
            return Err(String::from("For loop requires a check like `x = 2`"))
        };
        Ok(Syntax::Block(
            BlockType::For,
            left,
            op,
            right,
            Box::new(parse(tokens)?),
        ))
    } else {
        Ok(Syntax::Identifier(id))
    }
}

fn parse_macro<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> Result<Syntax, String> {
    {
        let Some(Token::Identifier(identifier)) = tokens.next() else {
            return Err("Expected identifier after `@`".to_string())
        };
        match identifier.as_ref() {
            "s" | "p" | "e" | "a" | "r" => {
                let mut statements_buf = BTreeMap::new();
                if tokens.peek() == Some(&Token::LSquare) {
                    tokens.next();
                    while let Some(tok) = tokens.next() {
                        if tok == Token::RSquare {
                            tokens.next();
                            break;
                        }
                        let Token::Identifier(ident) = tok else {
                            return Err(format!("Expected a selector parameter; got `{tok:?}`"))
                        };
                        let Some(Token::Equal) = tokens.next() else {
                            return Err(String::from("Expected `=` for selector property"))
                        };
                        statements_buf.insert(ident.clone(), parse(tokens)?);
                    }
                }
                Ok(Syntax::Selector(Selector {
                    selector_type: match identifier.as_ref() {
                        "s" => SelectorType::S,
                        "p" => SelectorType::P,
                        "e" => SelectorType::E,
                        "a" => SelectorType::A,
                        "r" => SelectorType::R,
                        _ => unreachable!(),
                    },
                    args: statements_buf,
                }))
            }
            _ => Ok(Syntax::Macro(identifier, Box::new(parse(tokens)?))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        command::{Selector, SelectorType},
        lexer::Token,
        parser::{parse, BlockType, OpLeft, Operation, Syntax},
    };

    #[test]
    fn parse_literals() {
        // -20
        assert_eq!(
            parse(&mut [Token::Tack, Token::Integer(20)].into_iter().peekable()),
            Ok(Syntax::Integer(-20))
        );
    }

    #[test]
    fn parse_score_op() {
        // @a:x += 2
        assert_eq!(
            parse(
                &mut [
                    Token::At,
                    Token::Identifier("a".into()),
                    Token::Colon,
                    Token::Identifier("x".into()),
                    Token::PlusEq,
                    Token::Integer(2)
                ]
                .into_iter()
                .peekable()
            ),
            Ok(Syntax::BinaryOp(
                OpLeft::SelectorColon(
                    Selector {
                        selector_type: SelectorType::A,
                        args: BTreeMap::new()
                    },
                    "x".into()
                ),
                Operation::AddEq,
                Box::new(Syntax::Integer(2))
            ))
        );
    }

    #[test]
    fn parse_in_range() {
        // x in 0..10
        assert_eq!(
            parse(
                &mut [
                    Token::Identifier("x".into()),
                    Token::Identifier("in".into()),
                    Token::Range(Some(0), Some(10))
                ]
                .into_iter()
                .peekable()
            ),
            Ok(Syntax::BinaryOp(
                OpLeft::Ident("x".into()),
                Operation::In,
                Box::new(Syntax::Range(Some(0), Some(10)))
            ))
        );
    }

    #[test]
    fn parse_for_loop() {
        // for x in 0..10 {}
        assert_eq!(
            parse(
                &mut [
                    Token::Identifier("for".into()),
                    Token::Identifier("x".into()),
                    Token::Identifier("in".into()),
                    Token::Range(Some(0), Some(10)),
                    Token::LSquirrely,
                    Token::RSquirrely
                ]
                .into_iter()
                .peekable()
            ),
            Ok(Syntax::Block(
                BlockType::For,
                OpLeft::Ident("x".into()),
                Operation::In,
                Box::new(Syntax::Range(Some(0), Some(10))),
                Box::new(Syntax::Unit)
            ))
        );
    }
}
