use std::{collections::BTreeMap, rc::Rc};

use crate::types::prelude::*;

#[derive(Debug)]
pub struct Item {
    pub name: RStr,
    pub base: RStr,
    pub nbt: Nbt,
    /// function that runs when the item is consumed
    pub on_consume: Option<RStr>,
    /// function that runs when the item is used
    pub on_use: Option<RStr>,
    /// function that runs every tick while the item is being used
    pub while_using: Option<RStr>,
}

#[derive(Debug)]
pub struct InterRep {
    pub items: Vec<Item>,
    pub objectives: BTreeMap<RStr, RStr>,
    pub functions: Vec<(RStr, Vec<Command>)>,
    pub recipes: BTreeMap<RStr, String>,
}

impl InterRep {
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            objectives: BTreeMap::new(),
            functions: Vec::new(),
            recipes: BTreeMap::new(),
        }
    }
}

pub fn interpret(src: &Syntax) -> SResult<InterRep> {
    let mut state = InterRep::new();
    inner_interpret(src, &mut state)?;
    Ok(state)
}

fn inner_interpret(src: &Syntax, state: &mut InterRep) -> SResult<Vec<Command>> {
    match src {
        Syntax::Array(statements) => {
            let mut commands_buf = Vec::new();
            for statement in statements.iter() {
                commands_buf.extend(inner_interpret(statement, state)?);
            }
            return Ok(commands_buf);
        }
        Syntax::Macro(name, properties) => {
            match name.as_ref() {
                "item" => {
                    // can't borrow state as mutable more than once at a time
                    let item = interpret_item(properties, state)?;
                    state.items.push(item);
                }
                "effect" => {
                    return interpret_effect(properties);
                }
                "function" => {
                    return Ok(vec![Command::Function {
                        func: Rc::<str>::try_from(&**properties)
                            .map_err(|_| String::from("Function macro should have a string"))?,
                    }])
                }
                "raw" => {
                    match &**properties {
                        Syntax::String(cmd) => return Ok(vec![Command::Raw(cmd.clone())]),
                        Syntax::Array(arr) => {
                            return arr.iter().map(|syn| {
                        if let Syntax::String(cmd) = syn {
                            Ok(Command::Raw(cmd.clone()))
                        } else {
                            Err(format!("`@raw` takes a string or list of strings, not `{syn:?}`"))
                        }
                    }).collect::<SResult<Vec<Command>>>();
                        }
                        other => {
                            return Err(format!(
                                "`@raw` takes a string or list of strings, not `{other:?}`"
                            ))
                        }
                    }
                }
                "sound" | "playsound" => return interpret_sound(properties),
                other => return Err(format!("Unexpected macro invocation `{other}`")),
            }
        }
        Syntax::Function(func, content) => {
            let inner = inner_interpret(content, state)?;
            state.functions.push((func.clone(), inner));
        }
        Syntax::Block(BlockType::If, left, op, right, block) => {
            return interpret_if(
                left,
                *op,
                right,
                &inner_interpret(block, state)?,
                &format!("{:x}", get_hash(block)),
                state,
            )
        }
        Syntax::Block(block_type, left, op, right, block) => {
            return interpret_loop(*block_type, left, *op, right, block, state)
        }
        Syntax::BinaryOp(target, op, syn) => return interpret_operation(target, *op, syn, state),
        Syntax::BlockSelector(BlockSelectorType::Tp, selector, body) => {
            return interpret_teleport(selector, body)
        }
        Syntax::BlockSelector(BlockSelectorType::Damage, selector, body) => {
            return interpret_damage(selector, body)
        }
        Syntax::BlockSelector(block_type, selector, body) => {
            return interpret_block_selector(*block_type, selector, body, state)
        }
        // Syntax::Identifier(_) => todo!(),
        Syntax::Unit => {}
        other => return Err(format!("Unexpected item `{other:?}`")),
    }
    Ok(Vec::new())
}

/// This function allows a test to expose `inner_interpret` without interacting with `InterRep`
#[cfg(test)]
pub fn test_interpret(src: &Syntax) -> SResult<Vec<Command>> {
    inner_interpret(src, &mut InterRep::new())
}

