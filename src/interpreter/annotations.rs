use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use super::{get_data_location, inner_interpret};
use crate::{lexer::tokenize, parser::parse, types::prelude::*, Config};

mod effect;
mod item;

macro_rules! interpret_fn {
    ($fn_buf: ident $config: ident, $value: expr, $state: expr, $path: expr, $src_files: expr) => {
        match $value {
            Syntax::String(str) => $fn_buf.push(Command::Function (str.clone() ).into()),
            Syntax::Block(BlockType::Function, name, body) => {
                let (Syntax::Identifier(name) | Syntax::String(name)) = &**name else {
                                $fn_buf.extend(inner_interpret($value, $state, $path, $src_files, $config)?);
                                continue;
                            };
                let new_body = inner_interpret(body, $state, $path, $src_files, $config)?;
                $state.functions.insert(name.clone(), new_body);
                $fn_buf.push(Command::Function (name.clone() ).into());
            }
            other => $fn_buf.extend(inner_interpret(other, $state, $path, $src_files, $config)?),
        }
    };
}

pub(super) fn annotations(
    name: &str,
    properties: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
    config: &Config,
) -> SResult<VecCmd> {
    match name {
        "effect" => {
            return effect::effect(properties);
        }
        "function" => {
            let func = RStr::try_from(properties)
                .map_err(|e| format!("Function annotation should have a string; {e}"))?;
            return Ok(vec![Command::Function(func)].into());
        }
        "import" => {
            let Syntax::String(str) = properties else {
                return Err(format!("Import annotation expects a string, not `{properties:?}`"))
            };
            let new_path = path.join(str.as_ref());
            src_files.insert(new_path.clone());
            let text = fs::read_to_string(&new_path)
                .map_err(|err| format!("Error opening {str}: {err}"))?;
            let tokens = tokenize(&format!("[{text}]"))
                .map_err(|err| format!("Error parsing {str}: {err}"))?;
            let syntax = parse(tokens).map_err(|err| format!("Error parsing {str}: {err}"))?;
            return inner_interpret(&syntax, state, &new_path, src_files, config);
        }
        "item" => {
            // can't borrow state as mutable more than once at a time
            let item = item::item(properties, state, path, src_files, config)?;
            state.items.push(item);
        }
        "raw" => match properties {
            Syntax::String(cmd) => return Ok(vec![Command::Raw(cmd.clone())].into()),
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
                    .collect::<SResult<Vec<Command>>>()
                    .map(Into::into);
            }
            other => {
                return Err(format!(
                    "`@raw` takes a string or list of strings, not `{other:?}`"
                ))
            }
        },
        "raycast" => {
            return raycast(properties, state, path, src_files, config);
        }
        "sound" | "playsound" => return sound(properties),
        "random" | "rand" => {
            let Syntax::BinaryOp { lhs, operation: Operation::In, rhs } = properties else {
                return Err(format!("`@random` annotation takes `{{var}} in {{...}}`; got `{properties:?}`"))
            };
            let (mut commands, lhs) = get_data_location(lhs)?;
            commands.extend(random(
                lhs.stringify_scoreboard_target()?,
                lhs.stringify_scoreboard_objective(config)?,
                rhs,
                state,
            )?);
            return Ok(commands);
        }
        other => return Err(format!("Unexpected annotation `{other}`")),
    }
    Ok(VecCmd::default())
}

fn sound(properties: &Syntax) -> SResult<VecCmd> {
    let Syntax::Object(obj) = properties else {
        return Err(format!("Sound annotation expects an object, not {properties:?}"))
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
            other => return Err(format!("Invalid key for Sound annotation: `{other}`")),
        }
    }
    let Some(sound) = sound else {
                    return Err(String::from("Sound annotation must specify the sound to play"))
                };
    Ok(vec![Command::Sound {
        sound,
        source,
        target: target.stringify()?,
        pos,
        volume,
        pitch,
        min_volume,
    }]
    .into())
}

