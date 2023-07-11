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
    let mut recipe_buf = Vec::new();
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
                if let Syntax::Array(arr) = value {
                    recipe_buf.extend(arr.iter().map(recipe).collect::<SResult<Vec<_>>>()?);
                } else {
                    recipe_buf.push(recipe(value)?);
                }
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
    if let [recipe] = &recipe_buf[..] {
        state
            .recipes
            .insert(item.name.clone(), (recipe.to_json(), item.name.clone()));
    } else {
        for recipe in recipe_buf {
            state.recipes.insert(
                format!("{}_{:x}", item.name, get_hash(&recipe)).into(),
                (recipe.to_json(), item.name.clone()),
            );
        }
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

// given a syntax element within the item macro, crate the nbt contents of the recipe file
fn recipe(value: &Syntax) -> SResult<Nbt> {
    match value {
        Syntax::Object(obj) => {
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
            Ok(nbt!({
                type: "minecraft:crafting_shaped",
                pattern: pattern,
                key: new_key,
                result: nbt!({
                    item: "minecraft:knowledge_book",
                    count: 1
                })
            }))
        }
        Syntax::Macro(ident, inner) => match ident.as_ref() {
            "crafting_shaped" | "shaped" => {
                let inner @ Syntax::Object(_) = &**inner else {
                    return Err(format!("Expected recipe object; got `{inner:?}`"))
                };
                recipe(inner)
            }
            "crafting_shapeless" | "shapeless" => {
                let Syntax::Array(arr) = &**inner else {
                    return Err(format!("Expected an array for shapeless recipe; got `{inner:?}`"))
                };
                let mut arr_buf = Vec::new();
                for syn in arr.iter() {
                    match syn {
                        Syntax::String(ident) | Syntax::Identifier(ident) => arr_buf.push(
                            nbt!({
                                item: ident
                            })
                        ),
                        Syntax::Array(arr) => arr_buf.push(Nbt::Array(arr.iter().map(|syn| match syn {
                            Syntax::String(ident) | Syntax::Identifier(ident) => Ok(nbt!({item: ident})),
                            _ => Err(format!("Expected an item name for shapeless recipe element; got `{syn:?}`")),
                        }).collect::<SResult<Vec<_>>>()?)),
                        _ => return Err(format!("Expected a list or item name for shapeless recipe element; got `{syn:?}`"))
                    }
                }
                Ok(nbt!({
                    type: "minecraft:crafting_shapeless",
                    ingredients: arr_buf,
                    result: nbt!({
                        item: "minecraft:knowledge_book",
                        count: 1
                    })
                }))
            }
            "stonecutting" | "stonecutter" => {
                let id = String::try_from(&**inner)?;
                Ok(nbt!({
                    type: "minecraft:stonecutting",
                    ingredient: nbt!({
                        item: id
                    }),
                    result: "minecraft:knowledge_book",
                    count: 1
                }))
            }
            "smithing" | "smithing_table" => {
                let Syntax::Array(arr) = &**inner else {

                    return Err(format!(
                        "Smithing recipe expected `[base, addition]`; got `{inner:?}`"
                    ))
                };
                let [Syntax::String(base) | Syntax::Identifier(base), Syntax::String(addition) | Syntax::Identifier(addition)] = &arr[..] else {
                    return Err(format!("Smithing recipe expected `[base, addition]`; got `{arr:?}`"))
                };
                Ok(nbt!({
                    type: "minecraft:smithing",
                    base: nbt!({
                        item: base
                    }),
                    addition: nbt!({
                        item: addition
                    }),
                    template: Nbt::default()
                }))
            }
            other => Err(format!("Unexpected recipe macro: `{other}`")),
        },
        _ => Err(format!("Expected recipe object; got {value:?}")),
    }
}
