use crate::parser::Operation;

use super::prelude::*;

#[derive(Debug, Clone)]
pub enum Command {
    EffectGive {
        target: Selector<String>,
        effect: RStr,
        duration: Option<i32>,
        level: Option<i32>,
    },
    // Kill {
    //     target: Selector<String>,
    // },
    Function {
        func: RStr,
    },
    ScoreSet {
        target: RStr,
        objective: RStr,
        value: i32,
    },
    ScoreAdd {
        target: RStr,
        objective: RStr,
        value: i32,
    },
    ScoreOperation {
        target: RStr,
        target_objective: RStr,
        operation: Operation,
        source: RStr,
        source_objective: RStr,
    },
    Execute {
        options: Vec<ExecuteOption>,
        cmd: Box<Command>,
    },
    // Tag {
    //     target: Selector<String>,
    //     add: bool,
    //     tag: RStr,
    // },
}

#[derive(Debug, Clone)]
pub enum ExecuteOption {
    ScoreMatches {
        invert: bool,
        target: RStr,
        objective: RStr,
        lower: Option<i32>,
        upper: Option<i32>,
    },
    ScoreSource {
        invert: bool,
        target: RStr,
        target_objective: RStr,
        operation: Operation,
        source: RStr,
        source_objective: RStr,
    },
}

impl ExecuteOption {
    pub fn stringify(&self, _namespace: &str) -> String {
        match self {
            Self::ScoreMatches {
                invert,
                target,
                objective,
                lower,
                upper,
            } => {
                let match_statement = if lower == upper {
                    lower.map_or_else(|| String::from(".."), |l| format!("{l}"))
                } else {
                    format!(
                        "{}..{}",
                        lower.map_or_else(String::new, |l| format!("{l}")),
                        upper.map_or_else(String::new, |u| format!("{u}"))
                    )
                };
                format!(
                    "{} score {target} {objective} matches {}",
                    if *invert { "unless" } else { "if" },
                    match_statement
                )
            }
            Self::ScoreSource {
                invert,
                target,
                target_objective,
                operation,
                source,
                source_objective,
            } => format!(
                "{} score {target} {target_objective} {operation} {source} {source_objective}",
                if *invert { "unless" } else { "if" }
            ),
        }
    }
}

impl Command {
    pub fn stringify(&self, namespace: &str) -> String {
        match self {
            Self::EffectGive {
                target,
                effect,
                duration,
                level,
            } => {
                format!(
                    "effect give {target} {effect} {} {}",
                    duration.map_or_else(|| String::from("infinite"), |num| format!("{num}")),
                    level.unwrap_or(0)
                )
            }
            // Self::Kill { target } => format!("kill {target}"),
            Self::Function { func } => format!("function {namespace}:{func}"),
            // Self::Tag { target, add, tag } => format!("tag {} {target} {tag}", if *add {
            //     "add"
            // } else {
            //     "remove"
            // }),
            Self::ScoreSet {
                target: player,
                objective: score,
                value,
            } => if *value == 0 {
                format!("scoreboard players reset {player} {score}")
            } else {
                format!("scoreboard players set {player} {score} {value}")
            }
            Self::ScoreAdd {
                target: player,
                objective: score,
                value,
            } => format!("scoreboard players add {player} {score} {value}"),
            Self::ScoreOperation {
                target,
                target_objective,
                operation,
                source,
                source_objective,
            } => format!("scoreboard players operation {target} {target_objective} {operation} {source} {source_objective}"),
            Self::Execute { options, cmd } => {
                let mut options_buf = String::new();
                for option in options {
                    options_buf.push_str(&option.stringify(namespace));
                    options_buf.push(' ');
                }
                format!("execute {options_buf}run {}", cmd.stringify(namespace))
            },
        }
    }
}