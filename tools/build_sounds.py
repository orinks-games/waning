"""Build every sound in sounds/ for Waning, and sounds/CREDITS.md.

Almost everything is a recording or a piece of music from OpenGameArt.org,
all CC0. This script downloads the packs, cuts and levels each clip, builds
the loops (enemy presence, ambience, music) and writes the credits. Three
magic sounds with no recorded match (the wane, death, rising to heaven) are
synthesized with numpy.

Usage:
  python tools/build_sounds.py --cache <folder for the downloaded packs>

Needs numpy, scipy, ffmpeg (with libvorbis), unzip and 7z on PATH.
"""

import argparse
import glob
import os
import subprocess
import tempfile
import urllib.parse
import urllib.request

import numpy as np
from scipy import signal
from scipy.io import wavfile

SR = 44100
ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
OUT = os.path.join(ROOT, "sounds")
FILES = "https://opengameart.org/sites/default/files/"
PAGE = "https://opengameart.org/content/"

# id: (title, author, OpenGameArt page slug, download file names)
PACKS = {
    "rpg": ("RPG Sound Pack", "artisticdude", "rpg-sound-pack", ["rpg_sound_pack.zip"]),
    "rpg80": ("80 CC0 RPG SFX", "rubberduck", "80-cc0-rpg-sfx", ["80-CC0-RPG-SFX_0.zip"]),
    "creature80": ("80 CC0 creature SFX", "rubberduck", "80-cc0-creature-sfx", ["80-CC0-creature-SFX_0.zip"]),
    "water": ("40 CC0 water / splash / slime SFX", "rubberduck", "40-cc0-water-splash-slime-sfx", ["water-splash-slime-sfx.zip"]),
    "breaking": ("75 CC0 breaking / falling / hit sfx", "rubberduck", "75-cc0-breaking-falling-hit-sfx", ["sfx_breaking_and_falling.zip"]),
    "sfx100": ("100 CC0 SFX", "rubberduck", "100-cc0-sfx", ["100-CC0-SFX_0.zip"]),
    "steps": ("Fantozzi's Footsteps (Grass/Sand & Stone)", "qubodup", "fantozzis-footsteps-grasssand-stone", ["Fantozzi-footsteps.7z"]),
    "ghost": ("Ghost Monster Voice Moaning & Growling", "qubodup", "ghost-monster-voice-moaning-growling", ["qubodup-GhostMoans.zip"]),
    "monster": ("Monster Sound Pack, Volume 1", "ogrebane", "monster-sound-pack-volume-1", ["monster-sounds-volume-2.zip"]),
    "darkamb": ("Dark Ambiences", "ogrebane", "dark-ambiences", ["dark_ambiences.zip"]),
    "swishes": ("Swishes Sound Pack", "artisticdude", "swishes-sound-pack", ["swishes.zip"]),
    "tiny": ("Fantasy Sound Effects (Tinysized SFX)", "vehicle", "fantasy-sound-effects-tinysized-sfx", ["tinysized.zip"]),
    "park": ("Park ambiences", "thimras", "park-ambiences", ["park_ambience_birds.wav"]),
    "crickets": ("Crickets Ambient Noise - loopable", "wolfgang", "crickets-ambient-noise-loopable", ["crickets_1.mp3"]),
    "windy": ("windy ambient noise", "some-weirdo", "windy-ambient-noise", ["windy_ambiance.ogg"]),
    "swamp": ("Swamp Environment Audio", "lokif", "swamp-environment-audio", ["swamp.ogg"]),
    "drip": ("Dripping water loop", "qubodup", "dripping-water-loop", ["atmosbasement.mp3_.flac"]),
    "fire": ("Fireplace Sound loop", "pagdev", "fireplace-sound-loop", ["fire.wav"]),
    "fire1": ("Fire Crackling", "antumdeluge", "fire-crackling", ["fire-1.ogg"]),
    "dungeon": ("Loopable Dungeon Ambience", "jaggedstone", "loopable-dungeon-ambience", ["dungeon_ambient_1_0.ogg"]),
    "spacewind": ("Space Winds", "aquinn", "space-winds", ["space-wind.mp3"]),
    # Music
    "m_title": ("Tragic ambient main menu", "haeldb", "tragic-ambient-main-menu", ["ambientmain_0.ogg"]),
    "m_heaven": ("Heavenly Loop", "isaiah658", "heavenly-loop", ["Heavenly%20Loop.ogg"]),
    "m_explore": ("JRPG Pack 1 Exploration", "Juhani Junkala (subspaceaudio)", "jrpg-pack-1-exploration",
                  ["JRPG%20Music%20Pack%20%231%20%5BExploration%5D%20by%20Juhani%20Junkala.zip"]),
    "m_towns": ("JRPG Pack 2 Towns", "Juhani Junkala (subspaceaudio)", "jrpg-pack-2-towns",
                ["JRPG%20Music%20Pack%20%232%20%5BTowns%5D%20by%20Juhani%20Junkala.zip"]),
    "m_victory": ("Victory Theme for RPG", "cynicmusic", "victory-theme-for-rpg", ["Victory1_1.mp3"]),
    "m_evil": ("JRPG Pack 3 Evil", "Juhani Junkala (subspaceaudio)", "jrpg-pack-3-evil",
               ["Evil2%20-%20Catacombs.ogg", "Evil4%20-%20Witch%27s%20Lair.ogg",
                "Evil5%20-%20Whispers%20From%20Beyond.ogg", "Evil3%20-%20Apocalypse.ogg"]),
    "m_boss": ("Boss Battle Music", "Juhani Junkala (subspaceaudio)", "boss-battle-music",
               ["Juhani%20Junkala%20-%20Epic%20Boss%20Battle%20%5BSeamlessly%20Looping%5D.wav"]),
}

