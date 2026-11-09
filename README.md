# Waning

An audio sidescroller for blind and visually impaired players, built on
[Hearken](https://github.com/orinks-games/hearken). A prototype for trying
out ways to make and find sound effects.

Heaven's Lamp is going dark, and the Hollow King at the bottom of the world
is drinking its light. You descend at full strength through four regions,
each with two areas and a boss. Every gate takes 15 percent of your power
and health, so the deeper you go, the more blows each enemy takes and the
more you must dodge. Areas are laid out fresh each run, with wells, altars
and chests to find and ten relics to collect. When you fall, your souls
become grace, which buys blessings for the next descent, and the Keeper
tells you more of the story.

## Controls

Left and Right walk, Space attacks, Up jumps (over an enemy, or out of a
blow or bolt), Down uses a well, altar or chest, S reads your status, D
names the nearest enemy, R lists your relics, Escape pauses. F1 reads help anywhere. All game controls can be moved to
other keys in Settings.

## Build and run

Install the tools in Hearken's
[Getting started](https://github.com/orinks-games/hearken#getting-started),
then in this folder:

    cargo run --release

Run it from this folder so the game finds `sounds/`.

## Tests

    cargo test

`tests/balance.rs` plays 40 runs with a scripted player that misses one
warning in four. With no blessings it dies around the Weeping Mother and
never wins; with two levels of each blessing it wins about one run in five.

## Sounds

`tools/build_sounds.py` downloads CC0 recordings and music from
OpenGameArt and builds every file in `sounds/`. `sounds/CREDITS.md` lists the sources, and
[docs/making-sounds.md](docs/making-sounds.md) compares the ways of making
and finding sounds.

To hear a run without playing, record one that plays itself, with the
spoken lines read by Hearken's built-in voice:

    cargo run --release --example demo -- demo.wav 240 2
