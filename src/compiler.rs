use std::collections::{BTreeMap, BTreeSet};

use crate::types::prelude::*;

pub fn compile(src: &mut InterRepr, namespace: &str) -> SResult<CompiledRepr> {
    // @raycast {
    //   max
    //   amount
    //   callback
    // }
    // summon marker {
    //   for timer in 0..max {
    //     tp @s (^ ^ ^{amount});
    //     unless block #air timer = max;
    //   }
    //   at @s {callback}
    //   kill @s
    // }

    let mut compiled = CompiledRepr::new(namespace);

    let mut load = format!("say {namespace}, a datapack created with MineScript");
    // add all the scoreboard objectives
    for (objective, trigger) in &src.objectives {
        load.push('\n');
        load.push_str("scoreboard objectives add ");
        load.push_str(objective);
        load.push(' ');
        load.push_str(trigger);
    }
    compiled.functions.insert("load".into(), load);
    compile_items(src, namespace, &mut compiled)?;
    // put all the functions in
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
    // make all the recipes
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
                    recipe_id: format!("{namespace}:{name}")
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
fn compile_items(src: &mut InterRepr, namespace: &str, compiled: &mut CompiledRepr) -> SResult<()> {
    let mut tick_buf = String::new();
    let mut using_base_item_scores = BTreeSet::new();
    for item in src.items.clone() {
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

        // make the give function
        compiled.functions.insert(
            format!("give/{ident}").into(),
            format!(
                "give @s minecraft:{base}{nbt}",
                base = item.base,
                nbt = Nbt::Object(give_obj)
            ),
        );

        // make the consume function
        if !item.on_consume.is_empty() {
            let on_consume = format!("consume/{}", item.name).into();
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
            let mut consume_fn = format!("advancement revoke @s only {namespace}:consume/{ident}");
            for cmd in item.on_consume {
                consume_fn.push('\n');
                consume_fn.push_str(&cmd.stringify(namespace));
            }
            compiled
                .advancements
                .insert(format!("consume/{ident}").into(), advancement_content);
            compiled.functions.insert(on_consume, consume_fn);
        }

        // make the use function
        if !item.on_use.is_empty() {
            let on_use = format!("use/{}", item.name);
            let using_base = format!("use_{}", item.base);
            let holding_item = format!("holding_{ident}");
            let execute_fn = Command::execute(
                vec![
                    ExecuteOption::As {
                        selector: Selector {
                            selector_type: SelectorType::A,
                            args: [
                                ("tag".into(), holding_item.clone()),
                                ("scores".into(), format!("{{{using_base}=1}}")),
                            ]
                            .into_iter()
                            .collect(),
                        },
                    },
                    ExecuteOption::At {
                        selector: Selector::s(),
                    },
                ],
                item.on_use.clone(),
                &on_use,
                src,
            );
            tick_buf.push_str(&execute_fn.stringify(namespace));
            tick_buf.push('\n');
            tick_buf.push_str(&format!(
                "tag @a remove {holding_item}\ntag @a[nbt={{SelectedItem:{{id:\"minecraft:{}\",tag:{}}}}}] add {holding_item}\n",
                item.base,
                item.nbt
            ));
            using_base_item_scores.insert(using_base);
        }

        // make the while_using function
        if !item.while_using.is_empty() {
            let while_using = format!("using/{}", item.name).into();
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
            let mut fn_content = format!("advancement revoke @s only {namespace}:use/{ident}");
            for cmd in item.while_using {
                fn_content.push('\n');
                fn_content.push_str(&cmd.stringify(namespace));
            }
            compiled
                .advancements
                .insert(format!("use/{ident}").into(), advancement_content);
            compiled.functions.insert(while_using, fn_content);
        }
    }
    for base_score in using_base_item_scores {
        tick_buf.push_str(&format!("scoreboard players reset @a {base_score}\n"));
    }
    compiled.functions.insert("tick".into(), tick_buf);
    Ok(())
}
