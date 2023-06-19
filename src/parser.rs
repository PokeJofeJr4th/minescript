use std::{collections::BTreeMap, iter::Peekable};

use crate::types::prelude::*;

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]
pub fn parse<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> SResult<Syntax> {
    let first = match tokens.next() {
        Some(Token::String(str)) => Ok(Syntax::String(str)),
        Some(Token::Integer(num)) => Ok(Syntax::Integer(num)),
        Some(Token::Float(num)) => Ok(Syntax::Float(num)),
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
        Some(Token::LSquirrely) => parse_block(tokens, &Token::RSquirrely),
        Some(Token::LSquare) => parse_block(tokens, &Token::RSquare),
        Some(Token::LParen) => parse_block(tokens, &Token::RParen),
        Some(Token::At) => parse_macro(tokens),
        Some(Token::UCaret) => Ok(Syntax::CaretCoord(extract_float(tokens)?)),
        Some(Token::Woogly) => Ok(Syntax::WooglyCoord(extract_float(tokens)?)),
        other => Err(format!("Unexpected token `{other:?}`")),
    }?;
    match &first {
        Syntax::BinaryOp(OpLeft::Ident(first), Operation::Colon, second) => 'm: {
            // println!("Binary Operation");
            if let Syntax::Identifier(second) = &**second {
                let left = OpLeft::Colon(first.clone(), second.clone());
                let (op, right) = if let Some(op) = get_op(tokens) {
                    // println!("Secondary Operation");
                    (op, parse(tokens)?)
                } else if tokens.peek() == Some(&Token::PlusPlus) {
                    tokens.next();
                    (Operation::AddEq, Syntax::Integer(1))
                } else if tokens.peek() == Some(&Token::TackTack) {
                    tokens.next();
                    (Operation::SubEq, Syntax::Integer(1))
                } else {
                    break 'm;
                };
                return Ok(Syntax::BinaryOp(left, op, Box::new(right)));
            }
        }
        Syntax::Selector(sel) => 'm: {
            // println!("Selector");
            if tokens.peek() == Some(&Token::Colon) {
                tokens.next();
                let Some(Token::Identifier(ident)) = tokens.next() else {
                    return Err(String::from("Selectors can only be indexed with `:<identifier>`"))
                };
                let left = OpLeft::SelectorColon(sel.clone(), ident);
                let (op, right) = if let Some(op) = get_op(tokens) {
                    // println!("Secondary Operation");
                    (op, parse(tokens)?)
                } else if tokens.peek() == Some(&Token::PlusPlus) {
                    tokens.next();
                    (Operation::AddEq, Syntax::Integer(1))
                } else if tokens.peek() == Some(&Token::TackTack) {
                    tokens.next();
                    (Operation::SubEq, Syntax::Integer(1))
                } else {
                    break 'm;
                };
                return Ok(Syntax::BinaryOp(left, op, Box::new(right)));
            }
        }
        _ => {}
    }
    // println!("{first:?}");
    Ok(first)
}

#[allow(clippy::cast_precision_loss)]
fn extract_float<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> SResult<f32> {
    Ok(match tokens.peek() {
        Some(Token::Integer(_)) => {
            let Some(Token::Integer(int)) = tokens.next() else {panic!()};
            int as f32
        }
        Some(Token::Float(_)) => {
            let Some(Token::Float(float)) = tokens.next() else {panic!()};
            float
        }
        Some(Token::Tack) => {
            tokens.next();
            -match tokens.next() {
                Some(Token::Integer(int)) => int as f32,
                Some(Token::Float(float)) => float,
                other => return Err(format!("Expected int or float after `-`; got `{other:?}`")),
            }
        }
        _ => 0.0,
    })
}

fn parse_block<T: Iterator<Item = Token>>(
    tokens: &mut Peekable<T>,
    closing: &Token,
) -> SResult<Syntax> {
    let mut statements_buf = Vec::new();
    if tokens.peek() == Some(closing) {
        tokens.next();
        return Ok(Syntax::Unit);
    }
    statements_buf.push(parse(tokens)?);

    while let Some(tok) = tokens.peek() {
        if tok == closing {
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
) -> SResult<Syntax> {
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
    } else if &*id == "if" || &*id == "do" || &*id == "while" || &*id == "for" {
        if &*id == "do" && tokens.next() != Some(Token::Identifier("while".into())) {
            return Err(String::from("Expected `while` after `do`"));
        }
        let Syntax::BinaryOp(left, op, right) = parse(tokens)? else {
            return Err(format!("{id} statement requires a check like `x = 2`"))
        };
        let block_type = match &*id {
            "if" => BlockType::If,
            "do" => BlockType::DoWhile,
            "while" => BlockType::While,
            "for" => BlockType::For,
            _ => unreachable!(),
        };
        Ok(Syntax::Block(
            block_type,
            left,
            op,
            right,
            Box::new(parse(tokens)?),
        ))
    } else if &*id == "as" || &*id == "at" || &*id == "asat" || &*id == "tp" || &*id == "teleport" {
        let block_type = match &*id {
            "as" => {
                if let Some(Token::Identifier(id)) = tokens.peek() {
                    if &**id == "at" {
                        tokens.next();
                        BlockSelectorType::AsAt
                    } else {
                        BlockSelectorType::As
                    }
                } else {
                    BlockSelectorType::As
                }
            }
            "at" => BlockSelectorType::At,
            "tp" | "teleport" => BlockSelectorType::Tp,
            _ => unreachable!(),
        };
        let Syntax::Selector(sel) = parse(tokens)? else {
            return Err(format!("{id} requires a selector afterwards"))
        };
        Ok(Syntax::BlockSelector(
            block_type,
            sel,
            Box::new(parse(tokens)?),
        ))
    } else {
        Ok(Syntax::Identifier(id))
    }
}

fn parse_macro<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> SResult<Syntax> {
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
