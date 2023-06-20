mod lexer {
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
                Token::LSquirrely,
                Token::At,
                Token::Identifier("function".into()),
                Token::String("tick".into()),
                Token::RSquirrely
            ])
        );
    }
}

mod parser {
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
                OpLeft::Ident("x".into()),
                Operation::In,
                Box::new(Syntax::Range(Some(0), Some(10))),
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
            Ok(Syntax::BlockSelector(
                BlockSelectorType::As,
                Selector {
                    selector_type: SelectorType::S,
                    args: BTreeMap::new()
                },
                Box::new(Syntax::Unit)
            ))
        );
    }
}

mod interpreter {
    use std::rc::Rc;

    use crate::{interpreter::test_interpret, types::prelude::*};

    #[test]
    fn function() {
        assert_eq!(
            test_interpret(&Syntax::Macro(
                "function".into(),
                Box::new(Syntax::String("give/berry".into()))
            )),
            Ok(vec![Command::Function {
                func: "give/berry".into()
            }])
        );
    }

    #[test]
    fn tp_s_up() {
        assert_eq!(
            test_interpret(&Syntax::BlockSelector(
                BlockSelectorType::AsAt,
                Selector::r(),
                Box::new(Syntax::BlockSelector(
                    BlockSelectorType::Tp,
                    Selector::s(),
                    Box::new(Syntax::Array(Rc::from([
                        Syntax::WooglyCoord(0.0),
                        Syntax::WooglyCoord(10.0),
                        Syntax::WooglyCoord(0.0)
                    ])))
                ))
            )),
            Ok(vec![Command::Execute {
                options: vec![
                    ExecuteOption::As {
                        selector: Selector::r()
                    },
                    ExecuteOption::At {
                        selector: Selector::s()
                    }
                ],
                cmd: Box::new(Command::Teleport {
                    target: Selector::s(),
                    destination: Coordinate::Linear(true, 0.0, true, 10.0, true, 0.0)
                })
            }])
        );
    }

    #[test]
    fn as_s_if_score() {
        assert_eq!(
            test_interpret(&Syntax::BlockSelector(
                BlockSelectorType::As,
                Selector::r(),
                Box::new(Syntax::Block(
                    BlockType::If,
                    OpLeft::SelectorColon(Selector::s(), "count".into()),
                    Operation::RCaretEq,
                    Box::new(Syntax::Integer(3)),
                    Box::new(Syntax::Macro(
                        "function".into(),
                        Box::new(Syntax::String("give/my_item".into()))
                    ))
                ))
            )),
            Ok(vec![Command::Execute {
                options: vec![
                    ExecuteOption::As {
                        selector: Selector::r()
                    },
                    ExecuteOption::ScoreMatches {
                        invert: false,
                        target: "@s".into(),
                        objective: "count".into(),
                        lower: Some(3),
                        upper: None
                    }
                ],
                cmd: Box::new(Command::Function {
                    func: "give/my_item".into()
                })
            }])
        );
    }
}

mod types {
    use crate::types::farey_approximation;

    #[test]
    fn rational_approximator() {
        assert_eq!(farey_approximation(0.5, 10), (1, 2));
        assert_eq!(farey_approximation(2.5, 10), (5, 2));
        assert_eq!(farey_approximation(2.0, 10), (2, 1));
        assert_eq!(farey_approximation(1.618, 10), (13, 8));
        assert_eq!(farey_approximation(1.618, 100), (144, 89));
        // examples from https://www.johndcook.com/blog/2010/10/20/best-rational-approximation/
        assert_eq!(farey_approximation(0.367_879, 10), (3, 8));
        assert_eq!(farey_approximation(0.367_879, 100), (32, 87));
    }
}
