use std::rc::Rc;

use crate::interpreter::test_interpret;
use crate::types::{BlockType, Command, ExecuteOption, OpLeft, Operation, Syntax};

macro_rules! assert_e2e {
    ($src: expr => $output: expr) => {{
        let tokens = $crate::lexer::tokenize($src).unwrap();
        let syntax = $crate::parser::parse(&mut tokens.into_iter().peekable()).unwrap();
        let output = $crate::interpreter::test_interpret(&syntax).unwrap();
        assert_eq!($output, output);
        output
    }};
}

#[test]
fn if_statement() {
    assert_e2e!("if x = 1 { @function \"use/goodberry\" }" =>
        vec![Command::Execute {
            options: vec![ExecuteOption::ScoreMatches {
                invert: false,
                target: "%x".into(),
                objective: "dummy".into(),
                lower: Some(1),
                upper: Some(1)
            }],
            cmd: Box::new(Command::Function {
                func: "use/goodberry".into()
            })
        }]
    );
}
