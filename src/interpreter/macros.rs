use std::{collections::BTreeMap, fs, path::Path};

use super::{inner_interpret, InterRepr, Item};
use crate::{lexer::tokenize, parser::parse, types::prelude::*};

pub(super) fn macros(
    name: &str,
    properties: &Syntax,
    state: &mut InterRepr,
    path: &Path,
) -> SResult<Vec<Command>> {
    match name {
        "effect" => {
            return effect(properties);
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
            return inner_interpret(&syntax, state, &new_path);
        }
        "item" => {
            // can't borrow state as mutable more than once at a time
            let item = item(properties, state, path)?;
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
            return raycast(properties, state, path);
        }
        "sound" | "playsound" => return sound(properties),
        other => return Err(format!("Unexpected macro invocation `{other}`")),
    }
    Ok(Vec::new())
}

fn effect(src: &Syntax) -> SResult<Vec<Command>> {
    let mut selector: Option<Selector<String>> = None;
    let mut effect = None;
    let mut duration = None;
    let mut level = 1;
    if let Syntax::Object(src) = src {
        for (prop, value) in src.iter() {
            match prop.as_ref() {
                "selector" | "target" => match value {
                    Syntax::Selector(sel) => {
                        selector = Some(sel.stringify()?);
                    }
                    other => {
                        return Err(format!(
                            "Unexpected element: `{other:?}`; expected a selector"
                        ))
                    }
                },
                "effect" => {
                    let Ok(eff) = RStr::try_from(value) else {
                        return Err(String::from("Potion effect must be a string"))
                    };
                    effect = Some(eff);
                }
                "duration" => match value {
                    Syntax::Identifier(str) | Syntax::String(str) => {
                        if *str != "infinite".into() {
                            return Err(format!(
                                "Potion duration should be an integer or infinite, not `{str}`"
                            ));
                        }
                    }
                    Syntax::Integer(num) => duration = Some(*num),
                    other => {
                        return Err(format!(
                            "Potion duration should be an integer or infinite, not `{other:?}`"
                        ))
                    }
                },
                "level" => match value {
                    Syntax::Integer(num) => level = *num,
                    other => {
                        return Err(format!(
                            "Potion level should be an integer, not `{other:?}`"
                        ))
                    }
                },
                other => return Err(format!("Unexpected potion property: `{other}`")),
            }
        }
    } else if let Ok(str) = RStr::try_from(src) {
        effect = Some(str);
    } else {
        return Err(format!("Expected an object for item macro; got `{src:?}`"));
    };

    let Some(effect) = effect else {
        return Err(String::from("Effect must include the effect id; {... effect: \"...\"}"))
    };

    Ok(vec![Command::EffectGive {
        target: selector.unwrap_or(Selector {
            selector_type: SelectorType::S,
            args: BTreeMap::new(),
        }),
        effect,
        duration,
        level,
    }])
}
#[allow(clippy::too_many_lines)]
fn item(src: &Syntax, state: &mut InterRepr, path: &Path) -> SResult<Item> {
    let Syntax::Object(src) = src else {
        return Err(format!("Expected an object for item macro; got `{src:?}`"))
    };
    let mut item = Item {
        name: String::new().into(),
        base: String::new().into(),
        nbt: Nbt::default(),
        on_consume: Vec::new(),
        on_use: Vec::new(),
        while_using: Vec::new(),
    };
    let mut recipe_buf = None;
    for (prop, value) in src.iter() {
        match prop.as_ref() {
            "name" => {
                let Ok(name) = RStr::try_from(value) else {
                    return Err(String::from("Item name must be a string"))
                };
                item.name = name;
            }
            "base" => {
                let Ok(name) = RStr::try_from(value) else {
                    return Err(String::from("Item base must be a string"))
                };
                item.base = name;
            }
            "nbt" => {
                let Ok(name) = Nbt::try_from(value) else {
                    return Err(String::from("Item nbt must be nbt data"))
                };
                item.nbt = name;
            }
            "on_consume" => match value {
                Syntax::String(str) => item
                    .on_consume
                    .push(Command::Function { func: str.clone() }),
                Syntax::Function(name, body) => {
                    let new_body = inner_interpret(body, state, path)?;
                    state.functions.push((name.clone(), new_body));
                    item.on_consume
                        .push(Command::Function { func: name.clone() });
                }
                other => {
                    item.on_consume.extend(inner_interpret(other, state, path)?);
                }
            },
            "on_use" => match value {
                Syntax::String(str) => item.on_use.push(Command::Function { func: str.clone() }),
                Syntax::Function(name, body) => {
                    let new_body = inner_interpret(body, state, path)?;
                    state.functions.push((name.clone(), new_body));
                    item.on_use.push(Command::Function { func: name.clone() });
                }
                other => item.on_use.extend(inner_interpret(other, state, path)?),
            },
            "while_using" => match value {
                Syntax::String(str) => item
                    .while_using
                    .push(Command::Function { func: str.clone() }),
                Syntax::Function(name, body) => {
                    let new_body = inner_interpret(body, state, path)?;
                    state.functions.push((name.clone(), new_body));
                    item.while_using
                        .push(Command::Function { func: name.clone() });
                }
                other => {
                    item.while_using
                        .extend(inner_interpret(other, state, path)?);
                }
            },
            "recipe" => {
                let Syntax::Object(obj) = value else {
                    return Err(format!("Expected recipe object; got {value:?}"))
                };
                let Some(pattern) = obj.get("pattern") else {
                    return Err(String::from("Expected pattern for recipe"))
                };
                let pattern = Nbt::try_from(pattern.clone())?;
                let Some(Syntax::Object(key)) = obj.get("key") else {
                    return Err(String::from("Expected key for recipe"))
                };
                let new_key = Nbt::from(
                    key.iter()
                        .map(|(k, v)| {
                            String::try_from(v)
                                .map(|str| (k.clone(), nbt!({ item: str })))
                                .map_err(|_| String::from("Expected string for item"))
                        })
                        .collect::<Result<BTreeMap<RStr, Nbt>, String>>()?,
                );
                recipe_buf = Some(nbt!({
                    type: "minecraft:crafting_shaped",
                    pattern: pattern,
                    key: new_key,
                    result: nbt!({
                        item: "minecraft:knowledge_book",
                        count: 1
                    })
                }));
            }
            other => return Err(format!("Unexpected item property: `{other}`")),
        }
    }
    if !item.on_use.is_empty() {
        state.objectives.insert(
            format!("use_{}", item.base).into(),
            format!("minecraft.used:minecraft.{}", item.base).into(),
        );
    }
    if let Some(recipe) = recipe_buf {
        state.recipes.insert(item.name.clone(), recipe.to_json());
    }
    if item.base.is_empty() {
        Err(String::from(
            "Item must have a specified base item; @item {... base: \"potion\"}",
        ))
    } else if item.name.is_empty() {
        Err(String::from(
            "Item must have a specified name: @item {... name: \"My Item\"}",
        ))
    } else {
        Ok(item)
    }
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
fn raycast(properties: &Syntax, state: &mut InterRepr, path: &Path) -> SResult<Vec<Command>> {
    let Syntax::Object(obj) = properties else {
        return Err(format!("Raycast macro expects an object, not {properties:?}"))
    };
    let mut max = 0;
    let mut step = 0.0;
    let mut callback = Vec::new();
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
                Syntax::String(str) => callback = vec![Command::Function { func: str.clone() }],
                Syntax::Function(name, body) => {
                    let new_body = inner_interpret(body, state, path)?;
                    state.functions.push((name.clone(), new_body));
                    callback = vec![Command::Function { func: name.clone() }];
                }
                other => callback = inner_interpret(other, state, path)?,
            },
            other => return Err(format!("Invalid key for Raycast macro: `{other}`")),
        }
    }
    let hash = get_hash(properties);

    if callback.is_empty() {
        return Err(String::from("Raycast requires a callback function"));
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
            format!("callback_{hash:x}"),
            state,
        ),
        // kill @s
        Command::Kill {
            target: Selector::s(),
        },
    ];
    state.functions.push((closure_name.clone(), closure_fn));

    let loop_fn = vec![
        // tp @s ^ ^ ^1
        Command::Teleport {
            target: Selector::s(),
            destination: Coordinate::Angular(0.0, 0.0, step),
        },
        Command::Raw("particle minecraft:campfire_signal_smoke".into()),
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
    ];
    state.functions.push((loop_name, loop_fn));

    Ok(vec![Command::Execute {
        options: vec![ExecuteOption::Summon {
            ident: "marker".into(),
        }],
        cmd: Box::new(Command::Function { func: closure_name }),
    }])
}