# ---------- fetching ----------


def fetch(cache):
    for pid, (_, _, _, files) in PACKS.items():
        folder = os.path.join(cache, pid)
        if os.path.isdir(folder) and os.listdir(folder):
            continue
        os.makedirs(folder, exist_ok=True)
        for name in files:
            local = os.path.join(folder, urllib.parse.unquote(name))
            print("downloading", urllib.parse.unquote(name))
            urllib.request.urlretrieve(FILES + name, local)
            if local.endswith(".zip"):
                subprocess.run(["unzip", "-qo", local, "-d", folder], check=True)
            elif local.endswith(".7z"):
                subprocess.run(["7z", "x", "-y", "-o" + folder, local], check=True,
                               stdout=subprocess.DEVNULL)


def find(cache, pid, name):
    """The file in a pack whose path ends with `name`."""
    hits = [p for p in glob.glob(os.path.join(cache, pid, "**", "*"), recursive=True)
            if p.replace("\\", "/").endswith("/" + name) and "__MACOSX" not in p]
    if not hits:
        raise SystemExit(f"{name} not found in pack {pid}")
    return sorted(hits, key=len)[0]


# ---------- audio helpers ----------


def decode(path, channels=1):
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
        tmp = f.name
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", path, "-ac", str(channels),
                    "-ar", str(SR), "-c:a", "pcm_s16le", tmp], check=True)
    _, x = wavfile.read(tmp)
    os.unlink(tmp)
    return x.astype(np.float64) / 32768.0


def trim(x, threshold=0.004):
    level = np.abs(x) if x.ndim == 1 else np.max(np.abs(x), axis=1)
    loud = np.nonzero(level > threshold)[0]
    if len(loud) == 0:
        return x
    return x[max(0, loud[0] - 44):loud[-1] + 441]


