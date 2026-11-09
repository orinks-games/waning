# Waning

An audio roguelike sidescroller for blind and visually impaired players,
built on [Hearken](https://github.com/orinks-games/hearken).

Heaven's Lamp is going dark, and the Hollow King at the bottom of the world
is drinking its light. You descend at full strength through four regions,
each with two areas and a boss, and every gate takes 15 percent of your
power and health. When you fall, your souls become grace, which buys
blessings for the next descent, and the Keeper tells you more of the story.

- Walk, run, jump, dash, block and parry, and charge heavy blows.
- Six kinds of enemy, elites among them, and four bosses with their own
  attacks and warnings.
- Pits and spike traps from the second area on.
- Gold, potions, and gear in three slots and three rarities.
- A merchant in every region, plus wells, altars and chests.
- Ten relics, four blessings, and a story told between runs.
- Your descent is saved at the start of each stage.
- Settings for longer warnings, a slower game and always running.

## Controls

Left and Right walk (hold Shift to run), Space attacks (hold it for a heavy
blow), Up jumps, Z dashes, X blocks, Q drinks a potion, Down uses or picks
up, I opens the inventory, S reads your status, D names the nearest enemy,
Escape pauses. F1 reads help anywhere, and every game control can be moved
to another key in Settings.

## Build and run

Install the tools in Hearken's
[Getting started](https://github.com/orinks-games/hearken#getting-started),
then in this folder:

    cargo run --release

Run it from this folder so the game finds `sounds/`.

## Tests

    cargo test

`tests/balance.rs` plays 40 runs with a scripted player that misses one
warning in four and never blocks, dashes or charges a blow. With no
blessings it dies around the Drowned Steps and never wins; with two levels of each
blessing it wins a little under half its runs.

## Releases

`.github/workflows/release.yml` runs `hearken release` on Windows, macOS
and Linux. Start it by hand for test builds, or push a `v` tag to attach
the downloads to that tag's GitHub release.

## Sounds

`tools/build_sounds.py` downloads CC0 recordings and music from
OpenGameArt and builds every file in `sounds/`. `sounds/CREDITS.md` lists the sources, and
[docs/making-sounds.md](docs/making-sounds.md) compares the ways of making
and finding sounds.

To hear a run without playing, record one that plays itself, with the
spoken lines read by Hearken's built-in voice:

    cargo run --release --example demo -- demo.wav 240 2
