# minescript
Rust-based markup language to create Minecraft datapacks
## Usage
First, set up your src.txt file
```
@item {
    base: "minecraft:cookie"
    name: "Goodberry"
    nbt: {customModelData:10007}
    on_consume: {
        @effect {
            effect: "saturation"
            level: 10
            duration: 1
        }
    }
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
