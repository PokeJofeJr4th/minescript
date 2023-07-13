use std::{collections::BTreeMap, rc::Rc};

use crate::{parser::parse, types::prelude::*};

#[test]
fn literals() {
    // -20
    assert_eq!(
        parse(&mut [Token::Tack, Token::Integer(20)].into_iter().peekable()),
        Ok(Syntax::Integer(-20))
    );
}

#[test]
fn score_op() {
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
fn in_range() {
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
fn for_loop() {
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
            Box::new(Syntax::BinaryOp(
                OpLeft::Ident("x".into()),
                Operation::In,
                Box::new(Syntax::Range(Some(0), Some(10))),
            )),
            Box::new(Syntax::Unit)
        ))
    );
}

#[test]
fn coords() {
    // (^ ^2 ^1.5)
    assert_eq!(
        parse(
            &mut [
                Token::LParen,
                Token::UCaret,
                Token::UCaret,
                Token::Integer(2),
                Token::UCaret,
                Token::Float(1.5),
                Token::RParen,
            ]
            .into_iter()
            .peekable()
        ),
        Ok(Syntax::Array(Rc::from([
            Syntax::CaretCoord(0.0),
            Syntax::CaretCoord(2.0),
            Syntax::CaretCoord(1.5)
        ])))
    );
    // (~ ~-2 ~1.05)
    assert_eq!(
        parse(
            &mut [
                Token::LParen,
                Token::Woogly,
                Token::Woogly,
                Token::Tack,
                Token::Integer(2),
                Token::Woogly,
                Token::Float(1.05),
                Token::RParen,
            ]
            .into_iter()
            .peekable()
        ),
        Ok(Syntax::Array(Rc::from([
            Syntax::WooglyCoord(0.0),
            Syntax::WooglyCoord(-2.0),
            Syntax::WooglyCoord(1.05)
        ])))
    );
}

#[test]
fn tp() {
    assert_eq!(
        parse(
            &mut [
                Token::Identifier("as".into()),
                Token::At,
                Token::Identifier("s".into()),
                Token::LSquirrely,
                Token::RSquirrely
            ]
            .into_iter()
            .peekable()
        ),
        Ok(Syntax::Block(
            BlockType::As,
            Box::new(Syntax::Selector(Selector::s())),
            Box::new(Syntax::Unit)
        ))
    );
}

#[test]
fn xp_op() {
    assert_eq!(
        parse(
            &mut [
                Token::At,
                Token::Identifier("s".into()),
                Token::DoubleColon,
                Token::Identifier("level".into()),
                Token::PlusEq,
                Token::Integer(1)
            ]
            .into_iter()
            .peekable()
        ),
        Ok(Syntax::BinaryOp(
            OpLeft::SelectorDoubleColon(Selector::s(), "level".into()),
            Operation::AddEq,
            Box::new(Syntax::Integer(1))
        ))
    );
}

#[test]
fn do_until() {
    assert_eq!(
        parse(
            &mut [
                Token::Identifier("do".into()),
                Token::Identifier("until".into()),
                Token::Identifier("x".into()),
                Token::Equal,
                Token::Integer(10),
                Token::LSquirrely,
                Token::Identifier("x".into()),
                Token::PlusPlus,
                Token::RSquirrely
            ]
            .into_iter()
            .peekable()
        ),
        Ok(Syntax::Block(
            BlockType::DoUntil,
            Box::new(Syntax::BinaryOp(
                OpLeft::Ident("x".into()),
                Operation::Equal,
                Box::new(Syntax::Integer(10)),
            )),
            Box::new(Syntax::Array(Rc::from([Syntax::BinaryOp(
                OpLeft::Ident("x".into()),
                Operation::AddEq,
                Box::new(Syntax::Integer(1))
            )])))
        ))
    );
}
