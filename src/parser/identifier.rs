use std::iter::Peekable;

use crate::types::prelude::*;

use super::{get_op, parse, parse_nbt_path};
/// parse a statement that starts with an identifier
#[allow(clippy::too_many_lines)]
pub(super) fn parse_identifier<T: Iterator<Item = Token>>(
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
    } else if tokens.peek() == Some(&Token::Dot) {
        let mut path = vec![NbtPathPart::Ident(id)];
        path.extend(parse_nbt_path(tokens)?);
        Ok(Syntax::NbtStorage(path))
    } else if let Ok(mut block_type) = BlockType::try_from(&*id) {
        if block_type == BlockType::As && tokens.peek() == Some(&Token::Identifier("at".into())) {
            tokens.next();
            block_type = BlockType::AsAt;
        } else if block_type == BlockType::Do
            && tokens.peek() == Some(&Token::Identifier("while".into()))
        {
            tokens.next();
            block_type = BlockType::DoWhile;
        } else if block_type == BlockType::Do
            && tokens.peek() == Some(&Token::Identifier("until".into()))
        {
            tokens.next();
            block_type = BlockType::DoUntil;
        } else if block_type == BlockType::Do {
            return Err(String::from(
                "`do` is not a valid block type; did you mean `do while` or `do until`?",
            ));
        }
        Ok(Syntax::Block(
            block_type,
            Box::new(parse(tokens)?),
            Box::new(parse(tokens)?),
        ))
    } else {
        Ok(Syntax::Identifier(id))
    }
}
