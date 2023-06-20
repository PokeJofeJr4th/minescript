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
    /// teleport the selected entity(ies) to the destination
    Teleport {
        target: Selector<String>,
        destination: Coordinate,
    },
    Sound {
        sound: RStr,
        source: RStr,
        target: Selector<String>,
        pos: Coordinate,
        volume: f32,
        pitch: f32,
        min_volume: f32,
    },
    Damage {
        target: Selector<String>,
        amount: i32,
        damage_type: RStr,
        attacker: Selector<String>,
    },
}

impl Command {
    /// Convert the command to a string within the given namespace
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
            Self::Teleport { target, destination } => format!("tp {target} {destination}"),
            Self::Sound { sound, source, target, pos, volume, pitch, min_volume } => format!("playsound {sound} {source} {target} {pos} {volume} {pitch} {min_volume}"),
            Self::Damage { target, amount, damage_type, attacker } => format!("damage {target} {amount} {damage_type} by {attacker}")
        }
    }

    /// Create an Execute command that runs the specified other command.
    ///
    /// If the other command is an execute, it combines them into one.
    /// If there are no execute subcommands, it returns the given command.
    pub fn execute(mut options: Vec<ExecuteOption>, cmd: Self) -> Self {
        match cmd {
            Self::Execute {
                options: inner_options,
                cmd,
            } => {
                options.extend(inner_options);
                Self::Execute { options, cmd }
            }
            _ => {
                if options.is_empty() {
                    cmd
                } else {
                    Self::Execute {
                        options,
                        cmd: Box::new(cmd),
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Coordinate {
    /// coordinate given with xyz coordinates; booleans are for `~ ~ ~`
    Linear(bool, f32, bool, f32, bool, f32),
    /// coordinates given by angle; `^ ^ ^`
    Angular(f32, f32, f32),
}

impl Coordinate {
    pub const fn here() -> Self {
        Self::Linear(true, 0.0, true, 0.0, true, 0.0)
    }
}

impl TryFrom<&Syntax> for Coordinate {
    type Error = String;

    fn try_from(body: &Syntax) -> SResult<Self> {
        let Syntax::Array(arr) = body else {
        return Err(format!("Tp requires a list of 3 coordinates; got `{body:?}`"))
    };
        let [a, b, c] = &arr[..] else {
        return Err(format!("Tp requires a list of 3 coordinates; got `{body:?}`"))
    };
        Ok(
            if let (Syntax::CaretCoord(a), Syntax::CaretCoord(b), Syntax::CaretCoord(c)) = (a, b, c)
            {
                Self::Angular(*a, *b, *c)
            } else {
                let (a, af) = match a {
                    Syntax::WooglyCoord(float) => (true, *float),
                    Syntax::Integer(int) => (false, *int as f32),
                    Syntax::Float(float) => (false, *float),
                    _ => return Err(format!("Tp requires a list of 3 coordinates; got `{a:?}`")),
                };
                let (b, bf) = match b {
                    Syntax::WooglyCoord(float) => (true, *float),
                    Syntax::Integer(int) => (false, *int as f32),
                    Syntax::Float(float) => (false, *float),
                    _ => return Err(format!("Tp requires a list of 3 coordinates; got `{b:?}`")),
                };
                let (c, cf) = match c {
                    Syntax::WooglyCoord(float) => (true, *float),
                    Syntax::Integer(int) => (false, *int as f32),
                    Syntax::Float(float) => (false, *float),
                    _ => return Err(format!("Tp requires a list of 3 coordinates; got `{c:?}`")),
                };
                Self::Linear(a, af, b, bf, c, cf)
            },
        )
    }
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
