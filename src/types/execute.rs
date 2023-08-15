use std::hash::Hash;

use super::{Coordinate, NbtLocation, Operation, RStr, Selector};

#[derive(Debug, Clone, PartialEq)]
pub enum ExecuteOption {
    /// compare score to a static range
    IfScoreMatches {
        invert: bool,
        target: RStr,
        objective: RStr,
        lower: Option<i32>,
        upper: Option<i32>,
    },
    /// compare score to another score
    IfScoreSource {
        invert: bool,
        target: RStr,
        target_objective: RStr,
        operation: Operation,
        source: RStr,
        source_objective: RStr,
    },
    /// if an entity exists
    IfEntity {
        invert: bool,
        selector: Selector<String>,
    },
    /// store a result in a score
    StoreScore {
        target: RStr,
        objective: RStr,
        is_success: bool,
    },
    /// store a result in NBT
    StoreNBT {
        location: NbtLocation,
        is_success: bool,
    },
    /// change who `@s` is
    As(Selector<String>),
    /// change where the command executes
    At(Selector<String>),
    /// get rotation from an entity
    RotatedAs(Selector<String>),
    /// specific rotation
    Rotated {
        yaw_rel: bool,
        yaw: f32,
        pitch_rel: bool,
        pitch: f32,
    },
    /// choose a specific position
    Positioned(Coordinate),
    /// anchored eyes|feet
    Anchored(RStr),
    /// facing an entity
    FacingEntity(Selector<String>),
    /// facing a position
    FacingPos(Coordinate),
    /// Block matches id or tag
    IfBlock {
        invert: bool,
        pos: Coordinate,
        value: RStr,
    },
    /// Change `@s` to an entity with a certain relationship to current `@s`
    On(RStr),
    /// summon an entity of type `ident` and set it to `@s`
    Summon(RStr),
}

impl Hash for ExecuteOption {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::IfScoreMatches {
                invert,
                target,
                objective,
                lower,
                upper,
            } => (invert, target, objective, lower, upper).hash(state),
            Self::IfScoreSource {
                invert,
                target,
                target_objective,
                operation,
                source,
                source_objective,
            } => (
                invert,
                target,
                target_objective,
                operation,
                source,
                source_objective,
            )
                .hash(state),
            Self::IfEntity { invert, selector } => (invert, selector).hash(state),
            Self::StoreScore {
                target,
                objective,
                is_success,
            } => (target, objective, is_success).hash(state),
            Self::StoreNBT {
                location,
                is_success,
            } => (location, is_success).hash(state),
            Self::As(selector)
            | Self::At(selector)
            | Self::RotatedAs(selector)
            | Self::FacingEntity(selector) => selector.hash(state),
            Self::Rotated {
                yaw_rel,
                yaw,
                pitch_rel,
                pitch,
            } => (yaw_rel, yaw.to_bits(), pitch_rel, pitch.to_bits()).hash(state),
            Self::FacingPos(pos) | Self::Positioned(pos) => pos.hash(state),
            Self::IfBlock { invert, pos, value } => (invert, pos, value).hash(state),
            Self::Anchored(ident) | Self::On(ident) | Self::Summon(ident) => {
                ident.hash(state);
            }
        }
    }
}

impl ExecuteOption {
    pub fn stringify(&self, namespace: &str) -> String {
        match self {
            Self::IfScoreMatches {
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
            Self::IfScoreSource {
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
            Self::IfEntity { invert, selector } => format!(
                "{} entity {selector}",
                if *invert { "unless" } else { "if" }
            ),
            Self::StoreScore {
                target,
                objective,
                is_success,
            } => {
                format!(
                    "store {} score {target} {objective}",
                    if *is_success { "success" } else { "result" }
                )
            }
            Self::StoreNBT {
                location,
                is_success,
            } => {
                format!(
                    "store {} {} int 1",
                    if *is_success { "success" } else { "result" },
                    location.stringify(namespace)
                )
            }
            Self::IfBlock { invert, pos, value } => format!(
                "{} block {pos} {value}",
                if *invert { "unless" } else { "if" }
            ),
            Self::As(selector) => format!("as {selector}"),
            Self::At(selector) => format!("at {selector}"),
            Self::RotatedAs(selector) => format!("rotated as {selector}"),
            Self::Rotated {
                yaw_rel,
                yaw,
                pitch_rel,
                pitch,
            } => format!(
                "rotated {}{} {}{}",
                if *yaw_rel { "~" } else { " " },
                if *yaw == 0.0 && *yaw_rel {
                    String::new()
                } else {
                    yaw.to_string()
                },
                if *pitch_rel { "~" } else { " " },
                if *pitch == 0.0 && *pitch_rel {
                    String::new()
                } else {
                    pitch.to_string()
                },
            ),
            Self::Positioned(pos) => format!("positioned {pos}"),
            Self::FacingEntity(selector) => format!("facing entity {selector}"),
            Self::FacingPos(pos) => format!("facing {pos}"),
            Self::Anchored(ident) => format!("anchored {ident}"),
            Self::On(ident) => format!("on {ident}"),
            Self::Summon(ident) => format!("summon {ident}"),
        }
    }
}
