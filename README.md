# Quick Roll

Discord bot for rolling dice concisely. Provides an alternative to the slash command, since those
are fiddly, especially on
mobile. The aim is to provide dice rolling that doesn't interrupt roleplay.

Quick Roll currently crashes in some edge cases, but you won't run into them in normal gameplay.
Just don't roll a die with 0 or 10,000,000,000 sides.

## Usage

Clone the repo, create a `TOKEN.txt` containing a bot's token in the repo's root, and `cargo run`.

## Syntax

- d20: r
- d20-3: r-3
- 2d8+3: r2d8+3
- 4d8+2d6+d4+3 with advantage: r4d8+2d6+d4+3a
- d20 with disadvantage: rd
- d20+1 with arbitrary comment: r1/initiative

## License

Quick Roll is dual-licensed under MIT and Apache 2.0 at your option.

## Contributing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.
