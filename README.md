[![Rust](https://github.com/PokeJofeJr4th/minescript/actions/workflows/rust.yml/badge.svg)](https://github.com/PokeJofeJr4th/minescript/actions/workflows/rust.yml)
# Minescript

Minescript is a powerful language with a Rust transpiler that turns it into a Minecraft datapack. It simplifies many of minecraft commands' roughest edges, like control flow, variables, and interoperability between commands, advancements, and recipes.

## Usage

First, download Minescript with the instructions in the [quickstart guide](https://github.com/PokeJofeJr4th/minescript/wiki). Next, set up your source file. The following is a simple, fully-functional example:

```
@item {
    base: cookie
    name: Goodberry
    nbt: {customModelData:10007}
    on_consume: [
        @effect {
            effect: saturation
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
            b: sweet_berries
            l: lapis_lazuli
        }
    }
}
```

To compile to a Minecraft datapack, run the following command:

`minescript src.txt <namespace>`

where `namespace` is the name of your datapack. Once you provide the location of your `.minecraft` folder, Minescript will create a folder ready to be dropped into the datapacks folder of your minecraft world!

For more information on how to use the command line interface, see the [command line format](https://github.com/PokeJofeJr4th/minescript/wiki/Command-Line). To learn more about what Minescript can do, check out the [wiki](https://github.com/PokeJofeJr4th/minescript/wiki) and [examples](https://github.com/PokeJofeJr4th/minescript/tree/main/examples).

## Contributing

Contributions to the Minescript project are welcome. Please provide as much reproducibility information as you can for issues. Pull requests should be well-documented unless they are small. Hate speech, harassment, etc. are not tolerated.
