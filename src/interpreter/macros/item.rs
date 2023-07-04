use std::{collections::BTreeMap, path::Path};

use crate::{interpreter::inner_interpret, types::prelude::*};

#[allow(clippy::too_many_lines)]
pub(super) fn item(src: &Syntax, state: &mut InterRepr, path: &Path) -> SResult<Item> {
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