def fade(x, fin=0.002, fout=0.015):
    x = x.copy()
    i, o = min(int(SR * fin), len(x) // 4), min(int(SR * fout), len(x) // 4)
    ramp_in, ramp_out = np.linspace(0, 1, i), np.linspace(1, 0, o)
    if x.ndim == 2:
        ramp_in, ramp_out = ramp_in[:, None], ramp_out[:, None]
    if i:
        x[:i] *= ramp_in
    if o:
        x[-o:] *= ramp_out
    return x


def peak(x, db=-1.0):
    m = np.max(np.abs(x))
    return x if m == 0 else x * (10 ** (db / 20) / m)


def pitch(x, factor):
    """Resample to shift pitch and speed together (below 1 is lower)."""
    return signal.resample(x, int(len(x) / factor))


def lp(x, hz):
    b, a = signal.butter(2, hz / (SR / 2), "low")
    return signal.lfilter(b, a, x, axis=0)


def reverb(x, sec=1.0, wet=0.3, seed=1):
    rng = np.random.default_rng(seed)
    n = int(SR * sec)
    ir = rng.standard_normal(n) * np.exp(-6 * np.linspace(0, 1, n))
    ir = lp(ir, 5000)
    ir /= np.sqrt(np.sum(ir ** 2))
    tail = signal.fftconvolve(x, ir)
    dry = np.concatenate([x, np.zeros(len(tail) - len(x))])
    return (1 - wet) * dry + wet * tail


def crossfade_loop(x, xfade=1.0):
    """Fold the tail over the head so the clip loops with no seam."""
    n = min(int(SR * xfade), len(x) // 3)
    ramp = np.linspace(0, 1, n)
    if x.ndim == 2:
        ramp = ramp[:, None]
    head, body, tail = x[:n], x[n:-n], x[-n:]
    mixed = tail * np.cos(ramp * np.pi / 2) + head * np.sin(ramp * np.pi / 2)
    return np.concatenate([mixed, body])


def bed(x, seconds, start=0.0):
    """A loop of `seconds` cut from a longer recording."""
    a = int(SR * start)
    n = int(SR * (seconds + 2.0))
    while len(x) < a + n:
        x = np.concatenate([x, x])
    return crossfade_loop(x[a:a + n], 2.0)


def stitch(clips, seconds, gap, seed, gain_range=(0.6, 1.0)):
    """A loop made of clips dropped in at random times, so a creature keeps
    making its noise without an audible pattern."""
    rng = np.random.default_rng(seed)
    n = int(SR * seconds)
    out = np.zeros(n + SR * 3)
    t = int(SR * rng.uniform(0, max(gap[0], 0.05)))
    while t < n:
        c = clips[rng.integers(len(clips))][: SR * 3]
        out[t:t + len(c)] += c * rng.uniform(*gain_range)
        t += len(c) + int(SR * rng.uniform(*gap))
    # Wrap the overhang onto the start so the loop is seamless.
    out[:SR * 3] += out[n:n + SR * 3]
    return out[:n]


def write(key, x, loop=False, lufs=None, db=-1.0):
    """sounds/<key>.ogg: peak-normalized to `db` dBFS, or loudness-matched
    to `lufs` for beds and music."""
    path = os.path.join(OUT, key + ".ogg")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    x = np.asarray(x, dtype=np.float64)
    if not loop:
        x = fade(x)
    x = peak(x, -1.0)
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
        tmp = f.name
    wavfile.write(tmp, SR, (np.clip(x, -1, 1) * 32767).astype(np.int16))
    af = f"loudnorm=I={lufs}:TP=-2:LRA=15" if lufs is not None else f"volume={db + 1.0}dB"
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", tmp, "-af", af, "-ar", str(SR),
                    "-c:a", "libvorbis", "-q:a", "5", path], check=True)
    os.unlink(tmp)


# ---------- the catalog ----------

USED = {}  # key -> [(pack id, file)]


