use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use super::inner_interpret;
use crate::{lexer::tokenize, parser::parse, types::prelude::*};

mod effect;
mod item;

#[allow(clippy::too_many_lines)]
pub(super) fn macros(
    name: &str,
    properties: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
) -> SResult<Vec<Command>> {
    match name {
        "effect" => {
            return effect::effect(properties);
        }
        "function" => {
            let func = RStr::try_from(properties)
                .map_err(|e| format!("Function macro should have a string; {e}"))?;
            return Ok(vec![Command::Function { func }]);
        }
        "import" => {
            let Syntax::String(str) = properties else {
                return Err(format!("Import macro expects a string, not `{properties:?}`"))
            };
            let new_path = path.join(str.as_ref());
            let text = fs::read_to_string(&new_path)
                .map_err(|err| format!("Error opening {str}: {err}"))?;
            let tokens = tokenize(&format!("[{text}]"))
                .map_err(|err| format!("Error parsing {str}: {err}"))?;
            let syntax = parse(&mut tokens.into_iter().peekable())
                .map_err(|err| format!("Error parsing {str}: {err}"))?;
            return inner_interpret(&syntax, state, &new_path, src_files);
        }
        "item" => {
            // can't borrow state as mutable more than once at a time
            let item = item::item(properties, state, path, src_files)?;
            state.items.push(item);
        }
        "raw" => match properties {
            Syntax::String(cmd) => return Ok(vec![Command::Raw(cmd.clone())]),
            Syntax::Array(arr) => {
                return arr
                    .iter()
                    .map(|syn| {
                        if let Syntax::String(cmd) = syn {
                            Ok(Command::Raw(cmd.clone()))
                        } else {
                            Err(format!(
                                "`@raw` takes a string or list of strings, not `{syn:?}`"
                            ))
                        }
                    })
                    .collect::<SResult<Vec<Command>>>();
            }
            other => {
                return Err(format!(
                    "`@raw` takes a string or list of strings, not `{other:?}`"
                ))
            }
        },
        "raycast" => {
            return raycast(properties, state, path, src_files);
        }
        "sound" | "playsound" => return sound(properties),
        "random" | "rand" => {
            let Syntax::BinaryOp(lhs, Operation::In, properties) = properties else {
                return Err(format!("`@random` in statement position takes an argument of the form `var in 0..10` or `var in 5`; got `{properties:?}`"))
            };
            let (min, max) = match &**properties {
                Syntax::Integer(max) | Syntax::Range(None, Some(max)) => (0, *max),
                Syntax::Range(Some(min), Some(max)) => (*min, *max),
                _ => {
                    return Err(format!(
                        "`@random` in statement form takes an integer or bounded range; got `{properties:?}`"
                    ))
                }
            };
            let loot_table_name: RStr = format!("rng/{min}_{max}").into();
            // if the loot table doesn't exist, build it
            state
                .loot_tables
                .entry(loot_table_name.clone())
                .or_insert_with(|| {
                    nbt!({
                        pools: nbt!([nbt!({
                            rolls: nbt!({
                                min: min,
                                max: max
                            }),
                            entries: nbt!([
                                nbt!({
                                    type: "item",
                                    weight: 1,
                                    name: "minecraft:stone"
                                })
                            ])
                        })])
                    })
                    .to_json()
                    .into()
                });
            return Ok(vec![Command::execute(
                vec![ExecuteOption::StoreScore {
                    target: lhs.stringify_scoreboard_target()?,
                    objective: lhs.stringify_scoreboard_objective()?,
                }],
                vec![Command::Raw(
                    format!("loot spawn 0 -256 0 loot <NAMESPACE>:{loot_table_name}").into(),
                )],
                "",
                state,
            )]);
        }
        other => return Err(format!("Unexpected macro invocation `{other}`")),
    }
    Ok(Vec::new())
}

fn sound(properties: &Syntax) -> SResult<Vec<Command>> {
    let Syntax::Object(obj) = properties else {
        return Err(format!("Sound macro expects an object, not {properties:?}"))
    };
    let mut sound: Option<RStr> = None;
    let mut pos = Coordinate::here();
    let mut source: RStr = "master".into();
    let mut target: Selector<Syntax> = Selector::e();
    let mut volume = 1.0f32;
    let mut pitch = 1.0f32;
    let mut min_volume = 0.0f32;
    for (k, v) in obj {
        match &**k {
            "sound" => match RStr::try_from(v) {
                Ok(str) => sound = Some(str),
                Err(_) => {
                    return Err(format!(
                        "Expected a string or identifier for sound; got `{v:?}`"
                    ))
                }
            },
            "pos" | "posititon" | "location" => pos = Coordinate::try_from(v)?,
            "source" => match RStr::try_from(v) {
                Ok(str) => source = str,
                Err(_) => {
                    return Err(format!(
                        "Expected a string or identifier for sound source; got `{v:?}`"
                    ))
                }
            },
            "target" => {
                let Syntax::Selector(selector) = v else {
                    return Err(format!("Expected a selector for sound target; got `{v:?}`"))
                };
                target = selector.clone();
            }
            "volume" => match v {
                Syntax::Integer(int) => volume = *int as f32,
                Syntax::Float(float) => volume = *float,
                other => return Err(format!("Expected float or int for volume; got `{other:?}`")),
            },
            "pitch" => match v {
                Syntax::Integer(int) => pitch = *int as f32,
                Syntax::Float(float) => pitch = *float,
                other => return Err(format!("Expected float or int for pitch; got `{other:?}`")),
            },
            "minvolume" | "min_volume" => match v {
                Syntax::Integer(int) => min_volume = *int as f32,
                Syntax::Float(float) => min_volume = *float,
                other => {
                    return Err(format!(
                        "Expected float or int for min volume; got `{other:?}`"
                    ))
                }
            },
            other => return Err(format!("Invalid key for Sound macro: `{other}`")),
        }
    }
    let Some(sound) = sound else {
                    return Err(String::from("Sound macro must specify the sound to play"))
                };
    Ok(vec![Command::Sound {
        sound,
        source,
        target: target.stringify()?,
        pos,
        volume,
        pitch,
        min_volume,
    }])
}

