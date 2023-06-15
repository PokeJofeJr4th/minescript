use std::{collections::BTreeMap, iter::Peekable};

use crate::{command::SelectorType, lexer::Token};

#[derive(Debug, Clone)]
pub enum Syntax {
    Block(Vec<Syntax>),
    Identifier(String),
    Macro(String, Box<Syntax>),
    Object(BTreeMap<String, Syntax>),
    Array(Vec<Syntax>),
    Function(String, Box<Syntax>),
    Selector(SelectorType, Vec<(String, Syntax)>),
    Property(String, Box<Syntax>),
    String(String),
    Number(i32),
    Unit,
}

impl TryFrom<&Syntax> for String {
    type Error = ();

    fn try_from(value: &Syntax) -> Result<Self, Self::Error> {
        match value {
            Syntax::Identifier(str) | Syntax::String(str) => Ok(str.clone()),
            Syntax::Number(num) => Ok(format!("{num}")),
            _ => Err(()),
        }
    }
}

pub fn parse<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> Result<Syntax, String> {
    match tokens.next() {
        Some(Token::String(str)) => Ok(Syntax::String(str)),
        Some(Token::Number(num)) => Ok(Syntax::Number(num)),
        Some(Token::Identifier(id)) => {
            if let Some(Token::Colon) = tokens.peek() {
                tokens.next();
                Ok(Syntax::Property(id, Box::new(parse(tokens)?)))
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
                } else if tok == &Token::Comma {
                    tokens.next();
                }
                statements_buf.push(parse(tokens)?);
            }
            match statements_buf
                .iter()
                .map(|syn| match syn {
                    Syntax::Property(k, v) => Some((k.clone(), *(*v).clone())),
                    _ => None,
                })
                .collect::<Option<BTreeMap<_, _>>>()
            {
                Some(props) => Ok(Syntax::Object(props)),
                None => Ok(Syntax::Block(statements_buf)),
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
