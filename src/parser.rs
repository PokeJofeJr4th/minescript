use std::{collections::BTreeMap, iter::Peekable, rc::Rc};

use crate::types::prelude::*;

mod identifier;

pub fn parse(tokens: Vec<Token>) -> SResult<Syntax> {
    inner_parse_expr_greedy(&mut tokens.into_iter().peekable(), 0)
}

fn inner_parse_expr_greedy(
    tokens: &mut Peekable<impl Iterator<Item = Token>>,
    priority: u8,
) -> SResult<Syntax> {
    if priority >= 5 {
        return inner_parse(tokens);
    }
    let mut start = inner_parse_expr_greedy(tokens, priority + 1)?;
    loop {
        match tokens.peek() {
            Some(Token::Star | Token::Slash) if priority == 0 => {
                let op = tokens.next().unwrap().try_into().unwrap();
                let rhs = inner_parse_expr_greedy(tokens, priority + 1)?;
                start = Syntax::BinaryOp {
                    lhs: Box::new(start),
                    operation: op,
                    rhs: Box::new(rhs),
                };
            }
            Some(Token::Plus | Token::Tack) if priority == 1 => {
                let op = tokens.next().unwrap().try_into().unwrap();
                let rhs = inner_parse_expr_greedy(tokens, priority + 1)?;
                start = Syntax::BinaryOp {
                    lhs: Box::new(start),
                    operation: op,
                    rhs: Box::new(rhs),
                };
            }
            Some(Token::Identifier(id)) if priority == 2 && &**id == "in" => {
                let op = tokens.next().unwrap().try_into().unwrap();
                let rhs = inner_parse_expr_greedy(tokens, priority + 1)?;
                start = Syntax::BinaryOp {
                    lhs: Box::new(start),
                    operation: op,
                    rhs: Box::new(rhs),
                };
            }
            Some(
                Token::Equal
                | Token::Colon
                | Token::DotEq
                | Token::DotPlusEq
                | Token::DotTackEq
                | Token::DotStarEq
                | Token::DotSlashEq
                | Token::RLCaret
                | Token::LCaret
                | Token::LCaretEq
                | Token::RCaret
                | Token::RCaretEq
                | Token::PlusEq
                | Token::TackEq
                | Token::StarEq
                | Token::SlashEq
                | Token::PercEq
                | Token::QuestionEq
                | Token::BangEq
                | Token::ColonEq,
            ) if priority == 3 => {
                let op = tokens.next().unwrap().try_into().unwrap();
                let rhs = inner_parse_expr_greedy(tokens, priority + 1)?;
                start = Syntax::BinaryOp {
                    lhs: Box::new(start),
                    operation: op,
                    rhs: Box::new(rhs),
                };
            }
            _ => return Ok(start),
        }
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
fn inner_parse<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> SResult<Syntax> {
    let first = match tokens.next() {
        Some(Token::String(str)) => Ok(Syntax::String(str)),
        Some(Token::Integer(num)) => Ok(Syntax::Integer(num)),
        Some(Token::Float(num)) => Ok(Syntax::Float(num)),
        Some(Token::Range(l, r)) => Ok(Syntax::Range(l, r)),
        Some(Token::DotDot) => {
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
        Some(Token::Identifier(id)) => identifier::parse_identifier(tokens, id),
        Some(Token::LCurly) => {
            parse_block(tokens, &Token::RCurly, || Syntax::Object(BTreeMap::new()))
        }
        Some(Token::LSquare) => {
            parse_block(tokens, &Token::RSquare, || Syntax::Array(Rc::from([])))
        }
        Some(Token::LParen) => parse_block(tokens, &Token::RParen, || Syntax::Unit),
        Some(Token::At) => parse_annotation(tokens),
        Some(Token::UCaret) => Ok(Syntax::CaretCoord(extract_float(tokens)?)),
        Some(Token::Woogly) => Ok(Syntax::WooglyCoord(extract_float(tokens)?)),
        Some(Token::Bang) => {
            if let Some(Token::Identifier(ident)) = tokens.next() {
                Ok(Syntax::Identifier(format!("!{ident}").into()))
            } else {
                Err(String::from(
                    "Unexpected token `!`. Try appending an identifier, like `@s[type=!player]`.",
                ))
            }
        }
        other => Err(format!("Unexpected token `{other:?}`")),
    }?;
    if let Syntax::Selector(sel) = &first {
        // println!("Selector");
        if tokens.peek() == Some(&Token::Colon)
            || tokens.peek() == Some(&Token::DoubleColon)
            || tokens.peek() == Some(&Token::Dot)
        {
            let tok = tokens.next();
            let Some(Token::Identifier(ident)) = tokens.next() else {
                return Err(String::from("Selectors can only be indexed with `:<identifier>`, `::<identifier>`, or `.<nbt>`"))
            };
            let left = match tok {
                Some(Token::Colon) => Syntax::SelectorColon(sel.clone(), ident),
                Some(Token::DoubleColon) => Syntax::SelectorDoubleColon(sel.clone(), ident),
                Some(Token::Dot) => {
                    let mut nbt = vec![NbtPathPart::Ident(ident)];
                    nbt.extend(parse_nbt_path(tokens)?);
                    Syntax::SelectorNbt(sel.clone(), nbt)
                }
                _ => unreachable!(),
            };
            let (op, right) = if let Ok(op) = tokens.peek().unwrap().clone().try_into() {
                // println!("Secondary Operation");
                tokens.next();
                (op, inner_parse(tokens)?)
            } else if tokens.peek() == Some(&Token::PlusPlus) {
                tokens.next();
                (Operation::AddEq, Syntax::Integer(1))
            } else if tokens.peek() == Some(&Token::TackTack) {
                tokens.next();
                (Operation::SubEq, Syntax::Integer(1))
            } else {
                return Ok(left);
            };
            return Ok(Syntax::BinaryOp {
                lhs: Box::new(left),
                operation: op,
                rhs: Box::new(right),
            });
        }
    } else if let Syntax::NbtStorage(nbt) = &first {
        if Some(&Token::Equal) == tokens.peek() {
            tokens.next();
            return Ok(Syntax::BinaryOp {
                lhs: Box::new(Syntax::NbtStorage(nbt.clone())),
                operation: Operation::Equal,
                rhs: Box::new(inner_parse(tokens)?),
            });
        }
    }
    // println!("{first:?}");
    Ok(first)
}

/// get an nbt path, like `.Inventory[42].tag`
fn parse_nbt_path<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> SResult<NbtPath> {
    let mut path_buf: NbtPath = Vec::new();
    loop {
        match tokens.peek() {
            // `___.tag`
            Some(Token::Dot) => {
                tokens.next();
                match tokens.next() {
                    Some(Token::Identifier(ident)) => path_buf.push(NbtPathPart::Ident(ident)),
                    other => {
                        return Err(format!(
                            "Expected identifier after `.` in NBT path; got `{other:?}`"
                        ))
                    }
                }
            }
            // `___[42]`
            Some(Token::LSquare) => {
                tokens.next();
                match tokens.next() {
                    Some(Token::Integer(int @ 0..)) => {
                        #[allow(clippy::cast_sign_loss)]
                        // we know the integer is positive because of the match statement
                        path_buf.push(NbtPathPart::Index(int as u32));
                        if tokens.next() != Some(Token::RSquare) {
                            return Err(format!("Expected `]` after `[{int}` in NBT path"));
                        }
                    }
                    other => {
                        return Err(format!(
                            "Expected number after `.` in NBT path; got `{other:?}`"
                        ))
                    }
                }
            }
            // something else; end the thingy
            _ => break,
        }
    }
    Ok(path_buf)
}

#[allow(clippy::cast_precision_loss)]
fn extract_float<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> SResult<f32> {
    Ok(match tokens.peek() {
        Some(Token::Integer(_)) => {
            let Some(Token::Integer(int)) = tokens.next() else {unreachable!()};
            int as f32
        }
        Some(Token::Float(_)) => {
            let Some(Token::Float(float)) = tokens.next() else {unreachable!()};
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

/// get a list of statements with the given closing character; like `{...}` or `(...)`
fn parse_block<T: Iterator<Item = Token>>(
    tokens: &mut Peekable<T>,
    closing: &Token,
    default: impl Fn() -> Syntax,
) -> SResult<Syntax> {
    let mut statements_buf = Vec::new();
    if tokens.peek() == Some(closing) {
        tokens.next();
        return Ok(default());
    }
    statements_buf.push(inner_parse_expr_greedy(tokens, 0)?);

    while let Some(tok) = tokens.peek() {
        if tok == closing {
            tokens.next();
            break;
        } else if tok == &Token::Comma || tok == &Token::SemiColon {
            tokens.next();
        } else {
            // println!("Curly Object");
            statements_buf.push(inner_parse_expr_greedy(tokens, 0)?);
        }
    }
    statements_buf
        .iter()
        .map(|syn| match syn {
            Syntax::BinaryOp {
                lhs: k,
                operation: Operation::Colon,
                rhs: v,
            } => {
                let Syntax::Identifier(k) = &**k else { return None };
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

/// parse a statement that starts with `@`
fn parse_annotation<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> SResult<Syntax> {
    let Some(Token::Identifier(identifier)) = tokens.next() else {
        return Err("Expected identifier after `@`".to_string())
    };
    match identifier.as_ref() {
        "s" | "p" | "e" | "a" | "r" => {
            let mut selector_buf = BTreeMap::new();
            if tokens.peek() == Some(&Token::LSquare) {
                tokens.next();
                while let Some(tok) = tokens.next() {
                    if tok == Token::RSquare {
                        break;
                    } else if tok == Token::Comma {
                        continue;
                    }
                    let Token::Identifier(ident) = tok else {
                        return Err(format!("Expected a selector parameter; got `{tok:?}`"))
                    };
                    let Some(Token::Equal) = tokens.next() else {
                        return Err(String::from("Expected `=` for selector property"))
                    };
                    selector_buf.insert(ident.clone(), inner_parse(tokens)?);
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
                args: selector_buf,
            }))
        }
        _ => Ok(Syntax::Annotation(
            identifier,
            Box::new(inner_parse(tokens)?),
        )),
    }
}
