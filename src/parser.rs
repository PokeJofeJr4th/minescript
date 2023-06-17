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
    String(RStr),
    Integer(i32),
    Float(f32),
    Unit,
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
            Self::Integer(int) => int.hash(state),
            Self::Float(float) => unsafe { &*(float as *const f32).cast::<u32>() }.hash(state),
            Self::Unit => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum OpLeft {
    Ident(RStr),
    Dotted(RStr, RStr),
    Selector(Selector<Syntax>),
    DottedSelector(Selector<Syntax>, RStr),
}

impl OpLeft {
    pub fn stringify_scoreboard_target(&self) -> Result<RStr, String> {
        match self {
            Self::Ident(id) | Self::Dotted(id, _) => Ok(format!("%{id}").into()),
            Self::Selector(selector) | Self::DottedSelector(selector, _) => {
                Ok(format!("{}", selector.stringify()?).into())
            }
        }
    }

    pub fn stringify_scoreboard_objective(&self) -> RStr {
        match self {
            Self::Ident(_) | Self::Selector(_) => "dummy".into(),
            Self::Dotted(_, score) | Self::DottedSelector(_, score) => score.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    Colon,
    Dot,
    Equal,
    AddEq,
    SubEq,
    MulEq,
    DivEq,
    ModEq,
    Swap,
    Min,
    Max,
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
                Self::AddEq => "+=",
                Self::SubEq => "-=",
                Self::MulEq => "*=",
                Self::DivEq => "/=",
                Self::ModEq => "%=",
                Self::Swap => "><",
                Self::Min => "<",
                Self::Max => ">",
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
        Some(Token::Number(num)) => match tokens.peek() {
            Some(Token::Dot) => {
                tokens.next();
                let Some(Token::Number(decimal)) = tokens.next() else {
                    return Err(String::from("Expected a decimal part after a point"))
                };
                Ok(Syntax::Float(
                    num as f32 + (decimal as f32 / 10.0f32.powi(decimal.ilog10() as i32 + 1)),
                ))
            }
            _ => Ok(Syntax::Integer(num)),
        },
        Some(Token::Tack) => {
            let Some(Token::Number(num)) = tokens.next() else {
                return Err(String::from("Expected a number after `-`"))
            };
            match tokens.peek() {
                Some(Token::Dot) => {
                    tokens.next();
                    let Some(Token::Number(decimal)) = tokens.next() else {
                        return Err(String::from("Expected a decimal part after a point"))
                    };
                    Ok(Syntax::Float(
                        -num as f32 + (decimal as f32 / 10f32.powi(decimal.ilog10() as i32 + 1)),
                    ))
                }
                _ => Ok(Syntax::Integer(-num)),
            }
        }
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
                }
                statements_buf.push(parse(tokens)?);
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
                    || Err(String::from("Object syntax must only contain props")),
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
                }
                statements_buf.push(parse(tokens)?);
            }
            Ok(Syntax::Array(statements_buf.into()))
        }
        Some(Token::At) => parse_macro(tokens),
        other => Err(format!("Unexpected token `{other:?}`")),
    };
    match &first {
        Ok(Syntax::BinaryOp(OpLeft::Ident(first), Operation::Dot, second)) => {
            if let Syntax::Identifier(second) = &**second {
                if let Some(op) = get_op(tokens) {
                    return Ok(Syntax::BinaryOp(
                        OpLeft::Dotted(first.clone(), second.clone()),
                        op,
                        Box::new(parse(tokens)?),
                    ));
                }
            }
        }
        Ok(Syntax::Selector(sel)) => {
            if tokens.peek() == Some(&Token::Dot) {
                tokens.next();
                let Some(Token::Identifier(ident)) = tokens.next() else {
                    return Err(String::from("Selectors can only be indexed with `.<identifier>`"))
                };
                if let Some(op) = get_op(tokens) {
                    return Ok(Syntax::BinaryOp(
                        OpLeft::DottedSelector(sel.clone(), ident),
                        op,
                        Box::new(parse(tokens)?),
                    ));
                }
            }
        }
        _ => {}
    }
    first
}

fn get_op<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> Option<Operation> {
    let val = match tokens.peek() {
        Some(Token::Colon) => Some(Operation::Colon),
        Some(Token::Equal) => Some(Operation::Equal),
        Some(Token::PlusEq) => Some(Operation::AddEq),
        Some(Token::TackEq) => Some(Operation::SubEq),
        Some(Token::StarEq) => Some(Operation::MulEq),
        Some(Token::SlashEq) => Some(Operation::DivEq),
        Some(Token::PercEq) => Some(Operation::ModEq),
        Some(Token::LCaret) => Some(Operation::Min),
        Some(Token::RCaret) => {
            tokens.next();
            match tokens.peek() {
                Some(Token::LCaret) => {
                    tokens.next();
                    Some(Operation::Swap)
                }
                _ => Some(Operation::Max),
            }
        }
        _ => None,
    };
    if val.is_some() {
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
    } else if &*id == "function" {
        let Some(Token::Identifier(func)) = tokens.next() else {
                return Err(String::from("Expected identifier after function"))
            };
        Ok(Syntax::Function(func, Box::new(parse(tokens)?)))
    // } else if id == "effect" {
    //     todo!()
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
        parser::{parse, OpLeft, Operation, Syntax},
    };

    #[test]
    fn parse_literals() {
        assert_eq!(
            parse(
                &mut [Token::Number(0), Token::Dot, Token::Number(2)]
                    .into_iter()
                    .peekable()
            ),
            Ok(Syntax::Float(0.2))
        );
        assert_eq!(
            parse(&mut [Token::Tack, Token::Number(20)].into_iter().peekable()),
            Ok(Syntax::Integer(-20))
        );
    }

    #[test]
    fn parse_score_op() {
        assert_eq!(
            parse(
                &mut [
                    Token::At,
                    Token::Identifier("a".into()),
                    Token::Dot,
                    Token::Identifier("x".into()),
                    Token::PlusEq,
                    Token::Number(2)
                ]
                .into_iter()
                .peekable()
            ),
            Ok(Syntax::BinaryOp(
                OpLeft::DottedSelector(
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
}
