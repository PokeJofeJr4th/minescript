use std::{fmt::Display, hash::Hash};

use super::{nbt::NbtLocation, prelude::*};

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
    Kill(Selector<String>),
    /// call a function
    Function(RStr),
    /// schedule a function to execute at a later time
    Schedule {
        func: RStr,
        time: i32,
        replace: bool,
    },
    /// set a score to a value
    ScoreSet {
        target: RStr,
        objective: RStr,
        value: i32,
    },
    /// get a score
    ScoreGet { target: RStr, objective: RStr },
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
    /// add to a target's XP points / levels
    XpAdd {
        target: Selector<String>,
        amount: i32,
        levels: bool,
    },
    /// set a target's XP points / levels
    XpSet {
        target: Selector<String>,
        amount: i32,
        levels: bool,
    },
    /// get a player's XP points / levels
    XpGet {
        target: Selector<String>,
        levels: bool,
    },
    /// transfer NBT data
    DataSetFrom {
        target: NbtLocation,
        src: NbtLocation,
    },
    /// get NBT data
    DataGet(NbtLocation),
    /// set NBT data to a constant
    DataSetValue { target: NbtLocation, value: RStr },
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
    /// play a sound
    Sound {
        sound: RStr,
        source: RStr,
        target: Selector<String>,
        pos: Coordinate,
        volume: f32,
        pitch: f32,
        min_volume: f32,
    },
    /// damage the target
    Damage {
        target: Selector<String>,
        amount: i32,
        damage_type: RStr,
        attacker: Selector<String>,
    },
}

impl Hash for Command {
    #[allow(clippy::too_many_lines)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Raw(str) | Self::Function(str) => str.hash(state),
            Self::TellRaw(sel, content) => (sel, content).hash(state),
            Self::EffectGive {
                target,
                effect,
                duration,
                level,
            } => (target, effect, duration, level).hash(state),
            Self::Kill(target) => target.hash(state),
            Self::Schedule {
                func,
                time,
                replace,
            } => (func, time, replace).hash(state),
            Self::ScoreSet {
                target,
                objective,
                value,
            }
            | Self::ScoreAdd {
                target,
                objective,
                value,
            } => (target, objective, value).hash(state),
            Self::ScoreGet { target, objective } => (target, objective).hash(state),
            Self::ScoreOperation {
                target,
                target_objective,
                operation,
                source,
                source_objective,
            } => (
                target,
                target_objective,
                operation,
                source,
                source_objective,
            )
                .hash(state),
            Self::XpAdd {
                target,
                amount,
                levels,
            }
            | Self::XpSet {
                target,
                amount,
                levels,
            } => (target, amount, levels).hash(state),
            Self::XpGet { target, levels } => (target, levels).hash(state),
            Self::DataSetFrom { target, src } => (target, src).hash(state),
            Self::DataGet(target) => target.hash(state),
            Self::DataSetValue { target, value } => (target, value).hash(state),
            Self::Execute { options, cmd } => (options, cmd).hash(state),
            Self::Teleport {
                target,
                destination,
            } => (target, destination).hash(state),
            Self::TeleportTo {
                target,
                destination,
            } => (target, destination).hash(state),
            Self::Sound {
                sound,
                source,
                target,
                pos,
                volume,
                pitch,
                min_volume,
            } => (
                sound,
                source,
                target,
                pos,
                volume.to_bits(),
                pitch.to_bits(),
                min_volume.to_bits(),
            )
                .hash(state),
            Self::Damage {
                target,
                amount,
                damage_type,
                attacker,
            } => (target, amount, damage_type, attacker).hash(state),
        }
    }
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
            Self::Kill (target) => format!("kill {target}"),
            Self::Function (func) => format!("function {namespace}:{}", fmt_mc_ident(func)),
            Self::Schedule { func, time, replace } => format!("schedule function {func} {time} {}", if *replace { "replace" } else { "append" }),
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
            } => format!("scoreboard players {} {player} {score} {}", if value.is_negative() { "remove" } else { "add" }, value.abs()),
            Self::ScoreGet { target, objective } => format!("scoreboard players get {target} {objective}"),
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
            Self::DataSetFrom { target, src } => format!("data modify {} set from {}", target.stringify(namespace), src.stringify(namespace)),
            Self::DataSetValue { target, value } => format!("data modify {} set value {value}", target.stringify(namespace)),
            Self::DataGet (target) => format!("data get {}", target.stringify(namespace))
        }
    }

    /// Create an Execute command that runs the specified other command.
    ///
    /// If the other command is an execute, it telescopes their options into one.
    /// If there are no execute subcommands, it returns the given command.
    /// If there is more than one given command, it returns a function call and inserts the function into the state
    pub fn execute(
        options: &[ExecuteOption],
        cmd: VecCmd,
        hash: &str,
        state: &mut InterRepr,
    ) -> Versioned<Self> {
        let (output, inner) = cmd
            .map(|cmd| {
                let mut inner = Vec::new();
                let output = match &cmd[..] {
                    [Self::Execute {
                        options: inner_options,
                        cmd,
                    }] => {
                        let mut opts = options.to_vec();
                        opts.extend(inner_options.clone());
                        Self::Execute {
                            options: opts,
                            cmd: cmd.clone(),
                        }
                    }
                    [cmd] => {
                        if options.is_empty() {
                            cmd.clone()
                        } else {
                            Self::Execute {
                                options: options.to_vec(),
                                cmd: Box::new(cmd.clone()),
                            }
                        }
                    }
                    _ => {
                        let func_name: RStr = hash.into();
                        // TODO: This does not work. It will overshadow different versions because this fn has historically been called
                        // once per version. I should probably make a map internally or something but that seems really hard.
                        inner = cmd.clone();
                        let func = Self::Function(func_name);
                        if options.is_empty() {
                            func
                        } else {
                            Self::Execute {
                                options: options.to_vec(),
                                cmd: Box::new(func),
                            }
                        }
                    }
                };
                (output, inner)
            })
            .unzip();
        if !inner.is_empty() {
            state.functions.insert(hash.into(), inner);
        }
        output
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Coordinate {
    /// coordinate given with xyz coordinates; booleans are for `~ ~ ~`
    Linear(bool, f32, bool, f32, bool, f32),
    /// coordinates given by angle; `^ ^ ^`
    Angular(f32, f32, f32),
}

impl Hash for Coordinate {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Linear(bx, fx, by, fy, bz, fz) => {
                (bx, by, bz, fy.to_bits(), fx.to_bits(), fz.to_bits()).hash(state);
            }
            Self::Angular(x, y, z) => (x.to_bits(), y.to_bits(), z.to_bits()).hash(state),
        }
    }
}

