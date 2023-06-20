# Minescript

Rust-compiled markup language to create Minecraft datapacks

## Usage

First, set up your source file. The following is a simple, fully-functional example:

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
where namespace is the name of your datapack. Minescript will create a folder ready to be dropped into the datapacks folder of your minecraft world!

To learn more about the language, check out the [wiki](https://github.com/PokeJofeJr4th/minescript/wiki)