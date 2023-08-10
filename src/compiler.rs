use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Write,
};

use crate::types::prelude::*;
use crate::MAX_VERSION;

pub fn compile(src: &mut InterRepr, namespace: &str) -> SResult<CompiledRepr> {
    let mut compiled = CompiledRepr::new(core::mem::take(&mut src.loot_tables));

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
            "\nscoreboard players set %__const__{value:x} dummy {value}"
        ));
    }
    compiled.insert_fn("__load__", load.into());
    compile_items(src, namespace, &mut compiled)?;
    // put all the functions in
    for (name, statements) in &src.functions {
        let name: RStr = fmt_mc_ident(name).into();
        let fn_buf = statements.map_ref(|func| {
            let mut fn_buf = String::new();
            for statement in func {
                fn_buf.push('\n');
                fn_buf.push_str(&statement.stringify(namespace));
            }
            fn_buf
        });
        compiled.insert_fn(&name, fn_buf);
    }
    // put all the advancements in
    for (key, value) in &src.advancements {
        compiled.advancements.insert(key.clone(), value.to_json());
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
        let give_fn = compiled
            .functions
            .get::<str>(&format!("give/{item_name}"))
            .unwrap()
            .clone();
        compiled.insert_fn(
          &format!("craft/{name}"),
          give_fn.map(|give| format!("clear @s knowledge_book 1\nadvancement revoke @s only {namespace}:craft/{name}\n{give}", 
                  ))        );
    }
    Ok(compiled)
}

fn compile_items(src: &mut InterRepr, namespace: &str, compiled: &mut CompiledRepr) -> SResult<()> {
    let mut tick_buf = Versioned::default();
    let mut using_base_item_scores = BTreeSet::new();
    for item in src.items.clone() {
        let ident = fmt_mc_ident(&item.name);

        let mut give_obj = match &item.nbt {
            Nbt::Object(obj) => obj.clone(),
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
            format!(
                "give @s minecraft:{base}{nbt}",
                base = item.base,
                nbt = Nbt::Object(give_obj)
            )
            .into(),
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
                    ExecuteOption::As(
                        Selector::a().with_property(
                            "nbt",
                            nbt!({ Inventory: nbt!([nbt!({slot: *slot,tag:item.nbt.clone()})]) })
                                .to_string(),
                        ),
                    ),
                    ExecuteOption::At(Selector::s()),
                ],
                fn_content.clone(),
                &format!("__internal__/slot_{slot:x}_{:x}", get_hash(fn_content)),
                src,
            );
            tick_buf.push('\n');
            tick_buf.push_str_v(cmd.map(|cmd| cmd.stringify(namespace)));
        }
    }
    for base_score in using_base_item_scores {
        tick_buf.push_str(&format!("\nscoreboard players reset @a {base_score}\n"));
    }
    if !tick_buf.is_empty() {
        compiled.insert_fn("__tick__", tick_buf);
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
    let consume_fn = item.on_consume.map_ref(|func| {
        let mut consume_fn = format!("advancement revoke @s only {namespace}:consume/{ident}");
        for cmd in func {
            consume_fn.push('\n');
            consume_fn.push_str(&cmd.stringify(namespace));
        }
        consume_fn
    });
    compiled
        .advancements
        .insert(format!("consume/{ident}").into(), advancement_content);
    compiled.insert_fn(&on_consume, consume_fn);
}

