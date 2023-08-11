use std::{collections::BTreeMap, iter::Peekable, rc::Rc};

use crate::types::prelude::*;

mod identifier;

#[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
pub fn parse<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> SResult<Syntax> {
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
                Some(Token::Colon) => OpLeft::SelectorColon(sel.clone(), ident),
                Some(Token::DoubleColon) => OpLeft::SelectorDoubleColon(sel.clone(), ident),
                Some(Token::Dot) => {
                    let mut nbt = vec![NbtPathPart::Ident(ident)];
                    nbt.extend(parse_nbt_path(tokens)?);
                    OpLeft::SelectorNbt(sel.clone(), nbt)
                }
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
                match left {
                    OpLeft::SelectorColon(sel, ident) => {
                        return Ok(Syntax::SelectorColon(sel, ident))
                    }
                    OpLeft::SelectorDoubleColon(sel, ident) => {
                        return Ok(Syntax::SelectorDoubleColon(sel, ident))
                    }
                    OpLeft::SelectorNbt(sel, nbt) => return Ok(Syntax::SelectorNbt(sel, nbt)),
                    _ => unreachable!(),
                }
            };
            return Ok(Syntax::BinaryOp {
                lhs: left,
                operation: op,
                rhs: Box::new(right),
            });
        }
    } else if let Syntax::NbtStorage(nbt) = &first {
        if Some(&Token::Equal) == tokens.peek() {
            tokens.next();
            return Ok(Syntax::BinaryOp {
                lhs: OpLeft::NbtStorage(nbt.clone()),
                operation: Operation::Equal,
                rhs: Box::new(parse(tokens)?),
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
    statements_buf.push(parse(tokens)?);

    while let Some(tok) = tokens.peek() {
        if tok == closing {
            tokens.next();
            break;
        } else if tok == &Token::Comma || tok == &Token::SemiColon {
            tokens.next();
        } else {
            // println!("Curly Object");
            statements_buf.push(parse(tokens)?);
        }
    }
    statements_buf
        .iter()
        .map(|syn| match syn {
            Syntax::BinaryOp {
                lhs: OpLeft::Ident(k),
                operation: Operation::Colon,
                rhs: v,
            } => Some((k.clone(), *(*v).clone())),
            _ => None,
        })
        .collect::<Option<BTreeMap<_, _>>>()
        .map_or_else(
            || Ok(Syntax::Array(statements_buf.into())),
            |props| Ok(Syntax::Object(props)),
        )
}

/// get and consume an operation from the next token(s)
fn get_op<T: Iterator<Item = Token>>(tokens: &mut Peekable<T>) -> Option<Operation> {
    let val = match tokens.peek() {
        Some(Token::Colon) => Some(Operation::Colon),
        Some(Token::DoubleColon) => Some(Operation::DoubleColon),
        Some(Token::Equal) => Some(Operation::Equal),
        Some(Token::LCaretEq) => Some(Operation::LCaretEq),
        Some(Token::RCaretEq) => Some(Operation::RCaretEq),
        Some(Token::LCaret) => Some(Operation::LCaret),
        Some(Token::RCaret) => Some(Operation::RCaret),
        Some(Token::RLCaret) => Some(Operation::Swap),
        Some(Token::BangEq) => Some(Operation::BangEq),
        Some(Token::PlusEq) => Some(Operation::AddEq),
        Some(Token::TackEq) => Some(Operation::SubEq),
        Some(Token::StarEq) => Some(Operation::MulEq),
        Some(Token::SlashEq) => Some(Operation::DivEq),
        Some(Token::PercEq) => Some(Operation::ModEq),
        Some(Token::ColonEq) => Some(Operation::ColonEq),
        Some(Token::QuestionEq) => Some(Operation::QuestionEq),
        Some(Token::Identifier(ident)) => {
            if ident.as_ref() == "in" {
                Some(Operation::In)
            } else {
                None
            }
        }
        _ => None,
    };
    if val.is_some() {
        tokens.next();
    }
    val
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
                    selector_buf.insert(ident.clone(), parse(tokens)?);
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
        _ => Ok(Syntax::Annotation(identifier, Box::new(parse(tokens)?))),
    }
}
