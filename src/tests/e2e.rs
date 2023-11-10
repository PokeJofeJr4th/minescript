use lazy_regex::lazy_regex;

use crate::types::RStr;

macro_rules! assert_e2e {
    ($src: expr => $output: expr) => {{
        let tokens = $crate::lexer::tokenize(&format!("[{}]", $src)).unwrap();
        let syntax = $crate::parser::parse(tokens).unwrap();
        let output = $crate::interpreter::test_interpret(&syntax);
        let output_txt = output
            .into_iter()
            .map(|cmd| format!("\n{}", cmd.stringify("test")))
            .collect::<String>()
            .trim()
            .to_string();
        assert_eq!(output_txt, $output);
    }};
}

macro_rules! build_e2e {
    ($src: expr) => {{
        let tokens = $crate::lexer::tokenize(&format!("[{}]", $src)).unwrap();
        let syntax = $crate::parser::parse(tokens).unwrap();
        let mut inter = $crate::interpreter::interpret(
            &syntax,
            ::std::path::Path::new(""),
            &mut ::std::collections::BTreeSet::new(),
            &$crate::Config { namespace: "test".into(), dummy_objective: "dummy".into(), fixed_point_accuracy: 100 }
        )
        .unwrap();
        $crate::compiler::compile(&mut inter, "test").unwrap()
    }};
}

#[test]
fn control_flow() {
    assert_e2e!("if x = 1 { @function \"use/goodberry\" }" =>
        "execute if score %x dummy matches 1 run function test:use/goodberry"
    );
    assert_e2e!("unless @s:score in 0..10 @raw \"...\"" => "execute unless score @s score matches 0..10 run ...");
    assert_e2e!("if @s:score != 2 @raw \"...\"" => "execute unless score @s score matches 2 run ...");
    assert_e2e!("unless x >= 1 @raw \"...\"" => "execute unless score %x dummy matches 1.. run ...");
    assert_e2e!("unless x > 1 @raw \"...\"" => "execute if score %x dummy matches ..1 run ...");
    assert_e2e!("if @s[type=cow] @raw \"...\"" =>"execute if entity @s[type=cow] run ...");
    assert_e2e!("unless @s[type=cow] @raw \"...\"" => "execute unless entity @s[type=cow] run ...");
    assert_e2e!("if @s::lvl > 10 @raw \"...\""
    => "execute store result score %__if__ dummy run xp query @s levels\nexecute unless score %__if__ dummy matches ..10 run ...");
}

#[test]
fn loops() {
    let for_loop = build_e2e!("function load for x in 0..10 @raw \"...\"");
    println!("{:#?}", for_loop.functions);
    let for_inner: RStr = lazy_regex!(".*\nfunction test:(__internal__/[0-9a-f]+)")
        .captures(for_loop.functions.get("load").unwrap().base())
        .unwrap()
        .get(1)
        .unwrap()
        .as_str()
        .into();
    assert_eq!(for_loop.functions.get(&for_inner).unwrap().base(), 
    &format!("\n...\nscoreboard players add %x dummy 1\nexecute if score %x dummy matches 0..10 run function test:{for_inner}"));

    let while_loop = build_e2e!("function load while x <= 10 x++");
    let while_inner: RStr = lazy_regex!(".*\nexecute if score %x dummy matches ..10 run function test:(__internal__/[0-9a-f]+)")
        .captures(while_loop.functions.get("load").unwrap().base())
        .unwrap()
        .get(1)
        .unwrap()
        .as_str()
        .into();
    assert_eq!(while_loop.functions.get(&while_inner).unwrap().base(), 
    &format!("\nscoreboard players add %x dummy 1\nexecute if score %x dummy matches ..10 run function test:{while_inner}"));
    
    let do_while_loop = build_e2e!("function load do while x <= 10 x++");
    let do_while_inner: RStr = lazy_regex!(".*\nfunction test:(__internal__/[0-9a-f]+)")
        .captures(do_while_loop.functions.get("load").unwrap().base())
        .unwrap()
        .get(1)
        .unwrap()
        .as_str()
        .into();
    assert_eq!(do_while_loop.functions.get(&do_while_inner).unwrap().base(), 
    &format!("\nscoreboard players add %x dummy 1\nexecute if score %x dummy matches ..10 run function test:{do_while_inner}"));

    let until_loop = build_e2e!("function load until x = 10 x++");
    let until_inner: RStr = lazy_regex!(".*\nexecute unless score %x dummy matches 10 run function test:(__internal__/[0-9a-f]+)")
        .captures(until_loop.functions.get("load").unwrap().base())
        .unwrap()
        .get(1)
        .unwrap()
        .as_str()
        .into();
    assert_eq!(until_loop.functions.get(&until_inner).unwrap().base(), 
    &format!("\nscoreboard players add %x dummy 1\nexecute unless score %x dummy matches 10 run function test:{until_inner}"));
    
    let do_until_loop = build_e2e!("function load do until x = 10 x++");
    let do_until_inner: RStr = lazy_regex!(".*\nfunction test:(__internal__/[0-9a-f]+)")
        .captures(do_until_loop.functions.get("load").unwrap().base())
        .unwrap()
        .get(1)
        .unwrap()
        .as_str()
        .into();
    assert_eq!(do_until_loop.functions.get(&do_until_inner).unwrap().base(), 
    &format!("\nscoreboard players add %x dummy 1\nexecute unless score %x dummy matches 10 run function test:{do_until_inner}"));
}