fn make_on_use(
    item: &Item,
    ident: &str,
    tick_buf: &mut Versioned<String>,
    namespace: &str,
    using_base_item_scores: &mut BTreeSet<String>,
    src: &mut InterRepr,
) {
    let on_use = format!("use/{}", item.name);
    let using_base = format!("use_{}", item.base);
    let holding_item = format!("holding_{ident}");
    let execute_fn = Command::execute(
        vec![
            ExecuteOption::As(Selector {
                selector_type: SelectorType::A,
                args: [
                    ("tag".into(), holding_item.clone()),
                    ("scores".into(), format!("{{{using_base}=1}}")),
                ]
                .into_iter()
                .collect(),
            }),
            ExecuteOption::At(Selector::s()),
        ],
        item.on_use.clone(),
        &on_use,
        src,
    );
    tick_buf.push('\n');
    tick_buf.push_str_v(execute_fn.map(|cmd| cmd.stringify(namespace)));
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
    let on_use_fn = item.while_using.map_ref(|func| {
        let mut on_use_fn_content = format!("advancement revoke @s only {namespace}:use/{ident}");
        for cmd in func {
            on_use_fn_content.push('\n');
            on_use_fn_content.push_str(&cmd.stringify(namespace));
        }
        on_use_fn_content
    });
    compiled
        .advancements
        .insert(format!("use/{ident}").into(), advancement_content);
    compiled.insert_fn(&while_using, on_use_fn);
}

pub fn write(repr: &CompiledRepr, parent: &str, nmsp: &str) -> Result<(), std::io::Error> {
    let _ = fs::remove_dir_all(&format!("{parent}{nmsp}"));
    let mut versions = BTreeSet::new();
    for (path, contents) in &repr.functions {
        if !contents.base().is_empty() {
            let mut file = create_file_with_parent_dirs(&format!(
                "{parent}{nmsp}/data/{nmsp}/functions/{path}.mcfunction"
            ))?;
            write!(file, "{}", contents.base())?;
            if &**path == "__tick__" {
                let mut tick = create_file_with_parent_dirs(&format!(
                    "{parent}{nmsp}/data/minecraft/tags/functions/tick.json"
                ))?;
                write!(tick, "{{\"values\":[\"{nmsp}:__tick__\"]}}")?;
            }
            if &**path == "__load__" {
                let mut load = create_file_with_parent_dirs(&format!(
                    "{parent}{nmsp}/data/minecraft/tags/functions/load.json"
                ))?;
                write!(load, "{{\"values\":[\"{nmsp}:__load__\"]}}")?;
            }
        }
        for (version, content) in contents.versions() {
            versions.insert(*version);
            let mut file = create_file_with_parent_dirs(&format!(
                "{parent}{nmsp}/fmt_{version}/data/{nmsp}/functions/{path}.mcfunction"
            ))?;
            write!(file, "{content}")?;
        }
    }
    for (path, contents) in &repr.advancements {
        let mut file = create_file_with_parent_dirs(&format!(
            "{parent}{nmsp}/data/{nmsp}/advancements/{path}.json"
        ))?;
        write!(file, "{contents}")?;
    }
    for (path, contents) in &repr.recipes {
        let mut file = create_file_with_parent_dirs(&format!(
            "{parent}{nmsp}/data/{nmsp}/recipes/{path}.json"
        ))?;
        write!(file, "{contents}")?;
    }
    for (path, contents) in &repr.loot_tables {
        let mut file = create_file_with_parent_dirs(&format!(
            "{parent}{nmsp}/data/{nmsp}/loot_tables/{path}.json"
        ))?;
        write!(file, "{contents}")?;
    }
    let mut mcmeta = create_file_with_parent_dirs(&format!("{parent}{nmsp}/pack.mcmeta"))?;
    #[allow(clippy::cast_lossless)]
    let overlays_nbt = versions
        .into_iter()
        .map(|version| {
            nbt!({
                directory: format!("fmt_{version}"),
                formats: nbt!({min_inclusive: version as i32})
            })
        })
        .rev()
        .collect::<Vec<Nbt>>();
    write!(
        mcmeta,
        "{}",
        nbt!({
            pack: nbt!({pack_format: 15, description: format!("{nmsp}, made with MineScript"), overlays: overlays_nbt, supported_formats: nbt!({
                min_inclusive: 15, max_inclusive: MAX_VERSION as i32
            })})
        })
        .to_json()
    )?;
    Ok(())
}

fn create_file_with_parent_dirs(filename: &str) -> Result<File, std::io::Error> {
    let parent_dir = std::path::Path::new(filename).parent().unwrap();
    fs::create_dir_all(parent_dir)?;

    File::create(filename)
}
