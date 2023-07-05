use std::fmt::Display;

use super::prelude::*;

/// One Minecraft command
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// A user-made command that passes through the compiler unchanged
    Raw(RStr),
    /// A tellraw command
    TellRaw(Selector<String>, RStr),
    /// give a target an effect. Duration defaults to infinite, level defaults to 1
    EffectGive {
        target: Selector<String>,
        effect: RStr,
        duration: Option<i32>,
        level: i32,
    },
    /// kill the target
    Kill { target: Selector<String> },
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
    XpAdd {
        target: Selector<String>,
        amount: i32,
        levels: bool,
    },
    XpSet {
        target: Selector<String>,
        amount: i32,
        levels: bool,
    },
    XpGet {
        target: Selector<String>,
        levels: bool,
    },
    DataMergeFrom {
        target_type: RStr,
        target: RStr,
        target_path: NbtPath,
        src_type: RStr,
        src: RStr,
        src_path: NbtPath,
    },
    DataMergeValue {
        target_type: RStr,
        target: RStr,
        target_path: NbtPath,
        value: RStr,
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
    /// teleport the selected entity(ies) to another selected entity
    TeleportTo {
        target: Selector<String>,
        destination: Selector<String>,
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
            Self::TellRaw(sel, raw) => format!("tellraw {sel} {raw}"),
            Self::EffectGive {
                target,
                effect,
                duration,
                level,
            } => {
                format!(
                    "effect give {target} {effect} {} {level}",
                    duration.map_or_else(|| String::from("infinite"), |num| format!("{num}"))
                )
            }
            Self::Kill { target } => format!("kill {target}"),
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
            Self::TeleportTo { target, destination } => format!("tp {target} {destination}"),
            Self::Sound { sound, source, target, pos, volume, pitch, min_volume } => format!("playsound {sound} {source} {target} {pos} {volume} {pitch} {min_volume}"),
            Self::Damage { target, amount, damage_type, attacker } => format!("damage {target} {amount} {damage_type} by {attacker}"),
            Self::XpAdd { target, amount, levels } => format!("xp add {target} {amount} {}", if *levels { "levels" } else {"points"}),
            Self::XpSet { target, amount, levels } => format!("xp set {target} {amount} {}", if *levels { "levels"} else {"points"}),
            Self::XpGet { target, levels } => format!("xp query {target} {}", if *levels { "levels"} else {"points"}),
            Self::DataMergeFrom { target_type, target, target_path, src_type, src, src_path } => format!("data modify {target_type} {target} {} merge from {src_type} {src} {}", fmt_nbt_path(target_path), fmt_nbt_path(src_path)),
            Self::DataMergeValue { target_type, target, target_path, value } => format!("data modify {target_type} {target} {} merge value {value}", fmt_nbt_path(target_path))
        }
    }

    /// Create an Execute command that runs the specified other command.
    ///
    /// If the other command is an execute, it telescopes their options into one.
    /// If there are no execute subcommands, it returns the given command.
    /// If there is more than one given command, it returns a function call
    pub fn execute(
        mut options: Vec<ExecuteOption>,
        cmd: Vec<Self>,
        hash: &str,
        state: &mut InterRepr,
    ) -> Self {
        match &cmd[..] {
            [Self::Execute {
                options: inner_options,
                cmd,
            }] => {
                options.extend(inner_options.clone());
                Self::Execute {
                    options,
                    cmd: cmd.clone(),
                }
            }
            [cmd] => {
                if options.is_empty() {
                    cmd.clone()
                } else {
                    Self::Execute {
                        options,
                        cmd: Box::new(cmd.clone()),
                    }
                }
            }
            _ => {
                let func_name: RStr = format!("closure/{hash}").into();
                state.functions.push((func_name.clone(), cmd));
                let func = Self::Function { func: func_name };
                if options.is_empty() {
                    func
                } else {
                    Self::Execute {
                        options,
                        cmd: Box::new(func),
                    }
                }
            }
        }
    }
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
    /// if an entity exists
    Entity {
        invert: bool,
        selector: Selector<String>,
    },
    /// store a result in a score
    StoreScore { target: RStr, objective: RStr },
    /// change who `@s` is
    As { selector: Selector<String> },
    /// change where the command executes
    At { selector: Selector<String> },
    // /// get rotation from an entity
    // Rotated { selector: Selector<String> },
    /// anchored eyes|feet
    Anchored { ident: RStr },
    /// facing an entity
    FacingEntity { selector: Selector<String> },
    /// Block matches id or tag
    Block {
        invert: bool,
        pos: Coordinate,
        value: RStr,
    },
    /// Change `@s` to an entity with a certain relationship to current `@s`
    On { ident: RStr },
    /// summon an entity of type `ident` and set it to `@s`
    Summon { ident: RStr },
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
            Self::Entity { invert, selector } => format!(
                "{} entity {selector}",
                if *invert { "unless" } else { "if" }
            ),
            Self::StoreScore { target, objective } => {
                format!("store result score {target} {objective}")
            }
            Self::Block { invert, pos, value } => format!(
                "{} block {pos} {value}",
                if *invert { "unless" } else { "if" }
            ),
            Self::As { selector } => format!("as {selector}"),
            Self::At { selector } => format!("at {selector}"),
            // Self::Rotated { selector } => format!("rotated as {selector}"),
            Self::FacingEntity { selector } => format!("facing {selector}"),
            Self::Anchored { ident } => format!("anchored {ident}"),
            Self::On { ident } => format!("on {ident}"),
            Self::Summon { ident } => format!("summon {ident}"),
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
