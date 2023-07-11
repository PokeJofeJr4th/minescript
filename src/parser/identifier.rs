use std::iter::Peekable;

use crate::types::prelude::*;

use super::{get_op, parse, parse_nbt_path};
/// parse a statement that starts with an identifier
#[allow(clippy::too_many_lines)]
pub(super) fn parse_identifier<T: Iterator<Item = Token>>(
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
    } else if tokens.peek() == Some(&Token::Dot) {
        let mut path = vec![NbtPathPart::Ident(id)];
        path.extend(parse_nbt_path(tokens)?);
        Ok(Syntax::NbtStorage(path))
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
    } else if matches!(id_ref, "summon" | "on" | "anchored" | "async" | "function") {
        let block_type = match id_ref {
            "summon" => IdentBlockType::Summon,
            "on" => IdentBlockType::On,
            "anchored" => IdentBlockType::Anchored,
            "async" => IdentBlockType::Async,
            "function" => IdentBlockType::Function,
            _ => unreachable!(),
        };
        let Some(Token::Identifier(ident) | Token::String(ident)) = tokens.next() else {
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
