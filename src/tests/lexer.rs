use crate::lexer::tokenize;
use crate::types::Token;

#[test]
fn literals() {
    assert_eq!(
        tokenize("-20.02"),
        Ok(vec![Token::Tack, Token::Float(20.02),])
    );
    assert_eq!(
        tokenize("0-lol -0"),
        Ok(vec![
            Token::Integer(0),
            Token::Tack,
            Token::Identifier(String::from("lol").into()),
            Token::Tack,
            Token::Integer(0)
        ])
    );
}

#[test]
fn for_loop() {
    assert_eq!(tokenize("1..10"), Ok(vec![Token::Range(Some(1), Some(10))]));
    assert_eq!(
        tokenize("for x in 1..10 {@function \"tick\"}"),
        Ok(vec![
            Token::Identifier("for".into()),
            Token::Identifier("x".into()),
            Token::Identifier("in".into()),
            Token::Range(Some(1), Some(10)),
            Token::LCurly,
            Token::At,
            Token::Identifier("function".into()),
            Token::String("tick".into()),
            Token::RCurly
        ])
    );
}

#[test]
fn get_xp() {
    assert_eq!(
        tokenize("@s::level += 1"),
        Ok(vec![
            Token::At,
            Token::Identifier("s".into()),
            Token::DoubleColon,
            Token::Identifier("level".into()),
            Token::PlusEq,
            Token::Integer(1)
        ])
    );
}
