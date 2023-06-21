use std::collections::{BTreeMap, BTreeSet};

use crate::types::prelude::*;

pub fn compile(src: &InterRepr, namespace: &str) -> SResult<CompiledRepr> {
    let mut compiled = CompiledRepr {
        mcmeta: nbt!({
            pack: nbt!({
              pack_format: 15,
              description: format!("{namespace}, made with MineScript")
            })
        })
        .to_json(),
        ..Default::default()
    };
    let mut load = format!("say {namespace}, a datapack created with MineScript");
    for (objective, trigger) in &src.objectives {
        load.push('\n');
        load.push_str("scoreboard objectives add ");
        load.push_str(objective);
        load.push(' ');
        load.push_str(trigger);
    }
    compiled.functions.insert("load".into(), load);
    compile_items(src, namespace, &mut compiled)?;
    for (name, statements) in &src.functions {
        let name: RStr = name.to_lowercase().replace(' ', "_").into();
        let mut fn_buf = String::new();
        for statement in statements {
            fn_buf.push('\n');
            fn_buf.push_str(&statement.stringify(namespace));
        }
        match compiled.functions.get_mut(&name) {
            Some(func) => func.push_str(&fn_buf),
            None => {
                compiled.functions.insert(name.clone(), fn_buf);
            }
        }
    }
    for (name, content) in &src.recipes {
        let name: RStr = name.to_lowercase().replace(' ', "_").into();
        compiled.recipes.insert(name.clone(), content.clone());
        compiled.advancements.insert(
            format!("craft/{name}").into(),
            nbt!({
              criteria: nbt!{{
                requirement: nbt!{{
                  trigger: "minecraft:recipe_crafted",
                  conditions: nbt!{{
                    recipe: format!("{namespace}:{name}")
                  }}
                }}
              }},
              rewards: nbt!{{
                function: format!("{namespace}:craft/{name}")
              }}
            })
            .to_json(),
        );
        compiled.functions.insert(
          format!("craft/{name}").into(),
          format!("clear @s knowledge_book 1\nadvancement revoke @s only {namespace}:craft/{name}\n{give}", 
          give=compiled.functions.get::<RStr>(&format!("give/{name}").into()).ok_or_else(|| String::from("Some kind of weird internal error happened with the recipe :("))?)
        );
    }
    Ok(compiled)
}

#[allow(clippy::too_many_lines)]
fn compile_items(src: &InterRepr, namespace: &str, compiled: &mut CompiledRepr) -> SResult<()> {
    let mut tick_buf = String::new();
    let mut using_base_item_scores = BTreeSet::new();
    for item in &src.items {
        let ident = item.name.to_lowercase().replace(' ', "_");

        let mut give_obj = match &item.nbt {
            Nbt::Object(obj) => obj.clone(),
            Nbt::Unit => BTreeMap::new(),
            other => return Err(format!("Expected NBT object; got {other}")),
        };

        give_obj.insert(
            String::from("display").into(),
            nbt!({
                Name: format!(
                    "{{\\\"text\\\":\\\"{}\\\",\\\"italic\\\":\\\"false\\\"}}",
                    item.name
                )
            }),
        );

        compiled.functions.insert(
            format!("give/{ident}").into(),
            format!(
                "give @s minecraft:{base}{nbt}",
                base = item.base,
                nbt = Nbt::Object(give_obj)
            ),
        );

        if let Some(on_consume) = &item.on_consume {
            let on_consume = on_consume.to_lowercase().replace(' ', "_").into();
            let advancement_content = nbt!({
              criteria: nbt!({
                requirement: nbt!({
                  trigger: "minecraft:consume_item",
                  conditions: nbt!({
                    item: nbt!({
                      items: nbt!([
                        format!("minecraft:{}", item.base)
                      ]),
                      nbt: item.nbt.to_json()
                    })
                  })
                })
              }),
              rewards: nbt!({
                function: format!("{namespace}:{on_consume}")
              })
            })
            .to_json();
            compiled
                .advancements
                .insert(format!("consume/{ident}").into(), advancement_content);
            compiled.functions.insert(
                on_consume,
                format!("advancement revoke @s only {namespace}:consume/{ident}"),
            );
        }
        if let Some(on_use) = &item.on_use {
            let on_use = on_use.to_lowercase().replace(' ', "_");
            let using_base = format!("use_{}", item.base);
            let holding_item = format!("holding_{ident}");
            tick_buf.push_str(&format!("execute as @a[tag={holding_item},scores={{{using_base}=1}}] run function {namespace}:{on_use}\n"));
            tick_buf.push_str(&format!(
                "tag @a remove {holding_item}\ntag @a[nbt={{SelectedItem:{{id:\"minecraft:{}\",tag:{}}}}}] add {holding_item}\n",
                item.base,
                item.nbt
            ));
            using_base_item_scores.insert(using_base);
        }
        if let Some(while_using) = &item.while_using {
            let while_using = while_using.to_lowercase().replace(' ', "_").into();
            let advancement_content = nbt!({
              criteria: nbt!({
                requirement: nbt!({
                  trigger: "minecraft:using_item",
                  conditions: nbt!({
                    item: nbt!({
                      items: nbt!([
                        format!("minecraft:{}", item.base)
                      ]),
                      nbt: item.nbt.clone()
                    })
                  })
                })
              }),
              rewards: nbt!({
                function: format!("{namespace}:{while_using}")
              })
            })
            .to_json();
            compiled
                .advancements
                .insert(format!("use/{ident}").into(), advancement_content);
            compiled.functions.insert(
                while_using,
                format!("advancement revoke @s only {namespace}:use/{ident}"),
            );
        }
    }
    for base_score in using_base_item_scores {
        tick_buf.push_str(&format!("scoreboard players reset @a {base_score}\n"));
    }
    compiled.functions.insert("tick".into(), tick_buf);
    Ok(())
}