fn interpret_damage(selector: &Selector<Syntax>, properties: &Syntax) -> SResult<Vec<Command>> {
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

fn interpret_sound(properties: &Syntax) -> SResult<Vec<Command>> {
    let Syntax::Object(obj) = properties else {
        return Err(format!("Sound macro expects an object, not {properties:?}"))
    };
    let mut sound: Option<RStr> = None;
    let mut pos = Coordinate::here();
    let mut source: RStr = "master".into();
    let mut target: Selector<Syntax> = Selector::e();
    let mut volume = 1.0f32;
    let mut pitch = 1.0f32;
    let mut min_volume = 0.0f32;
    for (k, v) in obj {
        match &**k {
            "sound" => match RStr::try_from(v) {
                Ok(str) => sound = Some(str),
                Err(_) => {
                    return Err(format!(
                        "Expected a string or identifier for sound; got `{v:?}`"
                    ))
                }
            },
            "pos" | "posititon" | "location" => pos = Coordinate::try_from(v)?,
            "source" => match RStr::try_from(v) {
                Ok(str) => source = str,
                Err(_) => {
                    return Err(format!(
                        "Expected a string or identifier for sound source; got `{v:?}`"
                    ))
                }
            },
            "target" => {
                let Syntax::Selector(selector) = v else {
                    return Err(format!("Expected a selector for sound target; got `{v:?}`"))
                };
                target = selector.clone();
            }
            "volume" => match v {
                Syntax::Integer(int) => volume = *int as f32,
                Syntax::Float(float) => volume = *float,
                other => return Err(format!("Expected float or int for volume; got `{other:?}`")),
            },
            "pitch" => match v {
                Syntax::Integer(int) => pitch = *int as f32,
                Syntax::Float(float) => pitch = *float,
                other => return Err(format!("Expected float or int for pitch; got `{other:?}`")),
            },
            "minvolume" | "min_volume" => match v {
                Syntax::Integer(int) => min_volume = *int as f32,
                Syntax::Float(float) => min_volume = *float,
                other => {
                    return Err(format!(
                        "Expected float or int for min volume; got `{other:?}`"
                    ))
                }
            },
            other => return Err(format!("Invalid key for Sound macro: `{other}`")),
        }
    }
    let Some(sound) = sound else {
                    return Err(String::from("Sound macro must specify the sound to play"))
                };
    Ok(vec![Command::Sound {
        sound,
        source,
        target: target.stringify()?,
        pos,
        volume,
        pitch,
        min_volume,
    }])
}

fn interpret_block_selector(
    block_type: BlockSelectorType,
    selector: &Selector<Syntax>,
    body: &Syntax,
    state: &mut InterRep,
) -> SResult<Vec<Command>> {
    let mut res_buf = Vec::new();
    if block_type == BlockSelectorType::As || block_type == BlockSelectorType::AsAt {
        res_buf.push(ExecuteOption::As {
            selector: selector.stringify()?,
        });
    }
    if block_type == BlockSelectorType::At {
        res_buf.push(ExecuteOption::At {
            selector: selector.stringify()?,
        });
    } else if block_type == BlockSelectorType::AsAt {
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

fn interpret_teleport(selector: &Selector<Syntax>, body: &Syntax) -> SResult<Vec<Command>> {
    Ok(vec![Command::Teleport {
        target: selector.stringify()?,
        destination: Coordinate::try_from(body)?,
    }])
}

fn interpret_loop(
    block_type: BlockType,
    left: &OpLeft,
    op: Operation,
    right: &Syntax,
    block: &Syntax,
    state: &mut InterRep,
) -> SResult<Vec<Command>> {
    assert_ne!(block_type, BlockType::If);
    let fn_name: RStr = format!("closure/{:x}", get_hash(block)).into();
    // for _ in .. => replace `_` with hash
    let left = if block_type == BlockType::For && left == &OpLeft::Ident("_".into()) {
        OpLeft::Ident(get_hash(block).to_string().into())
    } else {
        left.clone()
    };
    let [goto_fn] = &interpret_if(
            &left,
            op,
            right,
            &[Command::Function {
                func: fn_name.clone(),
            }],
            "",
            state,
        )?[..] else {
            return Err(format!("Internal compiler error - please report this to the devs. {}{}", file!(), line!()))
        };
    let mut body = inner_interpret(block, state)?;
    if block_type == BlockType::For {
        let &Syntax::Range(start, _) = right else {
                return Err(format!("Expected a range in for loop; got `{right:?}`"))
            };
        body.push(Command::ScoreAdd {
            target: left.stringify_scoreboard_target()?,
            objective: left.stringify_scoreboard_objective(),
            value: start.unwrap_or(0),
        });
    }
    // don't perform the initial check for do-while or for loops
    if block_type == BlockType::DoWhile || block_type == BlockType::For {
        body.push(Command::Function {
            func: fn_name.clone(),
        });
    } else {
        body.push(goto_fn.clone());
    }
    state.functions.push((fn_name, body));
    Ok(if block_type == BlockType::For {
        // reset the value at the end of a for loop
        vec![
            goto_fn.clone(),
            Command::ScoreSet {
                target: left.stringify_scoreboard_target()?,
                objective: left.stringify_scoreboard_objective(),
                value: 0,
            },
        ]
    } else {
        vec![goto_fn.clone()]
    })
}

#[allow(clippy::too_many_lines)]
fn interpret_if(
    left: &OpLeft,
    op: Operation,
    right: &Syntax,
    content: &[Command],
    hash: &str,
    state: &mut InterRep,
) -> SResult<Vec<Command>> {
    if content.is_empty() {
        return Err(String::from("`if` body cannot be empty"));
    }
    let cmd: Command = if let [cmd] = content {
        cmd.clone()
    } else {
        let func_name: RStr = format!("closure/{hash}").into();
        state.functions.push((func_name.clone(), content.to_vec()));
        Command::Function { func: func_name }
    };
    let target_player = left.stringify_scoreboard_target()?;
    let target_objective = left.stringify_scoreboard_objective();
    let options = match right {
        Syntax::Identifier(_) | Syntax::BinaryOp(_, _, _) | Syntax::ColonSelector(_, _) => {
            let (source, source_objective) = match right {
                Syntax::Identifier(ident) => (ident.clone(), "dummy".into()),
                Syntax::BinaryOp(left, Operation::Colon, right) => match &**right {
                    Syntax::Identifier(ident) => {
                        (left.stringify_scoreboard_target()?, ident.clone())
                    }
                    _ => {
                        return Err(format!(
                            "Scoreboard must be indexed by an identifier; got {right:?}"
                        ))
                    }
                },
                Syntax::ColonSelector(selector, right) => {
                    (selector.stringify()?.to_string().into(), right.clone())
                }
                _ => return Err(format!("Can't compare to `{right:?}`")),
            };
            match op {
                // x = var
                Operation::LCaret
                | Operation::LCaretEq
                | Operation::Equal
                | Operation::RCaretEq
                | Operation::RCaret => {
                    vec![ExecuteOption::ScoreSource {
                        invert: false,
                        target: target_player,
                        target_objective,
                        operation: op,
                        source,
                        source_objective,
                    }]
                }
                // x != var
                Operation::BangEq => {
                    vec![ExecuteOption::ScoreSource {
                        invert: true,
                        target: target_player,
                        target_objective,
                        operation: Operation::Equal,
                        source,
                        source_objective,
                    }]
                }
                _ => return Err(format!("Can't compare using `{op}`")),
            }
        }
        Syntax::Integer(num) => {
            let (invert, lower, upper): (bool, Option<i32>, Option<i32>) = match op {
                // x = 1 => if x matches 1
                Operation::Equal => (false, Some(*num), Some(*num)),
                // x >= 1 => if x matches 1..
                Operation::RCaretEq => (false, Some(*num), None),
                // x <= 1 => if x matches ..1
                Operation::LCaretEq => (false, None, Some(*num)),
                // x != 1 => unless x matches 1
                Operation::BangEq => (true, Some(*num), Some(*num)),
                // x > 1 => unless x matches ..1
                Operation::RCaret => (true, None, Some(*num)),
                // x < 1 => unless x matches 1..
                Operation::LCaret => (true, Some(*num), None),
                _ => return Err(format!("Can't evaluate `if <variable> {op} <number>`")),
            };
            vec![ExecuteOption::ScoreMatches {
                invert,
                target: target_player,
                objective: target_objective,
                lower,
                upper,
            }]
        }
        Syntax::Range(left, right) => {
            if op != Operation::In {
                return Err(format!(
                    "The only available operation for a range like `{right:?}` is `in`; not `{op}`"
                ));
            };
            vec![ExecuteOption::ScoreMatches {
                invert: false,
                target: target_player,
                objective: target_objective,
                lower: *left,
                upper: *right,
            }]
        }
        _ => return Err(format!("Can't end an if statement with {right:?}")),
    };
    Ok(vec![Command::execute(options, cmd)])
}

#[allow(clippy::too_many_lines)]
fn interpret_operation(
    target: &OpLeft,
    op: Operation,
    syn: &Syntax,
    state: &mut InterRep,
) -> SResult<Vec<Command>> {
    let target_objective = target.stringify_scoreboard_objective();
    let target = target.stringify_scoreboard_target()?;
    if !state.objectives.contains_key(&target_objective) {
        state
            .objectives
            .insert(target_objective.clone(), "dummy".into());
    }
    match (op, syn) {
        // x = y
        (op, Syntax::Identifier(ident)) => {
            if !state.objectives.contains_key(&target_objective) {
                state
                    .objectives
                    .insert(target_objective.clone(), "dummy".into());
            }
            Ok(vec![Command::ScoreOperation {
                target,
                target_objective,
                operation: op,
                source: format!("%{ident}").into(),
                source_objective: "dummy".into(),
            }])
        }
        // x = @r.y
        (op, Syntax::ColonSelector(sel, ident)) => Ok(vec![Command::ScoreOperation {
            target,
            target_objective,
            operation: op,
            source: format!("{}", sel.stringify()?).into(),
            source_objective: ident.clone(),
        }]),
        // x *= 0
        (Operation::MulEq, Syntax::Integer(0)) => Ok(vec![Command::ScoreSet {
            target,
            objective: target_objective,
            value: 0,
        }]),
        // x /= 0
        (Operation::DivEq | Operation::ModEq, Syntax::Integer(0)) => {
            Err(String::from("Can't divide by zero"))
        }
        // x = 2
        (Operation::Equal, Syntax::Integer(int)) => Ok(vec![Command::ScoreSet {
            target,
            objective: target_objective,
            value: *int,
        }]),
        // x *= 1 => nop
        (Operation::MulEq | Operation::DivEq | Operation::ModEq, Syntax::Integer(1))
        | (Operation::AddEq | Operation::SubEq, Syntax::Integer(0)) => Ok(Vec::new()),
        // x += 2
        (Operation::AddEq, Syntax::Integer(int)) => Ok(vec![Command::ScoreAdd {
            target,
            objective: target_objective,
            value: *int,
        }]),
        // x -= 2
        (Operation::SubEq, Syntax::Integer(int)) => Ok(vec![Command::ScoreAdd {
            target,
            objective: target_objective,
            value: -int,
        }]),
        // x *= 2 => x += x
        (Operation::MulEq, Syntax::Integer(2)) => Ok(vec![Command::ScoreOperation {
            source: target.clone(),
            source_objective: target_objective.clone(),
            target,
            target_objective,
            operation: Operation::MulEq,
        }]),
        // x %= 2
        (op, Syntax::Integer(int)) => {
            state.objectives.insert("dummy".into(), "dummy".into());
            Ok(vec![
                Command::ScoreSet {
                    target: "%".into(),
                    objective: "dummy".into(),
                    value: *int,
                },
                Command::ScoreOperation {
                    target,
                    target_objective,
                    operation: op,
                    source: "%".into(),
                    source_objective: "dummy".into(),
                },
            ])
        }
        (Operation::MulEq | Operation::DivEq, Syntax::Float(float)) => {
            let approx = farey_approximation(
                if op == Operation::MulEq {
                    *float
                } else {
                    1.0 / *float
                },
                100,
            );
            Ok(vec![
                Command::ScoreSet {
                    target: "%".into(),
                    objective: "dummy".into(),
                    value: approx.0,
                },
                Command::ScoreOperation {
                    target: target.clone(),
                    target_objective: target_objective.clone(),
                    operation: Operation::MulEq,
                    source: "%".into(),
                    source_objective: "dummy".into(),
                },
                Command::ScoreSet {
                    target: "%".into(),
                    objective: "dummy".into(),
                    value: approx.1,
                },
                Command::ScoreOperation {
                    target,
                    target_objective,
                    operation: Operation::DivEq,
                    source: "%".into(),
                    source_objective: "dummy".into(),
                },
            ])
        }
        _ => Err(format!("Unsupported operation: {target:?} {op} {syn:?}")),
    }
}

#[allow(clippy::too_many_lines)]
fn interpret_item(src: &Syntax, state: &mut InterRep) -> SResult<Item> {
    let Syntax::Object(src) = src else {
        return Err(format!("Expected an object for item macro; got `{src:?}`"))
    };
    let mut item = Item {
        name: String::new().into(),
        base: String::new().into(),
        nbt: Nbt::default(),
        on_consume: None,
        on_use: None,
        while_using: None,
    };
    let mut on_consume_buf = Vec::new();
    let mut on_use_buf = Vec::new();
    let mut while_using_buf = Vec::new();
    let mut recipe_buf = None;
    for (prop, value) in src.iter() {
        match prop.as_ref() {
            "name" => {
                let Ok(name) = Rc::<str>::try_from(value) else {
                    return Err(String::from("Item name must be a string"))
                };
                item.name = name;
            }
            "base" => {
                let Ok(name) = Rc::<str>::try_from(value) else {
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
                Syntax::String(str) => item.on_consume = Some(str.clone()),
                Syntax::Function(name, body) => {
                    let new_body = inner_interpret(body, state)?;
                    state.functions.push((name.clone(), new_body));
                    item.on_consume = Some(name.clone());
                }
                other => {
                    on_consume_buf = inner_interpret(other, state)?;
                }
            },
            "on_use" => match value {
                Syntax::String(str) => item.on_use = Some(str.clone()),
                Syntax::Function(name, body) => {
                    let new_body = inner_interpret(body, state)?;
                    state.functions.push((name.clone(), new_body));
                    item.on_use = Some(name.clone());
                }
                other => on_use_buf = inner_interpret(other, state)?,
            },
            "while_using" => match value {
                Syntax::String(str) => item.while_using = Some(str.clone()),
                Syntax::Function(name, body) => {
                    let new_body = inner_interpret(body, state)?;
                    state.functions.push((name.clone(), new_body));
                    item.while_using = Some(name.clone());
                }
                other => {
                    while_using_buf = inner_interpret(other, state)?;
                }
            },
            "recipe" => {
                let Syntax::Object(obj) = value else {
                    return Err(format!("Expected recipe object; got {value:?}"))
                };
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
                recipe_buf = Some(nbt!({
                    type: "minecraft:crafting_shaped",
                    pattern: pattern,
                    key: new_key,
                    result: nbt!({
                        item: "minecraft:knowledge_book",
                        count: 1
                    })
                }));
            }
            other => return Err(format!("Unexpected item property: `{other}`")),
        }
    }
    if !on_consume_buf.is_empty() {
        let func_name: RStr = format!("consume/{}", item.name).into();
        state.functions.push((func_name.clone(), on_consume_buf));
        item.on_consume = Some(func_name);
    }
    if !on_use_buf.is_empty() {
        let func_name: RStr = format!("use/{}", item.name).into();
        state.functions.push((func_name.clone(), on_use_buf));
        item.on_use = Some(func_name);
    }
    if item.on_use.is_some() {
        state.objectives.insert(
            format!("use_{}", item.base).into(),
            format!("minecraft.used:minecraft.{}", item.base).into(),
        );
    }
    if !while_using_buf.is_empty() {
        let func_name: RStr = format!("using/{}", item.name).into();
        state.functions.push((func_name.clone(), while_using_buf));
        item.while_using = Some(func_name);
    }
    if let Some(recipe) = recipe_buf {
        state.recipes.insert(item.name.clone(), recipe.to_json());
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

fn interpret_effect(src: &Syntax) -> SResult<Vec<Command>> {
    let mut selector: Option<Selector<String>> = None;
    let mut effect = None;
    let mut duration = None;
    let mut level = 1;
    if let Syntax::Object(src) = src {
        for (prop, value) in src.iter() {
            match prop.as_ref() {
                "selector" => match value {
                    Syntax::Selector(sel) => {
                        selector = Some(sel.stringify()?);
                    }
                    other => {
                        return Err(format!(
                            "Unexpected element: `{other:?}`; expected a selector"
                        ))
                    }
                },
                "effect" => {
                    let Ok(eff) = Rc::<str>::try_from(value) else {
                        return Err(String::from("Potion effect must be a string"))
                    };
                    effect = Some(eff);
                }
                "duration" => match value {
                    Syntax::Identifier(str) | Syntax::String(str) => {
                        if *str != "infinite".into() {
                            return Err(format!(
                                "Potion duration should be an integer or infinite, not `{str}`"
                            ));
                        }
                    }
                    Syntax::Integer(num) => duration = Some(*num),
                    other => {
                        return Err(format!(
                            "Potion duration should be an integer or infinite, not `{other:?}`"
                        ))
                    }
                },
                "level" => match value {
                    Syntax::Integer(num) => level = *num,
                    other => {
                        return Err(format!(
                            "Potion level should be an integer, not `{other:?}`"
                        ))
                    }
                },
                other => return Err(format!("Unexpected potion property: `{other}`")),
            }
        }
    } else if let Ok(str) = RStr::try_from(src) {
        effect = Some(str);
    } else {
        return Err(format!("Expected an object for item macro; got `{src:?}`"));
    };

    let Some(effect) = effect else {
        return Err(String::from("Effect must include the effect id; {... effect: \"...\"}"))
    };

    Ok(vec![Command::EffectGive {
        target: selector.unwrap_or(Selector {
            selector_type: SelectorType::S,
            args: BTreeMap::new(),
        }),
        effect,
        duration,
        level,
    }])
}