#[allow(clippy::too_many_lines)]
fn raycast(
    properties: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
    config: &Config,
) -> SResult<VecCmd> {
    let Syntax::Object(obj) = properties else {
        return Err(format!("Raycast annotation expects an object, not {properties:?}"))
    };
    let mut max = 0;
    let mut step = 0.0;
    let mut callback = VecCmd::default();
    let mut each = VecCmd::default();
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
            "callback" | "hit" => interpret_fn!(callback config, v, state, path, src_files),
            "each" => interpret_fn!(each config, v, state, path, src_files),
            other => return Err(format!("Invalid key for Raycast annotation: `{other}`")),
        }
    }
    let hash = get_hash(properties);

    if callback.is_empty() {
        return Err(String::from("Raycast requires a hit/callback function"));
    };

    let hash: RStr = format!("{hash:x}").into();
    let score_name: RStr = format!("%__timer__{hash}").into();
    let closure_name: RStr = format!("__internal__/{hash}").into();
    let loop_name: RStr = format!("__internal__/loop_{hash}").into();

    let closure_fn = Command::execute(
        &[ExecuteOption::At(Selector::s())],
        callback,
        &format!("__internal__/callback_{hash}"),
        state,
    )
    .map(|cmds| {
        vec![
            Command::Raw("execute rotated as @p run tp @s ~ ~1.5 ~ ~ ~".into()),
            // scoreboard players reset %timer dummy
            Command::ScoreSet {
                target: score_name.clone(),
                objective: config.dummy_objective.clone(),
                value: 0,
            },
            // at @s function loop
            Command::Execute {
                options: vec![ExecuteOption::At(Selector::s())],
                cmd: Box::new(Command::Function(loop_name.clone())),
            },
            // at @s {callback}
            cmds,
            // kill @s
            Command::Kill(Selector::s()),
        ]
    });
    state.functions.insert(closure_name.clone(), closure_fn);

    each.extend(
        [
            // tp @s ^ ^ ^1
            Command::Teleport {
                target: Selector::s(),
                destination: Coordinate::Angular(0.0, 0.0, step),
            },
            // timer ++
            Command::ScoreAdd {
                target: score_name.clone(),
                objective: config.dummy_objective.clone(),
                value: 1,
            },
            // execute unless %timer < max at @s if block ~ ~ ~ air run loop
            Command::Execute {
                options: vec![
                    ExecuteOption::IfScoreMatches {
                        invert: false,
                        target: score_name,
                        objective: config.dummy_objective.clone(),
                        lower: None,
                        upper: Some(max),
                    },
                    ExecuteOption::At(Selector::s()),
                    ExecuteOption::IfBlock {
                        invert: false,
                        pos: Coordinate::here(),
                        value: "air".into(),
                    },
                ],
                cmd: Box::new(Command::Function(loop_name.clone())),
            },
        ]
        .into(),
    );
    state.functions.insert(loop_name, each);

    Ok(vec![Command::Execute {
        options: vec![ExecuteOption::Summon("marker".into())],
        cmd: Box::new(Command::Function(closure_name)),
    }]
    .into())
}

pub fn random(
    target_name: RStr,
    target_objective: RStr,
    properties: &Syntax,
    state: &mut InterRepr,
) -> SResult<VecCmd> {
    let (min, max) = match properties {
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
    let mut rng_cmd: Versioned<Command> =
        Command::Raw(format!("loot spawn 0 -256 0 loot <NAMESPACE>:{loot_table_name}").into())
            .into();
    rng_cmd.add_version(
        16,
        Command::Raw(format!("random value {min}..{max}").into()),
    );
    Ok(Command::execute(
        &[ExecuteOption::StoreScore {
            target: target_name,
            objective: target_objective,
            is_success: false,
        }],
        rng_cmd.into_vec(),
        "",
        state,
    )
    .map(|c| vec![c]))
}
