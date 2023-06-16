use std::{collections::BTreeMap, iter::Peekable};

use crate::{command::SelectorType, lexer::Token};

#[derive(Debug, Clone)]
pub enum Syntax {
    Identifier(String),
    Macro(String, Box<Syntax>),
    Object(BTreeMap<String, Syntax>),
    Array(Vec<Syntax>),
    Function(String, Box<Syntax>),
    Selector(SelectorType, Vec<(String, Syntax)>),
    BinaryOp(String, Operation, Box<Syntax>),
    String(String),
    Integer(i32),
    Float(f32),
    Unit,
}

#[derive(Debug, Clone)]
pub enum Operation {
    Colon,
    Equal,
    AddEq,
    SubEq,
    MulEq,
    DivEq,
}

impl TryFrom<&Syntax> for String {
    type Error = ();

    fn try_from(value: &Syntax) -> Result<Self, Self::Error> {
        match value {
            Syntax::Identifier(str) | Syntax::String(str) => Ok(str.clone()),
            Syntax::Integer(num) => Ok(format!("{num}")),
            Syntax::Float(float) => Ok(format!("{float}")),
            _ => Err(()),
        }
    }
}

pub fn parse<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> Result<Syntax, String> {
    match tokens.next() {
        Some(Token::String(str)) => Ok(Syntax::String(str)),
        Some(Token::Number(num)) => match tokens.peek() {
            Some(Token::Dot) => {
                tokens.next();
                let Some(Token::Number(decimal)) = tokens.next() else {
                    return Err(String::from("Expected a decimal part after a number"))
                };
                Ok(Syntax::Float(
                    num as f32 + (decimal as f32 / 10.0f32.powi(decimal.ilog10() as i32)),
                ))
            }
            _ => Ok(Syntax::Integer(num)),
        },
        Some(Token::Identifier(id)) => {
            let operation = match tokens.peek() {
                Some(Token::Colon) => Some(Operation::Colon),
                Some(Token::Equal) => Some(Operation::Equal),
                Some(Token::PlusEq) => Some(Operation::AddEq),
                Some(Token::TackEq) => Some(Operation::SubEq),
                Some(Token::StarEq) => Some(Operation::MulEq),
                Some(Token::SlashEq) => Some(Operation::DivEq),
                _ => None,
            };
            if let Some(op) = operation {
                tokens.next();
                Ok(Syntax::BinaryOp(id, op, Box::new(parse(tokens)?)))
            } else if id == "function" {
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
            match statements_buf
                .iter()
                .map(|syn| match syn {
                    Syntax::BinaryOp(k, Operation::Colon, v) => Some((k.clone(), *(*v).clone())),
                    _ => None,
                })
                .collect::<Option<BTreeMap<_, _>>>()
            {
                Some(props) => Ok(Syntax::Object(props)),
                None => Err(String::from("Object syntax must only contain props")),
            }
        }
        Some(Token::LSquare) => {
            let mut statements_buf = Vec::new();
            while let Some(tok) = tokens.peek() {
                if tok == &Token::RSquare {
                    tokens.next();
                    break;
                } else if tok == &Token::Comma {
                    tokens.next();
                }
                statements_buf.push(parse(tokens)?)
            }
            Ok(Syntax::Array(statements_buf))
        }
        Some(Token::At) => {
            let Some(Token::Identifier(identifier)) = tokens.next() else {
                return Err("Expected identifier after `@`".to_string())
            };
            match identifier.as_ref() {
                "s" | "p" | "e" | "a" | "r" => {
                    let mut statements_buf = Vec::new();
                    if let Some(Token::LSquare) = tokens.peek() {
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
                            statements_buf.push((ident.clone(), parse(tokens)?))
                        }
                    }
                    Ok(Syntax::Selector(
                        match identifier.as_ref() {
                            "s" => SelectorType::S,
                            "p" => SelectorType::P,
                            "e" => SelectorType::E,
                            "a" => SelectorType::A,
                            "r" => SelectorType::R,
                            _ => unreachable!(),
                        },
                        statements_buf,
                    ))
                }
                _ => Ok(Syntax::Macro(identifier, Box::new(parse(tokens)?))),
            }
        }
        other => Err(format!("Unexpected token `{other:?}`")),
    }
}
