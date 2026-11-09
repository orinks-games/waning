# Waning

An audio sidescroller for blind and visually impaired players, built on
[Hearken](https://github.com/orinks-games/hearken). A prototype for trying
out ways to make and find sound effects.

You descend from heaven at full strength through eight areas. Every gate
takes a quarter of your power and health, so the deeper you go, the more
blows each enemy takes and the more you must dodge. Enemies drop souls; when
you fall, they become grace, which buys blessings for the next descent:
Might, Vigor, Steadfast and Mercy.

## Controls

Left and Right walk, Space attacks, Up jumps (over an enemy, or out of its
blow after it cries out), S reads your status, D names the nearest enemy,
Escape pauses. F1 reads help anywhere. All game controls can be moved to
other keys in Settings.

## Build and run

Install the tools in Hearken's
[Getting started](https://github.com/orinks-games/hearken#getting-started),
then in this folder:

    cargo run --release

Run it from this folder so the game finds `sounds/`.

## Tests

    cargo test

`tests/balance.rs` plays 40 runs with a scripted player that misses one cry
in four. It reaches about area 4 of 8 with no blessings and area 7 with a
handful.

## Sounds

`tools/make_sounds.py` builds every file in `sounds/`, synthesized in code
or converted from a CC0 pack. `sounds/CREDITS.md` lists the sources, and
[docs/making-sounds.md](docs/making-sounds.md) compares the ways of making
and finding sounds.

To hear a run without playing, record one that plays itself, with the
spoken lines read by Hearken's built-in voice:

    cargo run --release --example demo -- demo.wav 150
