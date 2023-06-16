use std::collections::BTreeMap;

use crate::{command::Nbt, interpreter::InterpreterState, nbt};

#[derive(Debug, Clone, Default)]
pub struct CompiledData {
    pub functions: BTreeMap<String, String>,
    pub advancements: BTreeMap<String, String>,
    pub recipes: BTreeMap<String, String>,
    pub mcmeta: String,
}

pub fn compile(src: InterpreterState, namespace: &str) -> Result<CompiledData, String> {
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
    for item in src.items {
        let ident = item.name.to_lowercase().replace(' ', "_");

        let mut give_obj = match &item.nbt {
            Nbt::Object(obj) => obj.clone(),
            Nbt::Unit => BTreeMap::new(),
            other => return Err(format!("Expected NBT object; got {other}")),
        };

        give_obj.insert(
            String::from("display"),
            nbt!({
                Name: format!(
                    "{{\\\"text\\\":\\\"{}\\\",\\\"italic\\\":\\\"false\\\"}}",
                    item.name
                )
            }),
        );

        compiled.functions.insert(
            format!("give/{ident}"),
            format!(
                "give @s {base}{nbt}",
                base = item.base,
                nbt = Nbt::Object(give_obj)
            ),
        );

        if let Some(on_consume) = item.on_consume {
            let on_consume = on_consume.to_lowercase().replace(' ', "_");
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
                fuction: format!("{namespace}:{}", on_consume)
              })
            })
            .to_json();
            compiled
                .advancements
                .insert(format!("consume/{ident}"), advancement_content);
            compiled.functions.insert(
                on_consume,
                format!("advancement revoke @s only {namespace}:consume/{ident}"),
            );
        }
        if let Some(on_use) = item.on_use {
            let on_use = on_use.to_lowercase().replace(' ', "_");
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
                fuction: format!("{namespace}:{}", on_use)
              })
            })
            .to_json();
            compiled
                .advancements
                .insert(format!("use/{ident}"), advancement_content);
            compiled.functions.insert(
                on_use,
                format!("advancement revoke @s only {namespace}:use/{ident}"),
            );
        }
    }
    for (name, statements) in src.functions {
        let name = name.to_lowercase().replace(' ', "_");
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
    for (name, content) in src.recipes {
        let name = name.to_lowercase().replace(' ', "_");
        compiled.recipes.insert(name.clone(), content);
        compiled.advancements.insert(
            format!("craft/{name}"),
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
          format!("craft/{name}"),
          format!("clear @s knowledge_book 1\nadvancement revoke @s only {namespace}:craft/{name}\nrecipe take @s {namespace}:{name}\n{give}", 
          give=compiled.functions.get(&format!("give/{name}")).ok_or(String::from("Some kind of weird internal error happened with the recipe :("))?)
        );
    }
    println!("{compiled:?}");
    Ok(compiled)
}
