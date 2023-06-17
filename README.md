# minescript

Rust-comipled markup language to create Minecraft datapacks

## Usage

First, set up your source file. The following is a simple, fully-functional example:

```
@item {
    base: "minecraft:cookie"
    name: "Goodberry"
    nbt: {customModelData:10007}
    on_consume: [
        @effect {
            effect: "saturation"
            level: 10
            duration: 1
        }
    ]
    recipe: {
        pattern: [
            " l "
            "lbl"
            " l "
        ]
        key: {
            b: "minecraft:sweet_berries"
            l: "minecraft:lapis_lazuli"
        }
    }
}
```

To compile to a Minecraft datapack, run the following command:
`minescript src.txt <namespace>`
where namespace is the name of your datapack. Minescript will create a folder ready to be dropped into the datapacks folder of your minecraft world!

## Syntax

### Literal

Minescript literals are very simple

- Integers are trivial
- Strings are enclosed in double quotes and allow escaped double quotes
- Floats require a portion on both the left and right side of the decimal point

### Identifier

Any string of ASCII alphanumeric characters, plus `_`, is considered an identifier if it cannot be coerced to an integer.

### Object

The object is a pair of curly braces containing a set of key-value pairs. Keys are identifiers and Values can be any type of syntax element, but macros will often require certain types. The comma or semicolon between a value and the following key is optional.

### Array

The array is a pair of square braces containing a set of syntax elements. These can be any element, but macros often require certain types. Commas or semicolons separating values are optional.

### Macro

The macro is an `@` sign followed by the name of the macro and a syntax element, most often an object. Macros are the main way code is produced.

#### Item

The item macro defines a custom item type, including a /give function by default. The following is the minimal implementation:

```
@item {
    base: "minecraft:bread"
    name: "My Custom Item"
}
```

The custom name is automatically incorporated into the item's corresponding give function. The base is the item the game uses as the base for its properties.

The following implementation shows the optional fields:

```
@item {
    nbt: {customModelData: 42}
    on_consume: [
        ...
    ]
    while_using: [
        ...
    ]
    recipe: {
        pattern: [
            "ooo"
            " l "
            " l "
        ]
        key: {
            l: "minecraft:stick"
            o: "minecraft:string"
        }
    }
}
```

`nbt` should be an object containing data to be placed into the item whenever it is given or checked for. `display:{Name:"..."}` should not be included unless you wish to remove the item's functionality when it is renamed.

`on_consume` and `while_using` contain function bodies, which are lists that expand to commands. `on_consume` only applies to items that trigger `consume_item` advancements, like food and potions. `while_using` only applies to items that trigger the `using_item` advancements, like bows, shields, and spyglasses. Currently, `while_using` is run every tick.

`recipe` creates a crafting recipe for the item. On the crafting table, it looks like it produces a knowledge book, but the datapack replaces it once it's crafted. Only shaped recipes are currently supported.

#### Effect

The effect macro compiles to a command in the form of `effect give @s [effect] [duration] [level]`. The following is a minimal implementation:

```
@effect {
    effect: "saturation"
}
```

The following implementation contains the default value for all optional fields:

```
@effect {
    selector: @s
    duration: infinite
    level: 1
}
```

#### Function

The function macro compiles to a command in the form of `function [namespace]:[function]`. The following is a standard invocation:

```
@function "give/my_custom_item"
```

Note that your datapack's namespace is inserted by the compiler, so you don't need to include it in the macro invocation.

### Variables

Variables and assignments allow more complex control flow to be implemented. Here are some examples:

```
x = 10
x %= 2
x *= x
x = @r:some_objective
@p:some_objective = 2
```

The possible forms of the left side include `variable_name`, `variable_name.objective_name`, and `selector.objective_name`. The right side can have any of those forms or be an integer.
Internally, minescript uses scoreboard objectives for variables. These examples compile to the following commands:

```
scoreboard players set %x dummy 10
scoreboard players set % dummy 2
scoreboard players operation %x dummy %= % dummy
scoreboard players operation %x dummy *= %x dummy
scoreboard players operation %x dummy = @r some_objective
scoreboard players set @p some_objective 2
```

The compiler ensures that all scoreboard objectives referenced in variables exist and initializes them to the `dummy` objective if they don't. Minescript assignments support all operations present in the `scoreboard players operation` command: `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `><` (swap values), `<` (choose lower), and `>` (choose greater).

> To complete certain operations, the compiler uses multiple minecraft commands. `x %= 2` requires the literal `2` to be placed into another objective before the game will complete the `%=` operation.

### Control Flow

Currently, the only form of control flow in Minescript is the if statement. Here's an example:

```
x += 1
if x > 10 {
    @effect { ... }
    x = 0
}
```

Operations supported include `=`, `!=`, `<`, `>`, `<=`, and `>=`. The left and right sides follow the same rules as variables; note that numbers can't be used on the left.

> Even though Minecraft doesn't make all of these operations available, the compiler can use the `unless` option to get the effect.