#[test]
fn execution_context() {
    assert_e2e!("as at @a @raw \"...\"" => "execute as @a at @s run ...");
    assert_e2e!("facing @p @raw \"...\"" => "execute facing entity @p run ...");
    assert_e2e!("facing (~ ~ ~1) @raw \"...\"" => "execute facing ~ ~ ~1 run ...");
    assert_e2e!("positioned (~ ~2 ~) @raw \"...\"" => "execute positioned ~ ~2 ~ run ...");
    assert_e2e!("rotated @p @raw \"...\"" => "execute rotated as @p run ...");
    assert_e2e!("rotated (~ ~) @raw \"...\"" => "execute rotated ~ ~ run ...");
    assert_e2e!("summon sheep @raw \"...\"" => "execute summon sheep run ...");
}

#[test]
fn annotations() {
    assert_e2e!("@function \"give/my_item\"" => "function test:give/my_item");
    assert_e2e!("@raw [\"function <NAMESPACE>:give/my_item\"; \"give @s diamond 64\"]" 
    => "function test:give/my_item\ngive @s diamond 64");
    assert_e2e!("@random flower_type in 10..20"
    => "execute store result score %flower_type dummy run loot spawn 0 -256 0 loot test:rng/10_20");
    assert_e2e!("@rand x in 10" 
    => "execute store result score %x dummy run loot spawn 0 -256 0 loot test:rng/0_10");
    assert_e2e!("@effect { effect: strength selector: @a duration: 30 level: 2 }" => "effect give @a strength 30 2");

    let raycast_repr = build_e2e!("function raycast @raycast {
  max: 200
  step: 0.2
  each: @raw \"each\"
  hit: @raw \"hit\"
}");
    let raycast_hash: RStr = lazy_regex!(".*\nexecute summon marker run function test:__internal__/([0-9a-f]+)")
        .captures(raycast_repr.functions.get("raycast").unwrap().base())
        .unwrap()
        .get(1)
        .unwrap()
        .as_str()
        .into();
    assert_eq!(raycast_repr.functions.get::<str>(&format!("__internal__/{raycast_hash}")).unwrap().base(), &format!("
execute rotated as @p run tp @s ~ ~1.5 ~ ~ ~
scoreboard players reset %__timer__{raycast_hash} dummy
execute at @s run function test:__internal__/loop_{raycast_hash}
execute at @s run hit
kill @s"));
    assert_eq!(raycast_repr.functions.get::<str>(&format!("__internal__/loop_{raycast_hash}")).unwrap().base(), &format!("
each
tp @s ^ ^ ^0.2
scoreboard players add %__timer__{raycast_hash} dummy 1
execute if score %__timer__{raycast_hash} dummy matches ..200 at @s if block ~ ~ ~ air run function test:__internal__/loop_{raycast_hash}"
));
}

#[test]
fn misc() {
    assert_e2e!("damage @p { amount: 10 source: fire by: @r }" => "damage @p 10 fire by @r");
    assert_e2e!("damage @p 10" => "damage @p 10 entity-attack by @s");
}

#[test]
fn teleport() {
    assert_e2e!("tp @s (~ ~10 ~)" => "tp @s ~ ~10 ~");
    assert_e2e!("teleport @s (^1 ^2 ^1)" => "tp @s ^1 ^2 ^1");
    assert_e2e!("tp @s (~, 255, ~)" => "tp @s ~ 255 ~");
    assert_e2e!("tp @s @p" => "tp @s @p");
}

#[test]
fn variables() {
    assert_e2e!("x = 1" => "scoreboard players set %x dummy 1");
    // assert_e2e!("enemy:score += 10" => "scoreboard players add %enemy score 10");
    assert_e2e!("@s:health -= @p:attack" => "scoreboard players operation @s health -= @p attack");
    assert_e2e!("x *= 2" => "scoreboard players operation %x dummy += %x dummy");
    assert_e2e!("x *= 1.618" 
    => "scoreboard players operation %x dummy *= %__const__90 dummy\nscoreboard players operation %x dummy /= %__const__59 dummy");
    assert_e2e!("x >< y" => "scoreboard players operation %x dummy >< %y dummy");
    assert_e2e!("x %= 10" => "scoreboard players operation %x dummy %= %__const__a dummy");
    assert_e2e!("x += 1" => "scoreboard players add %x dummy 1");

    assert_e2e!("@p::lvl += 1" => "xp add @p 1 levels");
    assert_e2e!("@p.Motion[0] = xvec" => "execute store result entity @p Motion[0] float 1 run scoreboard players get %xvec dummy");

    assert_e2e!("success ?= @raw \"kill @r\"" => "execute store success score %success dummy run kill @r");
    assert_e2e!("orbs := @raw \"kill @e[type=xp_orb,distance=..2]\""
    => "execute store result score %orbs dummy run kill @e[type=xp_orb,distance=..2]");
}

#[test]
fn floats() {
    assert_e2e!("x .= 1" => "scoreboard players set %x dummy 100");
    assert_e2e!("x .= 1.1" => "scoreboard players set %x dummy 110");
    assert_e2e!("x .+= 1" => "scoreboard players add %x dummy 100");
    assert_e2e!("x .-= 1.1" => "scoreboard players remove %x dummy 110");

    assert_e2e!("x .*= y" => "scoreboard players operation %x dummy *= %y dummy\nscoreboard players operation %x dummy /= %__const__64 dummy");
    assert_e2e!("x ./= y" => "scoreboard players operation %x dummy *= %__const__64 dummy\nscoreboard players operation %x dummy /= %y dummy");
}

#[test]
fn nbt() {
    assert_e2e!("@s.Inventory = @p.Inventory" => "data modify entity @s Inventory set from entity @p Inventory");
    assert_e2e!("@s.Inventory = []" => "data modify entity @s Inventory set value []");
    assert_e2e!("@e[type=cow].CustomName = \"Gregory\"" => "data modify entity @e[type=cow] CustomName set value \"Gregory\"");
    // assert_e2e!("@s.Health += 1" => "");
    assert_e2e!("@s:score += @p.Health" 
    => "execute store result score % dummy run data get entity @p Health\nscoreboard players operation @s score += % dummy");
}

#[test]
fn advancement() {
    let advancement_repr = build_e2e!("
    advancement \"join\" {
        criteria: {requirement: { trigger: \"minecraft:tick\" } }
        reward: @raw \"...\"
    }
    ");

    let advancement = advancement_repr.advancements.get("join").unwrap().clone();
    println!("{advancement:?}");
    let advancement_hash = lazy_regex!(r#"^\{"criteria":\s*\{"requirement":\s*\{"trigger":\s*"minecraft:tick"\}\},\s*"rewards":\{"function":"([a-z0-9:_/]+)"\}\}$"#).captures(&advancement).unwrap().get(1).unwrap().as_str().to_string();

    assert_eq!(advancement_repr.functions.get(&*advancement_hash).unwrap().base().trim(), "...");
}
