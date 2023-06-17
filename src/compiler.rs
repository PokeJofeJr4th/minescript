use std::collections::BTreeMap;

use crate::{command::Nbt, interpreter::InterRep, nbt, RStr};

#[derive(Debug, Clone, Default)]
pub struct CompiledData {
    pub functions: BTreeMap<RStr, String>,
    pub advancements: BTreeMap<RStr, String>,
    pub recipes: BTreeMap<RStr, String>,
    pub mcmeta: String,
}

pub fn compile(src: &InterRep, namespace: &str) -> Result<CompiledData, String> {
    let mut compiled = CompiledData {
        mcmeta: nbt!({
            pack: nbt!({
              pack_format: 11,
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
                  trigger: "minecraft:recipe_unlocked",
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
          format!("clear @s knowledge_book 1\nadvancement revoke @s only {namespace}:craft/{name}\nrecipe take @s {namespace}:{name}\n{give}", 
          give=compiled.functions.get::<RStr>(&format!("give/{name}").into()).ok_or_else(|| String::from("Some kind of weird internal error happened with the recipe :("))?)
        );
    }
    println!("{compiled:?}");
    Ok(compiled)
}

fn compile_items(
    src: &InterRep,
    namespace: &str,
    compiled: &mut CompiledData,
) -> Result<(), String> {
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
                "give @s {base}{nbt}",
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
                        item.base.clone()
                      ]),
                      nbt: item.nbt.clone()
                    })
                  })
                })
              }),
              rewards: nbt!({
                fuction: format!("{namespace}:{on_consume}")
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
            let on_use = on_use.to_lowercase().replace(' ', "_").into();
            let advancement_content = nbt!({
              criteria: nbt!({
                requirement: nbt!({
                  trigger: "minecraft:using_item",
                  conditions: nbt!({
                    item: nbt!({
                      items: nbt!([
                        item.base.clone()
                      ]),
                      nbt: item.nbt.clone()
                    })
                  })
                })
              }),
              rewards: nbt!({
                fuction: format!("{namespace}:{on_use}")
              })
            })
            .to_json();
            compiled
                .advancements
                .insert(format!("use/{ident}").into(), advancement_content);
            compiled.functions.insert(
                on_use,
                format!("advancement revoke @s only {namespace}:use/{ident}"),
            );
        }
    }
    Ok(())
}