#[allow(clippy::too_many_lines)]
fn raycast(
    properties: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
) -> SResult<Vec<Command>> {
    let Syntax::Object(obj) = properties else {
        return Err(format!("Raycast macro expects an object, not {properties:?}"))
    };
    let mut max = 0;
    let mut step = 0.0;
    let mut callback = Vec::new();
    let mut each = Vec::new();
    for (k, v) in obj {
        match &**k {
            "max" => {
                let Syntax::Integer(int) = v else {
                    return Err(format!("Expected integer for raycast max; got `{v:?}`"))
                };
                max = *int;
            }
            "step" | "amount" => {
                if let Syntax::Integer(int) = v {
                    step = *int as f32;
                } else if let Syntax::Float(float) = v {
                    step = *float;
                } else {
                    return Err(format!("Expected number as raycast step size; got `{v:?}`"));
                }
            }
            "callback" | "hit" => match v {
                Syntax::String(str) => callback.push(Command::Function { func: str.clone() }),
                Syntax::Block(BlockType::Function, name, body) => {
                    let (Syntax::Identifier(name) | Syntax::String(name)) = &**name else {
                        callback.extend(inner_interpret(v, state, path, src_files)?);
                        continue;
                    };
                    let new_body = inner_interpret(body, state, path, src_files)?;
                    state.functions.push((name.clone(), new_body));
                    callback.push(Command::Function { func: name.clone() });
                }
                other => callback.extend(inner_interpret(other, state, path, src_files)?),
            },
            "each" => match v {
                Syntax::String(str) => each.push(Command::Function { func: str.clone() }),
                Syntax::Block(BlockType::Function, name, body) => {
                    let (Syntax::Identifier(name) | Syntax::String(name)) = &**name else {
                        each.extend(inner_interpret(v, state, path, src_files)?);
                        continue;
                    };
                    let new_body = inner_interpret(body, state, path, src_files)?;
                    state.functions.push((name.clone(), new_body));
                    each.push(Command::Function { func: name.clone() });
                }
                other => each.extend(inner_interpret(other, state, path, src_files)?),
            },
            other => return Err(format!("Invalid key for Raycast macro: `{other}`")),
        }
    }
    let hash = get_hash(properties);

    if callback.is_empty() {
        return Err(String::from("Raycast requires a hit/callback function"));
    };

    let closure_name: RStr = format!("closure/{hash:x}").into();
    let loop_name: RStr = format!("closure/loop_{hash:x}").into();

    let closure_fn = vec![
        Command::Raw("execute at @s rotated as @p run tp @s ~ ~1.5 ~ ~ ~".into()),
        // scoreboard players reset %timer dummy
        Command::ScoreSet {
            target: "%timer".into(),
            objective: "dummy".into(),
            value: 0,
        },
        // at @s function loop
        Command::Execute {
            options: vec![ExecuteOption::At {
                selector: Selector::s(),
            }],
            cmd: Box::new(Command::Function {
                func: loop_name.clone(),
            }),
        },
        // at @s {callback}
        Command::execute(
            vec![ExecuteOption::At {
                selector: Selector::s(),
            }],
            callback,
            &format!("callback_{hash:x}"),
            state,
        ),
        // kill @s
        Command::Kill {
            target: Selector::s(),
        },
    ];
    state.functions.push((closure_name.clone(), closure_fn));

    each.extend([
        // tp @s ^ ^ ^1
        Command::Teleport {
            target: Selector::s(),
            destination: Coordinate::Angular(0.0, 0.0, step),
        },
        // timer ++
        Command::ScoreAdd {
            target: "%timer".into(),
            objective: "dummy".into(),
            value: 1,
        },
        // execute unless %timer < max at @s if block ~ ~ ~ air run loop
        Command::Execute {
            options: vec![
                ExecuteOption::ScoreMatches {
                    invert: false,
                    target: "%timer".into(),
                    objective: "dummy".into(),
                    lower: None,
                    upper: Some(max),
                },
                ExecuteOption::At {
                    selector: Selector::s(),
                },
                ExecuteOption::Block {
                    invert: false,
                    pos: Coordinate::here(),
                    value: "air".into(),
                },
            ],
            cmd: Box::new(Command::Function {
                func: loop_name.clone(),
            }),
        },
    ]);
    state.functions.push((loop_name, each));

    Ok(vec![Command::Execute {
        options: vec![ExecuteOption::Summon {
            ident: "marker".into(),
        }],
        cmd: Box::new(Command::Function { func: closure_name }),
    }])
}
