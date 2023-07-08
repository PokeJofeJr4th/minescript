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
            Box::new(Syntax::SelectorDoubleColon(Selector::s(), "lvl".into()))
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
