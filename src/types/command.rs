use std::fmt::Display;

use super::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// A user-made command that passes through the compiler unchanged
    Raw(RStr),
    /// give a target an effect. Duration defaults to infinite, level defaults to 1
    EffectGive {
        target: Selector<String>,
        effect: RStr,
        duration: Option<i32>,
        level: Option<i32>,
    },
    // Kill {
    //     target: Selector<String>,
    // },
    /// call a function
    Function { func: RStr },
    /// set a score to a value
    ScoreSet {
        target: RStr,
        objective: RStr,
        value: i32,
    },
    /// add to a score
    ScoreAdd {
        target: RStr,
        objective: RStr,
        value: i32,
    },
    /// perform an operation between two scores
    ScoreOperation {
        target: RStr,
        target_objective: RStr,
        operation: Operation,
        source: RStr,
        source_objective: RStr,
    },
    /// execute a command with certain options
    Execute {
        options: Vec<ExecuteOption>,
        cmd: Box<Command>,
    },
    // Tag {
    //     target: Selector<String>,
    //     add: bool,
    //     tag: RStr,
    // },
    Teleport {
        target: Selector<String>,
        destination: Coordinate,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecuteOption {
    /// compare score to a static range
    ScoreMatches {
        invert: bool,
        target: RStr,
        objective: RStr,
        lower: Option<i32>,
        upper: Option<i32>,
    },
    /// compare score to another score
    ScoreSource {
        invert: bool,
        target: RStr,
        target_objective: RStr,
        operation: Operation,
        source: RStr,
        source_objective: RStr,
    },
    /// change who `@s` is
    As { selector: Selector<String> },
    /// change where the command executes
    At { selector: Selector<String> },
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
            Self::As { selector } => format!("as {selector}"),
            Self::At { selector } => format!("at {selector}"),
        }
    }
}

impl Command {
    pub fn stringify(&self, namespace: &str) -> String {
        match self {
            Self::Raw (cmd) => cmd.replace("<NAMESPACE>", namespace),
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
            Self::Teleport { target, destination } => format!("tp {target} {destination}")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Coordinate {
    /// coordinate given with xyz coordinates; booleans are for `~ ~ ~`
    Linear(bool, f32, bool, f32, bool, f32),
    /// coordinates given by angle; `^ ^ ^`
    Angular(f32, f32, f32),
}

impl Display for Coordinate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Linear(a, af, b, bf, c, cf) => {
                write!(
                    f,
                    "{}{af} {}{bf} {}{cf}",
                    if a { "~" } else { "" },
                    if b { "~" } else { "" },
                    if c { "~" } else { "" }
                )
            }
            Self::Angular(a, b, c) => {
                write!(f, "^{a} ^{b} ^{c}")
            }
        }
    }
}
