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
            Ok(Syntax::SelectorBlock(
                SelectorBlockType::As,
                Selector {
                    selector_type: SelectorType::S,
                    args: BTreeMap::new()
                },
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
                OpLeft::Ident("x".into()),
                Operation::Equal,
                Box::new(Syntax::Integer(10)),
                Box::new(Syntax::Array(Rc::from([Syntax::BinaryOp(
                    OpLeft::Ident("x".into()),
                    Operation::AddEq,
                    Box::new(Syntax::Integer(1))
                )])))
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
            test_interpret(&Syntax::SelectorBlock(
                SelectorBlockType::AsAt,
                Selector::r(),
                Box::new(Syntax::SelectorBlock(
                    SelectorBlockType::Tp,
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
            test_interpret(&Syntax::SelectorBlock(
                SelectorBlockType::As,
                Selector::r(),
                Box::new(Syntax::IdentBlock(
                    IdentBlockType::On,
                    "owner".into(),
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
                ))
            )),
            Ok(vec![Command::Execute {
                options: vec![
                    ExecuteOption::As {
                        selector: Selector::r()
                    },
                    ExecuteOption::On {
                        ident: "owner".into()
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

    #[test]
    fn tellraw() {
        // tellraw @a [{\"text\":\"hello world\",\"italic\":true},{\"text\":\"plain\"}]
        assert_eq!(
            test_interpret(&Syntax::SelectorBlock(
                SelectorBlockType::TellRaw,
                Selector::a(),
                Box::new(Syntax::Array(Rc::from([
                    Syntax::Array(Rc::from([
                        Syntax::String("hello world".into()),
                        Syntax::Identifier("italic".into())
                    ])),
                    Syntax::String("plain".into())
                ])))
            )),
            Ok(vec![Command::TellRaw(
                Selector::a(),
                nbt!([
                    nbt!({text: "hello world", italic: true}),
                    nbt!({text: "plain"})
                ])
                .to_json()
                .into()
            )])
        );
    }

    #[test]
    fn xp_ops() {
        assert_eq!(
            test_interpret(&Syntax::BinaryOp(
                OpLeft::SelectorDoubleColon(Selector::s(), "level".into()),
                Operation::AddEq,
                Box::new(Syntax::Integer(2))
            )),
            Ok(vec![Command::XpAdd {
                target: Selector::s(),
                amount: 2,
                levels: true
            }])
        );
        assert_eq!(
            test_interpret(&Syntax::BinaryOp(
                OpLeft::Ident("x".into()),
                Operation::MulEq,
                Box::new(Syntax::BinaryOp(
                    OpLeft::Selector(Selector::s()),
                    Operation::DoubleColon,
                    Box::new(Syntax::Identifier("lvl".into()))
                ))
            )),
            Ok(vec![
                Command::Execute {
                    options: vec![ExecuteOption::StoreScore {
                        target: "%".into(),
                        objective: "dummy".into()
                    }],
                    cmd: Box::new(Command::XpGet {
                        target: Selector::s(),
                        levels: true
                    })
                },
                Command::ScoreOperation {
                    target: "%x".into(),
                    target_objective: "dummy".into(),
                    operation: Operation::MulEq,
                    source: "%".into(),
                    source_objective: "dummy".into()
                }
            ])
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
