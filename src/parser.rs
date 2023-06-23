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
    if let Syntax::Selector(sel) = &first {
        'm: {
            // println!("Selector");
            if tokens.peek() == Some(&Token::Colon) || tokens.peek() == Some(&Token::DoubleColon) {
                let tok = tokens.next();
                let Some(Token::Identifier(ident)) = tokens.next() else {
                    return Err(String::from("Selectors can only be indexed with `:<identifier>` or `::<identifier>`"))
                };
                let left = match tok {
                    Some(Token::Colon) => OpLeft::SelectorColon(sel.clone(), ident),
                    Some(Token::DoubleColon) => OpLeft::SelectorDoubleColon(sel.clone(), ident),
                    _ => unreachable!(),
                };
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
                    // don't return anything if it's not an increment
                    break 'm;
                };
                return Ok(Syntax::BinaryOp(left, op, Box::new(right)));
            }
        }
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
        Some(Token::DoubleColon) => Some(Operation::DoubleColon),
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
                Some(Token::LCaret) => Some(Operation::Swap),
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

#[allow(clippy::too_many_lines)]
fn parse_identifier<T: Iterator<Item = Token>>(
    tokens: &mut Peekable<T>,
    id: RStr,
) -> SResult<Syntax> {
    let id_ref = id.as_ref();
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
    } else if id_ref == "function" {
        let Some(Token::Identifier(func) | Token::String(func)) = tokens.next() else {
                return Err(String::from("Expected identifier after function"))
            };
        Ok(Syntax::Function(func, Box::new(parse(tokens)?)))
    // } else if id == "effect" {
    //     todo!()
    } else if matches!(id_ref, "if" | "unless" | "do" | "while" | "until" | "for") {
        let id_ref = if id_ref == "do" {
            match tokens.next() {
                Some(Token::Identifier(ident)) => match &*ident {
                    "while" => "do while",
                    "until" => "do until",
                    _ => return Err(String::from("Expected `while` or `until` after `do`")),
                },
                _ => return Err(String::from("Expected `while` or `until` after `do`")),
            }
        } else {
            id_ref
        };
        match (parse(tokens)?, id_ref) {
            (Syntax::BinaryOp(left, op, right), _) => {
                let block_type = match id_ref {
                    "if" => BlockType::If,
                    "unlesss" => BlockType::Unless,
                    "while" => BlockType::While,
                    "do while" => BlockType::DoWhile,
                    "until" => BlockType::Until,
                    "do until" => BlockType::DoUntil,
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
            }
            // if @s {...}
            (Syntax::Selector(sel), "if") => Ok(Syntax::SelectorBlock(
                SelectorBlockType::IfEntity,
                sel,
                Box::new(parse(tokens)?),
            )),
            // unless @s {...}
            (Syntax::Selector(sel), "unless") => Ok(Syntax::SelectorBlock(
                SelectorBlockType::UnlessEntity,
                sel,
                Box::new(parse(tokens)?),
            )),
            _ => return Err(format!("{id} statement requires a check like `x = 2`")),
        }
    } else if matches!(id_ref, "summon" | "on" | "anchored") {
        let block_type = match id_ref {
            "summon" => IdentBlockType::Summon,
            "on" => IdentBlockType::On,
            "anchored" => IdentBlockType::Anchored,
            _ => unreachable!(),
        };
        let Some(Token::Identifier(ident)) = tokens.next() else {
            return Err(format!("`{id}` requires an identifier next"))
        };
        Ok(Syntax::IdentBlock(
            block_type,
            ident,
            Box::new(parse(tokens)?),
        ))
    } else if matches!(
        id_ref,
        "as" | "at" | "asat" | "tp" | "teleport" | "facing" | "rotated"
    ) {
        // as @s {...}
        let block_type = match id_ref {
            "as" => {
                if let Some(Token::Identifier(id)) = tokens.peek() {
                    if &**id == "at" {
                        tokens.next();
                        SelectorBlockType::AsAt
                    } else {
                        SelectorBlockType::As
                    }
                } else {
                    SelectorBlockType::As
                }
            }
            "asat" => SelectorBlockType::AsAt,
            "at" => SelectorBlockType::At,
            "tp" | "teleport" => SelectorBlockType::Tp,
            "facing" => SelectorBlockType::FacingEntity,
            "rotated" => SelectorBlockType::Rotated,
            _ => unreachable!(),
        };
        let Syntax::Selector(sel) = parse(tokens)? else {
            return Err(format!("{id} requires a selector afterwards"))
        };
        Ok(Syntax::SelectorBlock(
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
