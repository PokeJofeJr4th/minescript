use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::{collections::BTreeMap, rc::Rc};

use super::{inner_interpret, InterRepr};
use crate::types::prelude::*;
use crate::Config;

/// interpret any selector block of the form `tp @s (...)`
pub(super) fn block(
    block_type: BlockType,
    selector: &Selector<Syntax>,
    body: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
    config: &Config,
) -> SResult<VecCmd> {
    match block_type {
        BlockType::Tp => teleport(selector, body),
        BlockType::Damage => damage(selector, body),
        BlockType::Tellraw => tellraw(selector, body, config),
        block_type => selector_block(block_type, selector, body, state, path, src_files, config),
    }
}

/// interpret a tellraw block like `tellraw @a {...}`
fn tellraw(selector: &Selector<Syntax>, properties: &Syntax, config: &Config) -> SResult<VecCmd> {
    let mut nbt_buf: Vec<Nbt> = Vec::new();

    let arr = if let Syntax::Array(arr) = properties {
        arr.clone()
    } else {
        Rc::from([properties.clone()])
    };

    for item in arr.iter() {
        nbt_buf.push(tellraw_component(item, config)?);
    }

    Ok(vec![Command::TellRaw(
        selector.stringify()?,
        Nbt::Array(nbt_buf).to_json().into(),
    )]
    .into())
}

/// get a tellraw component
fn tellraw_component(src: &Syntax, config: &Config) -> SResult<Nbt> {
    match src {
        // a given object
        Syntax::Object(_) => Nbt::try_from(src),
        // a string
        Syntax::String(str) => Ok(nbt!({ text: str })),
        // dummy score value
        Syntax::Identifier(ident) => Ok(nbt!({
            score: nbt!({name: format!("%{ident}"), objective: config.dummy_objective.clone()})
        })),
        // named score
        Syntax::BinaryOp {
            lhs,
            operation: Operation::Colon,
            rhs,
        } => {
            let Syntax::Identifier(ident) = &**lhs else {
            return Err(format!("Expected name identifier, not `{lhs:?}`"))
        };
            let Syntax::Identifier(objective) = &**rhs else {
            return Err(format!("Expected score identifier, not `{rhs:?}`"))
        };
            Ok(nbt!({
                score: nbt!({name: format!("%{ident}"), objective: objective})
            }))
        }
        // named selector score
        Syntax::SelectorColon(sel, objective) => Ok(nbt!({
            score: nbt!({name: sel.stringify()?.to_string(), objective: objective})
        })),
        // entity name
        Syntax::Selector(sel) => Ok(nbt!({selector: sel.stringify()?.to_string()})),
        // a list of modifiers
        Syntax::Array(arr) => {
            let mut nbt_buf = BTreeMap::new();
            let mut base = BTreeMap::new();
            for item in arr.iter() {
                match item {
                    Syntax::Identifier(ident) => match &**ident {
                        "bold" => {
                            nbt_buf.insert("bold".into(), Nbt::TRUE);
                        }
                        "italic" => {
                            nbt_buf.insert("italic".into(), Nbt::TRUE);
                        }
                        "underlined" | "underline" => {
                            nbt_buf.insert("underlined".into(), Nbt::TRUE);
                        }
                        "strikethrough" => {
                            nbt_buf.insert("strikethrough".into(), Nbt::TRUE);
                        }
                        "obfuscated" | "obfuscate" => {
                            nbt_buf.insert("obfuscated".into(), Nbt::TRUE);
                        }
                        other => return Err(format!("Unsupported tellraw component: `{other}`")),
                    },
                    // key-value pair
                    Syntax::BinaryOp {
                        lhs,
                        operation: Operation::Colon,
                        rhs,
                    } if matches!(&**lhs, Syntax::Identifier(_)) => {
                        let Syntax::Identifier(ident) = &**lhs else {
                            unreachable!()
                        };
                        let content = String::try_from(&**rhs)?;
                        nbt_buf.insert(ident.clone(), content.into());
                    }
                    other => base = tellraw_component(other, config)?.get_obj()?.clone(),
                }
            }
            base.extend(nbt_buf);
            Ok(Nbt::Object(base))
        }
        other => Err(format!("Unsupported tellraw component: `{other:?}`")),
    }
}

