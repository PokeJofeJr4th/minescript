macro_rules! assert_e2e {
    ($src: expr => $output: expr) => {{
        let tokens = $crate::lexer::tokenize(&format!("[{}]", $src)).unwrap();
        let syntax = $crate::parser::parse(&mut tokens.into_iter().peekable()).unwrap();
        let output = $crate::interpreter::test_interpret(&syntax).unwrap();
        let output_txt = output
            .into_iter()
            .map(|cmd| format!("\n{}", cmd.stringify("test")))
            .collect::<String>()
            .trim()
            .to_string();
        assert_eq!(output_txt, $output);
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
fn macros() {
    assert_e2e!("@function \"give/my_item\"" => "function test:give/my_item");
    assert_e2e!("@raw [\"function <NAMESPACE>:give/my_item\"; \"give @s diamond 64\"]" 
    => "function test:give/my_item\ngive @s diamond 64");
    assert_e2e!("@random flower_type in 10..20"
    => "execute store result score %flower_type dummy run loot spawn 0 -256 0 loot test:rng/10_20");
    assert_e2e!("@rand x in 1" 
    => "execute store result score %x dummy run loot spawn 0 -256 0 loot test:rng/0_1");
    assert_e2e!("@effect { effect: strength selector: @a duration: 30 level: 2 }" => "effect give @a strength 30 2");
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
    => "scoreboard players operation %x dummy *= %const_90 dummy\nscoreboard players operation %x dummy /= %const_59 dummy");
    assert_e2e!("x >< y" => "scoreboard players operation %x dummy >< %y dummy");
    assert_e2e!("x %= 10" => "scoreboard players operation %x dummy %= %const_a dummy");
    assert_e2e!("x += 1" => "scoreboard players add %x dummy 1");

    assert_e2e!("@p::lvl += 1" => "xp add @p 1 levels");
}

#[test]
fn nbt() {
    assert_e2e!("@s.Inventory = @p.Inventory" => "data modify entity @s Inventory set from entity @p Inventory");
    // assert_e2e!("@s.Inventory = []" => "data modify entity @s Inventory set value []");
    assert_e2e!("@e[type=cow].CustomName = \"Gregory\"" => "data modify entity @e[type=cow] CustomName set value \"Gregory\"");
    // assert_e2e!("@s.Health += 1" => "");
    assert_e2e!("@s:score += @p.Health" 
    => "execute store result score % dummy run data get entity @p Health\nscoreboard players operation @s score += % dummy");
}