impl Coordinate {
    pub const fn here() -> Self {
        Self::Linear(true, 0.0, true, 0.0, true, 0.0)
    }

    // pub const fn absolute(x: f32, y: f32, z: f32) -> Self {
    //     Self::Linear(false, x, false, y, false, z)
    // }
}

impl TryFrom<&Syntax> for Coordinate {
    type Error = String;

    fn try_from(body: &Syntax) -> SResult<Self> {
        let Syntax::Array(arr) = body else {
            return Err(format!("Expected a list of 3 coordinates; got `{body:?}`"))
        };
        let [a, b, c] = &arr[..] else {
            return Err(format!("Expected a list of 3 coordinates; got `{body:?}`"))
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
                    _ => return Err(format!("Expected a list of 3 coordinates; got `{a:?}`")),
                };
                let (b, bf) = match b {
                    Syntax::WooglyCoord(float) => (true, *float),
                    Syntax::Integer(int) => (false, *int as f32),
                    Syntax::Float(float) => (false, *float),
                    _ => return Err(format!("Expected a list of 3 coordinates; got `{b:?}`")),
                };
                let (c, cf) = match c {
                    Syntax::WooglyCoord(float) => (true, *float),
                    Syntax::Integer(int) => (false, *int as f32),
                    Syntax::Float(float) => (false, *float),
                    _ => return Err(format!("Expected a list of 3 coordinates; got `{c:?}`")),
                };
                Self::Linear(a, af, b, bf, c, cf)
            },
        )
    }
}

impl Display for Coordinate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Linear(x_relative, x, y_relative, y, z_relative, z) => {
                write!(
                    f,
                    "{}{} {}{} {}{}",
                    if x_relative { "~" } else { "" },
                    if x_relative && x == 0.0 {
                        String::new()
                    } else {
                        x.to_string()
                    },
                    if y_relative { "~" } else { "" },
                    if y_relative && y == 0.0 {
                        String::new()
                    } else {
                        y.to_string()
                    },
                    if z_relative { "~" } else { "" },
                    if z_relative && z == 0.0 {
                        String::new()
                    } else {
                        z.to_string()
                    }
                )
            }
            Self::Angular(a, b, c) => {
                write!(
                    f,
                    "^{} ^{} ^{}",
                    if a == 0.0 {
                        String::new()
                    } else {
                        a.to_string()
                    },
                    if b == 0.0 {
                        String::new()
                    } else {
                        b.to_string()
                    },
                    if c == 0.0 {
                        String::new()
                    } else {
                        c.to_string()
                    }
                )
            }
        }
    }
}