def build(cache):
    def S(key, pid, name, channels=1):
        USED.setdefault(key, []).append((pid, name))
        return decode(find(cache, pid, name), channels)

    def one(key, pid, name, db=-1.0, shift=1.0):
        x = trim(S(key, pid, name))
        if shift != 1.0:
            x = pitch(x, shift)
        write(key, x, db=db)

    # You
    order = ["L1", "R1", "L2", "R2", "L3", "R3"]
    for surface, file in (("grass", "Sand"), ("stone", "Stone")):
        for i, side in enumerate(order):
            one(f"sfx/step_{surface}{i + 1}", "steps", f"ogg/Fantozzi-{file}{side}.ogg", db=-9)
    for i in range(4):
        one(f"sfx/swing{i + 1}", "swishes", f"swish-{i + 1}.wav", db=-4)
    for i in range(3):
        one(f"sfx/hit{i + 1}", "rpg80", f"blade_0{i + 1}.ogg")
        one(f"sfx/hurt{i + 1}", "breaking", f"bfh1_hit_0{i + 1}.ogg", db=-2)
    one("sfx/jump", "tiny", "boots-leather-jump-01.wav", db=-6)
    one("sfx/land", "tiny", "boots-leather-step-01.wav", db=-5)
    one("sfx/wall", "breaking", "bfh1_hit_04.ogg", db=-6)
    one("sfx/bump", "rpg", "inventory/cloth-heavy.wav", db=-6)
    one("sfx/soul", "rpg80", "item_gem_04.ogg", db=-4)
    one("sfx/heal", "rpg80", "spell_01.ogg", db=-4)
    one("sfx/ward", "sfx100", "metal_01.ogg", db=-2)
    one("sfx/thorns", "rpg80", "metal_02.ogg", db=-4)
    one("sfx/ember", "rpg80", "spell_fire_01.ogg", db=-2)
    one("sfx/draw", "rpg", "battle/sword-unsheathe.wav", db=-4)
    one("sfx/buy", "rpg80", "item_coins_01.ogg", db=-3)
    one("sfx/relic", "sfx100", "bell_02.ogg", db=-3)
    # Moves, blocks and the blows they land.
    one("sfx/dash", "swishes", "swish-2.wav", db=-3, shift=1.25)
    one("sfx/heavy", "swishes", "swish-3.wav", db=-2, shift=0.7)
    one("sfx/charged", "rpg", "inventory/metal-ringing.wav", db=-8)
    heavy = trim(S("sfx/heavy_hit", "breaking", "bfh1_metal_hit_02.ogg"))
    blade = trim(S("sfx/heavy_hit", "rpg80", "blade_02.ogg"))
    n = max(len(heavy), len(blade))
    write("sfx/heavy_hit", np.pad(pitch(heavy, 0.8), (0, n))[:n] + 0.7 * np.pad(blade, (0, n))[:n])
    one("sfx/crit", "tiny", "sword-clash-01.wav", db=-3)
    one("sfx/block", "sfx100", "metal_03.ogg", db=-3)
    parry = trim(S("sfx/parry", "tiny", "sword-clash-03.wav"))
    write("sfx/parry", reverb(parry, 0.8, 0.25), db=-1)
    one("sfx/stagger", "rpg80", "creature_hurt_01.ogg", db=-4)

    # Gold, potions and gear.
    one("sfx/coins", "rpg80", "item_coins_02.ogg", db=-5)
    one("sfx/potion", "tiny", "bottle-glass-cork-01.wav", db=-4)
    uncork = trim(S("sfx/drink", "tiny", "vial-glass-uncork-01.wav"))
    bubble = trim(S("sfx/drink", "rpg", "inventory/bubble.wav"))
    write("sfx/drink", np.concatenate([uncork, np.zeros(int(0.08 * SR)), bubble]), db=-4)
    one("sfx/loot_drop", "rpg80", "item_misc_02.ogg", db=-4)
    one("sfx/equip", "rpg", "inventory/armor-light.wav", db=-4)
    one("sfx/merchant", "tiny", "coins-shake-01.wav", db=-5)
    gems = [trim(S("loop/loot", "rpg80", f"item_gem_0{i}.ogg")) * 0.5 for i in range(1, 4)]
    write("loop/loot", stitch(gems, 6, (0.6, 1.4), 21, (0.4, 0.7)), loop=True, db=-12)
    purse = [trim(S("loop/merchant", "tiny", n)) for n in
             ("coins-shake-01.wav", "cloth-pouch-shake-01.wav", "cloth-pouch-shake-02.wav")]
    write("loop/merchant", stitch(purse, 10, (0.8, 2.2), 22), loop=True, db=-8)

    # Pits and spikes.
    deep = lp(S("loop/pit", "spacewind", "space-wind.mp3"), 700)
    write("loop/pit", bed(deep, 12, 20), loop=True, db=-3)
    fall = trim(S("sfx/fall", "breaking", "bfh1_falling_03.ogg"))
    write("sfx/fall", reverb(fall, 1.2, 0.35), db=-2)
    one("sfx/spike_warn", "sfx100", "switch_01.ogg", db=-3)
    one("sfx/spike_up", "rpg", "battle/sword-unsheathe3.wav", db=-2, shift=0.8)

    gate = trim(S("sfx/gate", "sfx100", "bell_01.ogg"))
    write("sfx/gate", reverb(gate, 1.5, 0.3), db=-3)

    # Enemies: a presence loop, three cries before a blow, a death.
    slime = [trim(S("loop/slime", "water", f"slime_{i:02d}.ogg")) for i in range(1, 17)]
    write("loop/slime", stitch(slime, 12, (0.3, 1.1), 1), loop=True, db=-6)
    for i in range(3):
        one(f"sfx/slime_cry{i + 1}", "rpg80", f"creature_slime_0{i + 1}.ogg")
    one("sfx/slime_die", "water", "splash_03.ogg", db=-2)

    moans = [trim(S("loop/shade", "ghost", f"wav/qubodup-GhostMoan0{i}.wav")) for i in range(1, 6)]
    write("loop/shade", stitch([lp(m, 3000) for m in moans], 20, (0.2, 1.5), 2, (0.35, 0.6)),
          loop=True, db=-6)
    for i, n in enumerate((1, 5, 9)):
        one(f"sfx/shade_cry{i + 1}", "rpg", f"NPC/shade/shade{n}.wav")
    one("sfx/shade_die", "rpg80", "creature_die_01.ogg")

    stomp = [pitch(trim(S("loop/ogre", "steps", f"ogg/Fantozzi-Stone{s}.ogg")), 0.6)
             for s in ("L1", "R2", "L3")]
    breath = [trim(S("loop/ogre", "creature80", n)) * 0.5 for n in ("breath.ogg", "snore.ogg")]
    write("loop/ogre", stitch(stomp + breath, 12, (0.35, 0.8), 3), loop=True, db=-4)
    for i, n in enumerate((1, 3, 5)):
        one(f"sfx/ogre_cry{i + 1}", "rpg", f"NPC/ogre/ogre{n}.wav")
    one("sfx/ogre_die", "creature80", "roar_03.ogg", shift=0.8)

    wind = S("loop/wraith", "spacewind", "space-wind.mp3")
    weird = [trim(S("loop/wraith", "creature80", f"weird_0{i}.ogg")) * 0.5 for i in range(1, 6)]
    gust = bed(lp(wind, 2500), 10, 5)
    cries = stitch(weird, len(gust) / SR, (1.2, 2.5), 4)
    write("loop/wraith", cries[: len(gust)] + 0.6 * gust[: len(cries)], loop=True, db=-6)
    for i in range(3):
        one(f"sfx/wraith_cry{i + 1}", "creature80", f"alien_0{i + 1}.ogg")
    one("sfx/wraith_die", "creature80", "scream_01.ogg", db=-3)

    chew = [trim(S("loop/mimic", "creature80", f"eat_0{i}.ogg")) for i in range(1, 5)]
    write("loop/mimic", stitch(chew, 8, (0.2, 0.7), 5), loop=True, db=-5)
    for i in range(3):
        one(f"sfx/mimic_cry{i + 1}", "creature80", f"monster_0{i + 1}.ogg")
    one("sfx/mimic_die", "breaking", "bfh1_breaking_01.ogg")
    one("sfx/mimic", "creature80", "roar_01.ogg")

    # Bolts in flight, and one landing.
    bolt = trim(S("loop/bolt", "rpg80", "spell_fire_03.ogg"))
    write("loop/bolt", crossfade_loop(np.concatenate([bolt, bolt]), 0.4), loop=True, db=-4)
    write("loop/boss_bolt", crossfade_loop(S("loop/boss_bolt", "fire1", "fire-1.ogg"), 0.5),
          loop=True, db=-2)
    one("sfx/bolt_hit", "rpg80", "spell_fire_02.ogg")

    # Wells, altars and chests.
    write("loop/well", crossfade_loop(S("loop/well", "water", "loop_water_01.ogg"), 0.5),
          loop=True, db=-6)
    hum = S("loop/altar", "darkamb", "ambience-1.wav")
    write("loop/altar", crossfade_loop(np.concatenate([hum, hum]), 1.0), loop=True, db=-6)
    chains = [trim(S("loop/chest", "rpg80", f"chain_0{i}.ogg")) for i in range(1, 4)]
    write("loop/chest", stitch(chains, 8, (0.8, 2.0), 6), loop=True, db=-8)
    one("sfx/altar", "rpg", "battle/magic1.wav")
    one("sfx/chest_open", "rpg80", "lock_01.ogg")

    # Bosses.
    one("boss/gong", "sfx100", "gong_01.ogg")
    one("boss/slam", "breaking", "bfh1_rock_breaking_01.ogg")
    one("boss/charge", "creature80", "roar_02.ogg", shift=0.75)
    rumble = [pitch(trim(S("loop/charge", "breaking", f"bfh1_rock_falling_0{i}.ogg")), 0.7)
              for i in range(1, 4)]
    write("loop/charge", stitch(rumble, 4, (0.0, 0.1), 7, (0.8, 1.0)), loop=True, db=-1)
    one("boss/volley", "rpg80", "spell_fire_05.ogg")
    one("boss/summon", "creature80", "howl.ogg", shift=0.8)
    one("boss/vanish", "rpg80", "spell_02.ogg")
    one("boss/appear", "rpg", "battle/spell.wav")

    def boss(name, loop_clips, gap, cry, hurt, die, shift, seed):
        write(f"boss/{name}_loop", stitch(loop_clips, 12, gap, seed), loop=True, db=-4)
        for part, (pid, file) in (("cry", cry), ("hurt", hurt)):
            write(f"boss/{name}_{part}", pitch(trim(S(f"boss/{name}_{part}", pid, file)), shift))
        pid, file = die
        x = pitch(trim(S(f"boss/{name}_die", pid, file)), shift * 0.85)
        write(f"boss/{name}_die", reverb(x, 2.0, 0.4))

    heavy = [pitch(trim(S("boss/shepherd_loop", "creature80", n)), 0.7)
             for n in ("breath.ogg", "snore.ogg")]
    boss("shepherd", heavy + stomp, (0.3, 0.9), ("monster", "Monster-1.wav"),
         ("creature80", "troll_01.ogg"), ("monster", "Monster-2.wav"), 0.8, 11)
    boss("mother", [m * 0.8 for m in moans], (0.0, 0.4), ("creature80", "scream_02.ogg"),
         ("ghost", "wav/qubodup-GhostMoan05.wav"), ("ghost", "wav/qubodup-GhostMoan04.wav"), 1.0, 12)
    water = [trim(S("boss/ferryman_loop", "water", f"splash_{i:02d}.ogg")) * 0.6 for i in (5, 8, 11)]
    boss("ferryman", water + [heavy[0]], (0.4, 1.2), ("monster", "Monster-3.wav"),
         ("monster", "Monster-4.wav"), ("monster", "Monster-6.wav"), 0.75, 13)
    growl = [pitch(trim(S("boss/king_loop", "monster", f"Monster-{i}.wav")), 0.55) * 0.6
             for i in (5, 7)]
    boss("king", growl + heavy, (0.2, 0.8), ("monster", "Monster-8.wav"),
         ("monster", "monster-9.wav"), ("monster", "monster-10.wav"), 0.6, 14)

    # Ambience, one bed per region.
    def amb(key, parts, seconds=60):
        mix = None
        for pid, file, gain, start in parts:
            x = bed(S(key, pid, file), seconds, start) * gain
            mix = x if mix is None else mix[: len(x)] + x[: len(mix)]
        write(key, mix, loop=True, lufs=-30)

    amb("amb/region1", [("park", "park_ambience_birds.wav", 1.0, 30)])
    amb("amb/region2", [("crickets", "crickets_1.mp3", 0.6, 0), ("windy", "windy_ambiance.ogg", 1.0, 20)])
    amb("amb/region3", [("swamp", "swamp.ogg", 1.0, 10), ("drip", "atmosbasement.mp3_.flac", 0.7, 0)])
    amb("amb/region4", [("fire", "fire.wav", 0.8, 0), ("dungeon", "dungeon_ambient_1_0.ogg", 1.0, 0)])

    # Music, kept in stereo and whole.
    def music(key, pid, file, lufs=-22):
        write(key, S(key, pid, file, 2), loop=True, lufs=lufs)

    music("music/title", "m_title", "ambientmain_0.ogg")
    music("music/heaven", "m_heaven", "Heavenly Loop.ogg")
    music("music/region1", "m_explore", "Exploration1 - Grasslands.ogg")
    music("music/region2", "m_evil", "Evil2 - Catacombs.ogg")
    music("music/region3", "m_evil", "Evil4 - Witch's Lair.ogg")
    music("music/region4", "m_evil", "Evil5 - Whispers From Beyond.ogg")
    music("music/boss", "m_boss", "Juhani Junkala - Epic Boss Battle [Seamlessly Looping].wav")
    music("music/king", "m_evil", "Evil3 - Apocalypse.ogg")
    music("music/shop", "m_towns", "Town4 - Bazaar.ogg")
    music("music/summary", "m_towns", "Town2 - Where Time Stands Still.ogg")
    music("music/victory", "m_victory", "Victory1_1.mp3")

    synthesize()


