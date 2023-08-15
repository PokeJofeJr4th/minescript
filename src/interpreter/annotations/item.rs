use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use lazy_regex::lazy_regex;

use crate::{interpreter::inner_interpret, types::prelude::*, Config};

#[allow(clippy::too_many_lines)]
pub(super) fn item(
    src: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
    config: &Config,
) -> SResult<Item> {
    let Syntax::Object(src) = src else {
        return Err(format!("Expected an object for item annotation; got `{src:?}`"))
    };
    let mut item = Item::default();
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
                let Ok(obj @ Nbt::Object(_)) = Nbt::try_from(value) else {
                    return Err(format!("Item nbt must be nbt data; got `{value:?}`"))
                };
                item.nbt = obj;
            }
            "on_consume" => match value {
                Syntax::String(str) => item.on_consume.push(Command::Function(str.clone()).into()),
                Syntax::Block(BlockType::Function, name, body) => {
                    let (Syntax::Identifier(name) | Syntax::String(name)) = &**name else {
                        item.on_consume.extend(inner_interpret(value, state, path, src_files, config)?);
                        continue;
                    };
                    let new_body = inner_interpret(body, state, path, src_files, config)?;
                    state.functions.insert(name.clone(), new_body);
                    item.on_consume.push(Command::Function(name.clone()).into());
                }
                other => {
                    item.on_consume
                        .extend(inner_interpret(other, state, path, src_files, config)?);
                }
            },
            "on_use" => match value {
                Syntax::String(str) => item.on_use.push(Command::Function(str.clone()).into()),
                Syntax::Block(BlockType::Function, name, body) => {
                    let (Syntax::Identifier(name) | Syntax::String(name)) = &**name else {
                        item.on_use.extend(inner_interpret(value, state, path, src_files, config)?);
                        continue;
                    };
                    let new_body = inner_interpret(body, state, path, src_files, config)?;
                    state.functions.insert(name.clone(), new_body);
                    item.on_use.push(Command::Function(name.clone()).into());
                }
                other => item
                    .on_use
                    .extend(inner_interpret(other, state, path, src_files, config)?),
            },
            "while_using" => match value {
                Syntax::String(str) => item.while_using.push(Command::Function(str.clone()).into()),
                Syntax::Block(BlockType::Function, name, body) => {
                    let (Syntax::Identifier(name) | Syntax::String(name)) = &**name else {
                        item.while_using.extend(inner_interpret(value, state, path, src_files, config)?);
                        continue;
                    };
                    let new_body = inner_interpret(body, state, path, src_files, config)?;
                    state.functions.insert(name.clone(), new_body);
                    item.while_using
                        .push(Command::Function(name.clone()).into());
                }
                other => {
                    item.while_using
                        .extend(inner_interpret(other, state, path, src_files, config)?);
                }
            },
            "recipe" => {
                if let Syntax::Array(arr) = value {
                    recipe_buf.extend(arr.iter().map(recipe).collect::<SResult<Vec<_>>>()?);
                } else {
                    recipe_buf.push(recipe(value)?);
                }
            }
            "while_slot" => {
                let Syntax::Object(obj) = value else {
                    return Err(format!("Expected an object for `while_slot` property; got `{value:?}`"))
                };
                for (k, v) in obj.iter() {
                    // SLOT INFO
                    // 9  10 11 12 13 14 15 16 17
                    // 18 19 20 21 22 23 24 25 26
                    // 27 28 29 30 31 12 33 34 35
                    // 0  1  2  3  4  5  6  7  8
                    // -106
                    let slot = if let Some(captures) =
                        lazy_regex!("^slot_(?P<slot>[0-9]+)$").captures(k)
                    {
                        // given the regex above, `captures.name` can never fail
                        captures
                            .name("slot")
                            .unwrap()
                            .as_str()
                            .parse()
                            .map_err(|err| format!("While checking if item is in {k}: {err}"))?
                    } else if let Some(captures) =
                        lazy_regex!("^hotbar_(?P<slot>[0-8])$").captures(k)
                    {
                        // given the regex above, both `captures.name` and `parse::<i32>` can never fail
                        captures
                            .name("slot")
                            .unwrap()
                            .as_str()
                            .parse::<i32>()
                            .unwrap()
                    } else {
                        match &**k {
                            // "mainhand" => 98,
                            "offhand" => -106,
                            "head" => 103,
                            "chest" => 102,
                            "legs" => 101,
                            "feet" => 100,
                            _ => return Err(format!("Unexpected slot: `{k}`")),
                        }
                    };
                    item.slot_checks
                        .push((slot, inner_interpret(v, state, path, src_files, config)?));
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
    let Nbt::Object(ref mut obj) = item.nbt else {
        return Err(format!("Item nbt should be an object; got `{}`", item.nbt))
    };
    match obj.get_mut("tag") {
        Some(Nbt::Object(ref mut inner)) => {
            inner.insert("_is_minescript".into(), item.name.clone().into());
        }
        Some(other) => {
            return Err(format!(
                "Item nbt should be of form `nbt:{{tag:{{...}}}}`; got `nbt:{{tag:{other:?}}}`"
            ))
        }
        None => {
            obj.insert("tag".into(), nbt!({_is_minescript: item.name.clone()}));
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

// given a syntax element within the item annotation, crate the nbt contents of the recipe file
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
        Syntax::Annotation(ident, inner) => match ident.as_ref() {
            "crafting_shaped" | "shaped" => {
                if !inner.is_object() {
                    return Err(format!("Expected recipe object; got `{inner:?}`"));
                }
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
            other => Err(format!("Unexpected recipe annotation: `{other}`")),
        },
        _ => Err(format!("Expected recipe object; got {value:?}")),
    }
}
