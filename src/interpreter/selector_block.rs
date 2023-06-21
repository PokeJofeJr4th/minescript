use std::{collections::BTreeMap, rc::Rc};

use super::{inner_interpret, InterRepr};
use crate::types::prelude::*;
use crate::types::SelectorBlockType as SBT;

pub(super) fn block(
    block_type: SBT,
    selector: &Selector<Syntax>,
    body: &Syntax,
    state: &mut InterRepr,
) -> SResult<Vec<Command>> {
    match block_type {
        SBT::Tp => teleport(selector, body),
        SBT::Damage => damage(selector, body),
        SBT::TellRaw => tellraw(selector, body),
        block_type => selector_block(block_type, selector, body, state),
    }
}

fn tellraw(selector: &Selector<Syntax>, properties: &Syntax) -> SResult<Vec<Command>> {
    let mut nbt_buf: Vec<Nbt> = Vec::new();

    let arr = if let Syntax::Array(arr) = properties {
        arr.clone()
    } else {
        Rc::from([properties.clone()])
    };

    for item in arr.iter() {
        nbt_buf.push(tellraw_component(item)?);
    }

    Ok(vec![Command::TellRaw(
        selector.stringify()?,
        Nbt::Array(nbt_buf).to_json().into(),
    )])
}

fn tellraw_component(src: &Syntax) -> SResult<Nbt> {
    match src {
        // a given object
        Syntax::Object(_) => Nbt::try_from(src),
        // a string
        Syntax::String(str) => Ok(nbt!({ text: str })),
        // dummy score value
        Syntax::Identifier(ident) => Ok(nbt!({
            score: nbt!({name: format!("%{ident}"), objective: "dummy"})
        })),
        // named score
        Syntax::BinaryOp(OpLeft::Ident(ident), Operation::Colon, syn) => {
            let Syntax::Identifier(objective) = &**syn else {
            return Err(format!("Expected score identifier, not `{syn:?}`"))
        };
            Ok(nbt!({
                score: nbt!({name: format!("%{ident}"), objective: objective})
            }))
        }
        // named selector score
        Syntax::ColonSelector(sel, objective) => Ok(nbt!({
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
                    Syntax::BinaryOp(OpLeft::Ident(ident), Operation::Colon, syn) => {
                        let content = String::try_from(&**syn)?;
                        nbt_buf.insert(ident.clone(), content.into());
                    }
                    other => base = tellraw_component(other)?.get_obj()?.clone(),
                }
            }
            base.extend(nbt_buf);
            Ok(Nbt::Object(base))
        }
        other => Err(format!("Unsupported tellraw component: `{other:?}`")),
    }
}

fn damage(selector: &Selector<Syntax>, properties: &Syntax) -> SResult<Vec<Command>> {
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
                        return Err(format!("Damage macro attacker should be selector; got `{v:?}`"))
                    };
                    attacker = sel.clone();
                }
                other => return Err(format!("Invalid key for damage macro: `{other}`")),
            }
        }
    } else if let Syntax::Integer(int) = properties {
        amount = *int;
    } else {
        return Err(format!(
            "Damage macro expected an object or integer; got `{properties:?}`"
        ));
    };
    Ok(vec![Command::Damage {
        target: selector.stringify()?,
        amount,
        damage_type,
        attacker: attacker.stringify()?,
    }])
}

fn selector_block(
    block_type: SBT,
    selector: &Selector<Syntax>,
    body: &Syntax,
    state: &mut InterRepr,
) -> SResult<Vec<Command>> {
    let mut res_buf = Vec::new();
    if block_type == SBT::As || block_type == SBT::AsAt {
        res_buf.push(ExecuteOption::As {
            selector: selector.stringify()?,
        });
    }
    if block_type == SBT::At {
        res_buf.push(ExecuteOption::At {
            selector: selector.stringify()?,
        });
    } else if block_type == SBT::AsAt {
        res_buf.push(ExecuteOption::At {
            selector: Selector::s(),
        });
    }
    let inner = inner_interpret(body, state)?;
    let cmd = if let [cmd] = &inner[..] {
        cmd.clone()
    } else {
        let func_name: RStr = format!("closure/{:x}", get_hash(body)).into();
        state.functions.push((func_name.clone(), inner));
        Command::Function { func: func_name }
    };
    Ok(vec![Command::execute(res_buf, cmd)])
}

fn teleport(selector: &Selector<Syntax>, body: &Syntax) -> SResult<Vec<Command>> {
    Ok(vec![Command::Teleport {
        target: selector.stringify()?,
        destination: Coordinate::try_from(body)?,
    }])
}