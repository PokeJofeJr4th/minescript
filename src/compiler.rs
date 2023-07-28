use std::collections::{BTreeMap, BTreeSet};

use crate::types::prelude::*;

pub fn compile(src: &mut InterRepr, namespace: &str) -> SResult<CompiledRepr> {
    let mut compiled = CompiledRepr::new(namespace, core::mem::take(&mut src.loot_tables));

    let mut load = format!("say {namespace}, a datapack created with MineScript");
    // add all the scoreboard objectives
    for (objective, trigger) in &src.objectives {
        load.push('\n');
        load.push_str("scoreboard objectives add ");
        load.push_str(objective);
        load.push(' ');
        load.push_str(trigger);
    }
    // add all the consts
    for value in &src.constants {
        load.push_str(&format!(
            "\nscoreboard players set %const_{value:x} dummy {value}"
        ));
    }
    compiled.insert_fn("load", &load);
    compile_items(src, namespace, &mut compiled)?;
    // put all the functions in
    for (name, statements) in &src.functions {
        let name: RStr = fmt_mc_ident(name).into();
        let mut fn_buf = String::new();
        for statement in statements {
            fn_buf.push('\n');
            fn_buf.push_str(&statement.stringify(namespace));
        }
        compiled.insert_fn(&name, &fn_buf);
    }
    // make all the recipes
    for (name, (content, item_name)) in &src.recipes {
        let name: RStr = fmt_mc_ident(name).into();
        let item_name = fmt_mc_ident(item_name);
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
        compiled.insert_fn(
          &format!("craft/{name}"),
          &format!("clear @s knowledge_book 1\nadvancement revoke @s only {namespace}:craft/{name}\n{give}", 
          give=compiled.functions.get::<RStr>(&format!("give/{item_name}").into()).ok_or_else(|| String::from("Some kind of weird internal error happened with the recipe :("))?)
        );
    }
    Ok(compiled)
}

fn compile_items(src: &mut InterRepr, namespace: &str, compiled: &mut CompiledRepr) -> SResult<()> {
    let mut tick_buf = String::new();
    let mut using_base_item_scores = BTreeSet::new();
    for item in src.items.clone() {
        let ident = fmt_mc_ident(&item.name);

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
        compiled.insert_fn(
            &format!("give/{ident}"),
            &format!(
                "give @s minecraft:{base}{nbt}",
                base = item.base,
                nbt = Nbt::Object(give_obj)
            ),
        );

        // make the consume function
        if !item.on_consume.is_empty() {
            make_on_consume(&item, &ident, namespace, compiled);
        }

        // make the use function
        if !item.on_use.is_empty() {
            make_on_use(
                &item,
                &ident,
                &mut tick_buf,
                namespace,
                &mut using_base_item_scores,
                src,
            );
        }

        // make the while_using function
        if !item.while_using.is_empty() {
            make_while_using(&item, &ident, namespace, compiled);
        }

        // make the slot checks
        for (slot, fn_content) in &item.slot_checks {
            let cmd = Command::execute(
                vec![
                    ExecuteOption::As {
                        selector: Selector::a().with_property(
                            "nbt",
                            nbt!({
                                Inventory: nbt!([nbt!({
                                    slot: *slot,
                                    tag: item.nbt.clone()
                                })])
                            })
                            .to_string(),
                        ),
                    },
                    ExecuteOption::At {
                        selector: Selector::s(),
                    },
                ],
                fn_content.clone(),
                &format!("closure/slot_{slot:x}_{:x}", get_hash(fn_content)),
                src,
            );
            tick_buf.push('\n');
            tick_buf.push_str(&cmd.stringify(namespace));
        }
    }
    for base_score in using_base_item_scores {
        tick_buf.push_str(&format!("scoreboard players reset @a {base_score}\n"));
    }
    if !tick_buf.is_empty() {
        compiled.insert_fn("tick", &tick_buf);
    }
    Ok(())
}

fn make_on_consume(item: &Item, ident: &str, namespace: &str, compiled: &mut CompiledRepr) {
    let on_consume: RStr = format!("consume/{}", item.name).into();
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
    for cmd in &item.on_consume {
        consume_fn.push('\n');
        consume_fn.push_str(&cmd.stringify(namespace));
    }
    compiled
        .advancements
        .insert(format!("consume/{ident}").into(), advancement_content);
    compiled.insert_fn(&on_consume, &consume_fn);
}

fn make_on_use(
    item: &Item,
    ident: &str,
    tick_buf: &mut String,
    namespace: &str,
    using_base_item_scores: &mut BTreeSet<String>,
    src: &mut InterRepr,
) {
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

fn make_while_using(item: &Item, ident: &str, namespace: &str, compiled: &mut CompiledRepr) {
    let while_using: RStr = format!("using/{}", item.name).into();
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
    let mut on_use_fn_content = format!("advancement revoke @s only {namespace}:use/{ident}");
    for cmd in &item.while_using {
        on_use_fn_content.push('\n');
        on_use_fn_content.push_str(&cmd.stringify(namespace));
    }
    compiled
        .advancements
        .insert(format!("use/{ident}").into(), advancement_content);
    compiled.insert_fn(&while_using, &on_use_fn_content);
}
