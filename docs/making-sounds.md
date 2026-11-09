# Making and finding sounds

What works for a Claude session building sounds for a Hearken game, tried on
this prototype.

## 1. Synthesize in code

A Python script with numpy and scipy, writing WAV and converting with ffmpeg.
The first version of this game made 20 of its 34 sounds this way; the
current one uses recordings for all but three (see section 2).

- Works anywhere: a cloud session installs numpy and scipy from PyPI in
  seconds, and ffmpeg is already there.
- Every sound is a short recipe with a fixed seed, so a change is a
  reviewable diff and a rerun gives the same file. Tweaking ("shorter",
  "lower", "less hiss") is an edit to one line.
- Good at: impacts, whooshes, footsteps, bells and chimes, risers and
  falls, UI earcons, wind, drones, pads, machine loops, creature presence
  loops. Seamless loops are easy (crossfade the tail into the head).
- Weak at: voices, animals, real-world textures (gravel, water, cloth,
  crowds). Those sound synthetic; use recordings.

genny does the same job with a JSON spec instead of Python, and Hearken can
also synthesize live at run time with fundsp (`hearken::synth`), which suits
sounds that change continuously, such as engines.

## 2. Free sound libraries (what Waning uses now)

`tools/build_sounds.py` downloads 26 CC0 packs and tracks from OpenGameArt,
cuts and levels each clip, builds enemy presence loops by placing several
recordings at random times, and writes `sounds/CREDITS.md`. 104 of the 107
sounds and all the music come from there.

Prefer CC0 (public domain, no conditions). CC-BY is fine when the credit is
written down at once. Avoid CC-BY-NC (no commercial use) and anything
"free for personal use" unless the game will never be sold.

| Source | Licences | Reaches from a cloud session |
|--------|----------|------------------------------|
| OpenGameArt.org | Mostly CC0 and CC-BY, stated per item | Yes: pages and zip downloads |
| Freesound.org | CC0, CC-BY, CC-BY-NC per sound | No (blocked). Its API needs a free account and key |
| Kenney.nl | All CC0 | No (blocked); download on the PC |
| Sonniss GDC bundles | Royalty-free, use in games, no attribution, no reselling the raw files | Manual download, 20 to 30 GB a year |
| Pixabay sound effects | Pixabay licence, free to use | No (blocked) |

From the PC, every source works through a browser. A cloud session can
then pick up files placed in an approved folder.

Freesound has the largest catalogue. If you want it, make a free account and
an API key at https://freesound.org/apiv2/apply and add it to the cloud
environment's secrets; a session can then search by tag and licence and
download previews (high-quality MP3s). Full-quality originals need OAuth
sign-in, so do those from the PC.

## 3. Editing any sound

ffmpeg and sox cover trimming, fades, pitch, speed, EQ, reverb, mixing,
loudness matching (`loudnorm`) and format conversion. The script here trims
silence, mixes to mono, normalizes and encodes Ogg Vorbis for every clip,
so they sit at matching levels. Music stays stereo and is matched by
loudness.

## Music

OpenGameArt has plenty of CC0 game music; Waning uses tracks by haeldb,
isaiah658, eponasoft and Juhani Junkala. Suno or ElevenLabs could make
custom tracks, but they spend credits.

## 4. Licence bookkeeping

Every non-original file gets a row in `sounds/CREDITS.md`: the sound key,
the source file, the author, the licence and the URL. The build script
writes it from the same table it builds from, so it cannot fall behind. CC-BY credits also go in the game's own credits.

## 5. Hearing the result

Claude can't listen. It can check levels, length and clipping, and render a
scripted run to a file (`examples/demo.rs`) for a person to judge by ear.