# ---------- the three synthesized sounds ----------


def synthesize():
    rng = np.random.default_rng(7)

    def sweep(f0, f1, sec):
        n = int(SR * sec)
        f = f0 * (f1 / f0) ** np.linspace(0, 1, n)
        return np.sin(2 * np.pi * np.cumsum(f) / SR)

    # The wane: a detuned chord sliding down a fourth.
    n = int(SR * 2.4)
    wane = np.zeros(n)
    for f, det in ((220, 0), (277.2, 0.4), (329.6, -0.3)):
        ff = (f + det) * (0.75 ** np.linspace(0, 1, n))
        wane += np.sin(2 * np.pi * np.cumsum(ff) / SR)
    wane *= np.minimum(1, np.linspace(0, 16, n)) * np.exp(-2.5 * np.linspace(0, 1, n))
    write("sfx/wane", reverb(lp(wane, 2500), 2.0, 0.35), db=-4)
    USED["sfx/wane"] = []

    # Death: a low swell collapsing into a long falling tone.
    n = int(SR * 3.0)
    swell = lp(rng.standard_normal(n), 400) * np.sin(np.linspace(0, np.pi, n)) ** 1.5
    fall = sweep(300, 40, 3.0) * np.exp(-1.2 * np.linspace(0, 1, n))
    write("sfx/death", reverb(swell + 0.5 * fall, 2.5, 0.4), db=-2)
    USED["sfx/death"] = []

    # Rising to heaven: a chord gliding up an octave.
    n = int(SR * 2.5)
    rise = sum(a * sweep(f, f * 2, 2.5) for f, a in ((261.6, 1), (392, 0.6), (523.3, 0.4)))
    write("sfx/ascend", reverb(rise * np.sin(np.linspace(0, np.pi, n)), 2.0, 0.4), db=-4)
    USED["sfx/ascend"] = []


