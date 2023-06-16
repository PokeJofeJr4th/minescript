use std::collections::BTreeMap;

use crate::{
    command::{Command, Nbt, Selector, SelectorType},
    nbt,
    parser::Syntax,
};

#[derive(Debug)]
pub struct Item {
    pub name: String,
    pub base: String,
    pub nbt: Nbt,
    pub on_consume: Option<String>,
    pub on_use: Option<String>,
}

#[derive(Debug)]
pub struct InterpreterState {
    pub items: Vec<Item>,
    pub functions: Vec<(String, Vec<Command>)>,
    pub recipes: BTreeMap<String, String>,
}

impl InterpreterState {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            functions: Vec::new(),
            recipes: BTreeMap::new(),
        }
    }
}

pub fn interpret(src: &Syntax) -> Result<InterpreterState, String> {
    let mut state = InterpreterState::new();
    inner_interpret(src, &mut state)?;
    Ok(state)
}

fn inner_interpret(src: &Syntax, state: &mut InterpreterState) -> Result<Vec<Command>, String> {
    match src {
        Syntax::Array(statements) => {
            let mut commands_buf = Vec::new();
            for statement in statements {
                commands_buf.extend(inner_interpret(statement, state)?);
            }
            return Ok(commands_buf);
        }
        Syntax::Macro(name, properties) => match name.as_ref() {
            "item" => {
                let item = interpret_item(properties, state)?;
                state.items.push(item);
            }
            "effect" => {
                return interpret_effect(properties);
            }
            "function" => {
                return Ok(vec![Command::Function {
                    func: String::try_from(&**properties)
                        .map_err(|_| String::from("Function macro should have a string"))?,
                }])
            }
            other => return Err(format!("Unexpected macro invocation `{other}`")),
        },
        Syntax::Function(func, content) => {
            let inner = inner_interpret(content, state)?;
            state.functions.push((func.clone(), inner))
        }
        Syntax::Identifier(_) => todo!(),
        Syntax::Unit => {}
        other => return Err(format!("Unexpected item `{other:?}`")),
    }
    Ok(Vec::new())
}

fn interpret_item(src: &Syntax, state: &mut InterpreterState) -> Result<Item, String> {
    let Syntax::Object(src) = src else {
        return Err(format!("Expected an object for item macro; got `{src:?}`"))
    };
    let mut item = Item {
        name: String::new(),
        base: String::new(),
        nbt: Nbt::default(),
        on_consume: None,
        on_use: None,
    };
    let mut on_consume_buf = Vec::new();
    let mut on_use_buf = Vec::new();
    let mut recipe_buf = None;
    for (prop, value) in src.iter() {
        match prop.as_ref() {
            "name" => {
                let Ok(name) = String::try_from(value) else {
                    return Err(String::from("Item name must be a string"))
                };
                item.name = name;
            }
            "base" => {
                let Ok(name) = String::try_from(value) else {
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
                Syntax::String(str) => item.on_consume = Some(str.clone()),
                Syntax::Function(name, body) => {
                    let new_body = inner_interpret(body, state)?;
                    state.functions.push((name.clone(), new_body));
                    item.on_consume = Some(name.clone());
                }
                other => {
                    on_consume_buf = inner_interpret(other, state)?;
                }
            },
            "on_use" => match value {
                Syntax::String(str) => item.on_use = Some(str.clone()),
                Syntax::Function(name, body) => {
                    let new_body = inner_interpret(body, state)?;
                    state.functions.push((name.clone(), new_body));
                    item.on_use = Some(name.clone());
                }
                other => {
                    on_use_buf = inner_interpret(other, state)?;
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
                        .collect::<Result<BTreeMap<String, Nbt>, String>>()?,
                );
                recipe_buf = Some(nbt!({
                    type: "minecraft:crafting_shaped",
                    pattern: pattern,
                    key: new_key,
                    result: nbt!({
                        item: "minecraft:knowledge_book",
                        count: 1
                    })
                }))
            }
            other => return Err(format!("Unexpected item property: `{other}`")),
        }
    }
    if !on_consume_buf.is_empty() {
        let func_name = format!("consume/{}", item.name);
        state.functions.push((func_name.clone(), on_consume_buf));
        item.on_consume = Some(func_name);
    }
    if !on_use_buf.is_empty() {
        let func_name = format!("use/{}", item.name);
        state.functions.push((func_name.clone(), on_use_buf));
        item.on_use = Some(func_name);
    }
    if let Some(recipe) = recipe_buf {
        state.recipes.insert(item.name.clone(), recipe.to_json());
    }
    if item.base.is_empty() {
        Err(String::from(
            "Item must have a specified base item; @item {... base: \"minecraft:potion\"}",
        ))
    } else if item.name.is_empty() {
        Err(String::from(
            "Item must have a specified name: @item {... name: \"My Item\"}",
        ))
    } else {
        Ok(item)
    }
}

fn interpret_effect(src: &Syntax) -> Result<Vec<Command>, String> {
    let Syntax::Object(src) = src else {
        return Err(format!("Expected an object for item macro; got `{src:?}`"))
    };
    let mut selector: Option<Selector> = None;
    let mut effect = None;
    let mut duration = None;
    let mut level = None;

    for (prop, value) in src.iter() {
        match prop.as_ref() {
            "selector" => match value {
                Syntax::Selector(seltype, contents) => {
                    selector = Some(Selector {
                        selector_type: *seltype,
                        args: contents
                            .iter()
                            .map(|(k, v)| String::try_from(v).map(|v| (k.clone(), v)))
                            .collect::<Result<BTreeMap<String, String>, _>>()
                            .map_err(|_| String::from("Couldn't convert to string in selector"))?,
                    })
                }
                other => {
                    return Err(format!(
                        "Unexpected element: `{other:?}`; expected a selector"
                    ))
                }
            },
            "effect" => {
                let Ok(eff) = String::try_from(value) else {
                    return Err(String::from("Potion effect must be a string"))
                };
                effect = Some(eff);
            }
            "duration" => match value {
                Syntax::Identifier(str) | Syntax::String(str) => {
                    if str != "infinite" {
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
                Syntax::Integer(num) => level = Some(*num),
                other => {
                    return Err(format!(
                        "Potion level should be an integer, not `{other:?}`"
                    ))
                }
            },
            other => return Err(format!("Unexpected potion property: `{other}`")),
        }
    }

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