/// interpret a block of type `damage @p {...}`
fn damage(selector: &Selector<Syntax>, properties: &Syntax) -> SResult<VecCmd> {
    let mut amount = 1;
    let mut damage_type: RStr = "entity-attack".into();
    let mut attacker: Selector<Syntax> = Selector::s();
    if let Syntax::Object(obj) = properties {
        for (k, v) in obj {
            match &**k {
                "amount" => match v {
                    Syntax::Integer(int) => amount = *int,
                    other => {
                        return Err(format!(
                            "Expected a number for damage amount; got `{other:?}`"
                        ))
                    }
                },
                "damage_type" | "type" | "source" => match String::try_from(v) {
                    Ok(str) => damage_type = str.into(),
                    Err(_) => {
                        return Err(format!("Expected a string for damage type; got `{v:?}`"))
                    }
                },
                "attacker" | "from" | "by" => {
                    let Syntax::Selector(sel) = v else {
                        return Err(format!("Damage annotation attacker should be selector; got `{v:?}`"))
                    };
                    attacker = sel.clone();
                }
                other => return Err(format!("Invalid key for damage annotation: `{other}`")),
            }
        }
    } else if let Syntax::Integer(int) = properties {
        amount = *int;
    } else {
        return Err(format!(
            "Damage annotation expected an object or integer; got `{properties:?}`"
        ));
    };
    Ok(vec![Command::Damage {
        target: selector.stringify()?,
        amount,
        damage_type,
        attacker: attacker.stringify()?,
    }]
    .into())
}

/// interpret a block of the form `at @s {...}`
fn selector_block(
    block_type: BlockType,
    selector: &Selector<Syntax>,
    body: &Syntax,
    state: &mut InterRepr,
    path: &Path,
    src_files: &mut BTreeSet<PathBuf>,
    config: &Config,
) -> SResult<VecCmd> {
    let mut res_buf = Vec::new();
    let selector = selector.stringify()?;
    // special case for `as at`: as @p[..] at @s
    if block_type == BlockType::AsAt {
        res_buf.push(ExecuteOption::As(selector.clone()));
    }
    match block_type {
        // special case for `as at`: as @p[..] at @s
        BlockType::AsAt => res_buf.push(ExecuteOption::At(Selector::s())),
        BlockType::At => res_buf.push(ExecuteOption::At(selector)),
        BlockType::If => res_buf.push(ExecuteOption::IfEntity {
            invert: false,
            selector,
        }),
        BlockType::Unless => res_buf.push(ExecuteOption::IfEntity {
            invert: true,
            selector,
        }),
        BlockType::Facing => res_buf.push(ExecuteOption::FacingEntity(selector)),
        BlockType::Rotated => res_buf.push(ExecuteOption::RotatedAs(selector)),
        BlockType::As => {
            if selector == Selector::s() {
                println!("\x1b[33mWARN\x1b[0m\t`as @s {{ ... }}`; This is a non-operation.");
            }
            res_buf.push(ExecuteOption::As(selector));
        }
        _ => return Err(format!("`{block_type:?}` block doesn't take a selector")),
    }
    let inner = inner_interpret(body, state, path, src_files, config)?;
    Ok(Command::execute(
        &res_buf,
        inner,
        &format!("__internal__/{block_type}_{:x}", get_hash(body)),
        state,
    )
    .into_vec())
}

/// interpret a teleport block
/// `tp @s @p`
/// `tp @s (~ ~ ~)`
fn teleport(selector: &Selector<Syntax>, body: &Syntax) -> SResult<VecCmd> {
    let target = selector.stringify()?;

    if let Ok(destination) = Coordinate::try_from(body) {
        if destination == Coordinate::here() {
            println!("\x1b[33mWARN\x1b[0m\t`tp @s (~ ~ ~)`; This is a non-operation.");
        }
        Ok(vec![Command::Teleport {
            target,
            destination,
        }]
        .into())
    } else if let Syntax::Selector(sel) = body {
        Ok(vec![Command::TeleportTo {
            target,
            destination: sel.stringify()?,
        }]
        .into())
    } else {
        Err(format!(
            "Expected coordinates or target for `tp` body; got `{body:?}`"
        ))
    }
}