# ---------- credits ----------


def credits():
    by_pack = {}
    for key, uses in sorted(USED.items()):
        for pid, file in uses:
            by_pack.setdefault(pid, {}).setdefault(key, set()).add(file)
    synthesized = ", ".join(k for k, v in sorted(USED.items()) if not v)
    lines = [
        "# Sound and music credits",
        "",
        "Written by `tools/build_sounds.py`, which also builds every file here.",
        "Every source below is CC0 1.0 (public domain) on OpenGameArt.org. CC0 asks",
        "for no credit; it is given anyway so each sound can be traced to its source.",
        "",
        f"Synthesized for Waning, with no source: {synthesized}.",
        "",
    ]
    for pid in PACKS:
        if pid not in by_pack:
            continue
        title, author, slug, _ = PACKS[pid]
        lines += [f"## {title}, by {author}", "", f"{PAGE}{slug}", "",
                  "| Sound | From |", "|---|---|"]
        for key, files in sorted(by_pack[pid].items()):
            lines.append(f"| {key} | {', '.join(sorted(files))} |")
        lines.append("")
    lines += [
        "Each clip was trimmed, mixed to mono (music stays stereo), level-matched and",
        "converted to Ogg Vorbis. Presence loops are built from several clips placed at",
        "random times; some clips are pitched down or given reverb.",
    ]
    with open(os.path.join(OUT, "CREDITS.md"), "w") as f:
        f.write("\n".join(lines) + "\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache", required=True, help="folder for the downloaded packs")
    args = ap.parse_args()
    fetch(args.cache)
    for old in glob.glob(os.path.join(OUT, "**", "*.ogg"), recursive=True):
        os.unlink(old)
    build(args.cache)
    credits()
    print(f"{len(USED)} sounds written to {OUT}")


if __name__ == "__main__":
    main()
