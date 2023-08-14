use std::rc::Rc;

use crate::{interpreter::test_interpret, types::prelude::*};

#[test]
fn function() {
    assert_eq!(
        test_interpret(&Syntax::Annotation(
            "function".into(),
            Box::new(Syntax::String("give/berry".into()))
        )),
        vec![Command::Function("give/berry".into())]
    );
}

#[test]
fn tp_s_up() {
    assert_eq!(
        test_interpret(&Syntax::Block(
            BlockType::AsAt,
            Box::new(Syntax::Selector(Selector::r())),
            Box::new(Syntax::Block(
                BlockType::Tp,
                Box::new(Syntax::Selector(Selector::s())),
                Box::new(Syntax::Array(Rc::from([
                    Syntax::WooglyCoord(0.0),
                    Syntax::WooglyCoord(10.0),
                    Syntax::WooglyCoord(0.0)
                ])))
            ))
        )),
        vec![Command::Execute {
            options: vec![
                ExecuteOption::As(Selector::r()),
                ExecuteOption::At(Selector::s())
            ],
            cmd: Box::new(Command::Teleport {
                target: Selector::s(),
                destination: Coordinate::Linear(true, 0.0, true, 10.0, true, 0.0)
            })
        }]
    );
}

#[test]
fn as_s_if_score() {
    assert_eq!(
        test_interpret(&Syntax::Block(
            BlockType::As,
            Box::new(Syntax::Selector(Selector::r())),
            Box::new(Syntax::Block(
                BlockType::On,
                Box::new(Syntax::Identifier("owner".into())),
                Box::new(Syntax::Block(
                    BlockType::If,
                    Box::new(Syntax::BinaryOp {
                        lhs: OpLeft::SelectorColon(Selector::s(), "count".into()),
                        operation: Operation::RCaretEq,
                        rhs: Box::new(Syntax::Integer(3))
                    }),
                    Box::new(Syntax::Annotation(
                        "function".into(),
                        Box::new(Syntax::String("give/my_item".into()))
                    ))
                ))
            ))
        )),
        vec![Command::Execute {
            options: vec![
                ExecuteOption::As(Selector::r()),
                ExecuteOption::On("owner".into()),
                ExecuteOption::ScoreMatches {
                    invert: false,
                    target: "@s".into(),
                    objective: "count".into(),
                    lower: Some(3),
                    upper: None
                }
            ],
            cmd: Box::new(Command::Function("give/my_item".into()))
        }]
    );
}

#[test]
fn tellraw() {
    // tellraw @a [{\"text\":\"hello world\",\"italic\":true},{\"text\":\"plain\"}]
    assert_eq!(
        test_interpret(&Syntax::Block(
            BlockType::Tellraw,
            Box::new(Syntax::Selector(Selector::a())),
            Box::new(Syntax::Array(Rc::from([
                Syntax::Array(Rc::from([
                    Syntax::String("hello world".into()),
                    Syntax::Identifier("italic".into())
                ])),
                Syntax::String("plain".into())
            ])))
        )),
        vec![Command::TellRaw(
            Selector::a(),
            nbt!([
                nbt!({text: "hello world", italic: true}),
                nbt!({text: "plain"})
            ])
            .to_json()
            .into()
        )]
    );
}

#[test]
fn xp_ops() {
    assert_eq!(
        test_interpret(&Syntax::BinaryOp {
            lhs: OpLeft::SelectorDoubleColon(Selector::s(), "level".into()),
            operation: Operation::AddEq,
            rhs: Box::new(Syntax::Integer(2))
        }),
        vec![Command::XpAdd {
            target: Selector::s(),
            amount: 2,
            levels: true
        }]
    );
    assert_eq!(
        test_interpret(&Syntax::BinaryOp {
            lhs: OpLeft::Ident("x".into()),
            operation: Operation::MulEq,
            rhs: Box::new(Syntax::SelectorDoubleColon(Selector::s(), "lvl".into()))
        }),
        vec![
            Command::Execute {
                options: vec![ExecuteOption::StoreScore {
                    target: "%__xp__".into(),
                    objective: "dummy".into(),
                    is_success: false,
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
                source: "%__xp__".into(),
                source_objective: "dummy".into()
            }
        ]
    );
}
