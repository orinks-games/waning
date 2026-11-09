"""Build every sound in sounds/ for Waning.

Two sources, both reproducible from this script:
  * synthesized here with numpy (fixed seeds, so a rerun gives the same files)
  * CC0 clips from "RPG Sound Pack" by artisticdude on OpenGameArt,
    trimmed, level-matched and converted with ffmpeg

Usage:
  python tools/make_sounds.py --pack "<path to RPG Sound Pack folder>"

Needs numpy, scipy and ffmpeg (with libvorbis) on PATH.
"""

import argparse
import os
import subprocess
import tempfile

import numpy as np
from scipy import signal
from scipy.io import wavfile

SR = 44100
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "sounds")


# ---------- building blocks ----------

def t(sec):
    return np.arange(int(SR * sec)) / SR


def env_ad(n, attack, decay_curve=5.0):
    """Linear attack (seconds), exponential decay over the rest."""
    a = max(1, int(SR * attack))
    a = min(a, n)
    e = np.ones(n)
    e[:a] = np.linspace(0, 1, a)
    rest = n - a
    if rest > 0:
        e[a:] = np.exp(-decay_curve * np.linspace(0, 1, rest))
    return e


def fade(x, fin=0.002, fout=0.01):
    x = x.copy()
    i, o = int(SR * fin), int(SR * fout)
    if i:
        x[:i] *= np.linspace(0, 1, i)
    if o:
        x[-o:] *= np.linspace(1, 0, o)
    return x


def lp(x, hz, order=2):
    b, a = signal.butter(order, hz / (SR / 2), "low")
    return signal.lfilter(b, a, x)


def hp(x, hz, order=2):
    b, a = signal.butter(order, hz / (SR / 2), "high")
    return signal.lfilter(b, a, x)


def bp(x, lo, hi, order=2):
    b, a = signal.butter(order, [lo / (SR / 2), hi / (SR / 2)], "band")
    return signal.lfilter(b, a, x)


def pink(n, rng):
    # Voss-style approximation: filter white noise with a -3 dB/octave shelf.
    w = rng.standard_normal(n)
    b = [0.049922035, -0.095993537, 0.050612699, -0.004408786]
    a = [1, -2.494956002, 2.017265875, -0.522189400]
    return signal.lfilter(b, a, w)


def sweep(f0, f1, sec, shape="exp"):
    n = int(SR * sec)
    if shape == "exp":
        f = f0 * (f1 / f0) ** np.linspace(0, 1, n)
    else:
        f = np.linspace(f0, f1, n)
    return np.sin(2 * np.pi * np.cumsum(f) / SR)


def bell(freq, sec, partials=((1, 1.0), (2.76, 0.5), (5.4, 0.25), (8.93, 0.12)), decay=4.0):
    x = np.zeros(int(SR * sec))
    tt = t(sec)
    for ratio, amp in partials:
        x += amp * np.sin(2 * np.pi * freq * ratio * tt) * np.exp(-decay * ratio ** 0.5 * tt)
    return x


def reverb(x, sec=1.2, wet=0.3, seed=1):
    """Cheap convolution reverb: decaying filtered noise as the impulse."""
    rng = np.random.default_rng(seed)
    ir = rng.standard_normal(int(SR * sec)) * np.exp(-6 * np.linspace(0, 1, int(SR * sec)))
    ir = lp(ir, 5000)
    ir /= np.sqrt(np.sum(ir ** 2))
    tail = signal.fftconvolve(x, ir)
    dry = np.concatenate([x, np.zeros(len(tail) - len(x))])
    return (1 - wet) * dry + wet * tail


def loopable(x, xfade=1.0):
    """Fold the last `xfade` seconds over the start so the file loops with no seam."""
    n = int(SR * xfade)
    head, body, tail = x[:n], x[n:-n] if n else x, x[-n:]
    ramp = np.linspace(0, 1, n)
    # equal-power crossfade of the tail into the head
    mixed = tail * np.cos(ramp * np.pi / 2) + head * np.sin(ramp * np.pi / 2)
    return np.concatenate([mixed, body])


def norm(x, peak=0.89):
    m = np.max(np.abs(x))
    return x if m == 0 else x * (peak / m)


def write(key, x, loudness=-18.0, loop=False):
    """Write sounds/<key>.ogg, level-matched with ffmpeg's loudnorm (one pass)."""
    path = os.path.join(OUT, key + ".ogg")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    x = norm(np.asarray(x, dtype=np.float64))
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
        tmp = f.name
    wavfile.write(tmp, SR, (x * 32767).astype(np.int16))
    encode(tmp, path, loudness, loop)
    os.unlink(tmp)


def encode(src, dst, loudness, loop=False, extra_filters=()):
    # Short one-shots are level-matched by peak (loudnorm misreads clips under
    # ~0.4 s); loops and long sounds by integrated loudness.
    filters = list(extra_filters)
    if loop:
        filters.append(f"loudnorm=I={loudness}:TP=-2:LRA=11")
    else:
        filters.append("volume=0dB")
    cmd = ["ffmpeg", "-v", "error", "-y", "-i", src, "-af", ",".join(filters),
           "-ar", str(SR), "-ac", "1", "-c:a", "libvorbis", "-q:a", "5", dst]
    subprocess.run(cmd, check=True)


# ---------- synthesized sounds ----------

def make_synth():
    rng = np.random.default_rng(7)

    # Footsteps: a short filtered noise crunch over a soft low thump, three variants.
    for i in range(3):
        n = int(SR * 0.16)
        crunch = bp(rng.standard_normal(n), 300 + 120 * i, 2400) * env_ad(n, 0.003, 18)
        thump = np.sin(2 * np.pi * (70 + 8 * i) * t(0.16)) * env_ad(n, 0.002, 25)
        write(f"sfx/step{i + 1}", fade(0.6 * crunch + 0.8 * thump))

    # Sword hit: bright metallic ring over a noise crack and a body thump.
    n = int(SR * 0.45)
    crack = hp(rng.standard_normal(n), 1500) * env_ad(n, 0.001, 40)
    ring = bell(1180, 0.45, ((1, 1), (2.41, 0.6), (3.9, 0.3)), decay=9)
    body = np.sin(2 * np.pi * 95 * t(0.45)) * env_ad(n, 0.002, 14)
    write("sfx/hit", fade(0.9 * crack + 0.35 * ring + 0.9 * body))

    # Being hurt: a low dull thud with a falling growl.
    n = int(SR * 0.5)
    thud = lp(rng.standard_normal(n), 400) * env_ad(n, 0.002, 10)
    growl = sweep(180, 60, 0.5) * env_ad(n, 0.01, 6)
    write("sfx/hurt", fade(1.2 * thud + 0.6 * np.tanh(3 * growl)))

    # Jump: a rising breathy whoosh.  Land: a soft heavy thump.
    n = int(SR * 0.3)
    swoosh = bp(rng.standard_normal(n), 400, 3000) * np.sin(np.linspace(0, np.pi, n)) ** 2
    write("sfx/jump", fade(swoosh + 0.2 * sweep(200, 500, 0.3) * np.sin(np.linspace(0, np.pi, n))))
    n = int(SR * 0.25)
    write("sfx/land", fade(lp(rng.standard_normal(n), 300) * env_ad(n, 0.002, 14)
                          + np.sin(2 * np.pi * 55 * t(0.25)) * env_ad(n, 0.002, 12)))

    # Wall: you walked into the edge of the area.
    n = int(SR * 0.2)
    write("sfx/wall", fade(lp(rng.standard_normal(n), 250) * env_ad(n, 0.001, 20)))

    # A soul: two quick rising bell notes.
    s = np.concatenate([bell(880, 0.12, decay=10), bell(1320, 0.6, decay=5)])
    write("sfx/soul", fade(reverb(s, 0.8, 0.25)))

    # An enemy dies: a breathy puff that drops away.
    n = int(SR * 0.6)
    puff = bp(rng.standard_normal(n), 200, 1800) * env_ad(n, 0.01, 6)
    write("sfx/enemy_die", fade(puff + 0.4 * sweep(400, 90, 0.6) * env_ad(n, 0.01, 5)))

    # The gate at the end of an area: a bell chord, pinged while you approach.
    g = bell(523.25, 1.6, decay=2.5) + 0.7 * bell(659.25, 1.6, decay=2.5) + 0.5 * bell(783.99, 1.6, decay=2.5)
    write("sfx/gate", fade(reverb(g, 1.5, 0.3)))

    # The wane: a slow detuned chord sliding down a fourth as power leaves you.
    sec = 2.4
    n = int(SR * sec)
    wane = np.zeros(n)
    for f, det in ((220, 0), (277.2, 0.4), (329.6, -0.3)):
        ff = (f + det) * (0.75 ** np.linspace(0, 1, n))
        wane += np.sin(2 * np.pi * np.cumsum(ff) / SR)
    wane *= env_ad(n, 0.15, 2.5)
    write("sfx/wane", fade(reverb(lp(wane, 2500), 2.0, 0.35), 0.01, 0.2))

    # Death: a low swell that collapses into a long falling tone.
    sec = 3.0
    n = int(SR * sec)
    swell = lp(pink(n, rng), 600) * np.sin(np.linspace(0, np.pi, n)) ** 1.5
    fall = sweep(300, 40, sec) * np.exp(-1.2 * np.linspace(0, 1, n))
    write("sfx/death", fade(reverb(swell + 0.5 * fall, 2.5, 0.4), 0.05, 0.5))

    # Rising into heaven after death, and the bell when you buy a blessing.
    sec = 2.5
    n = int(SR * sec)
    rise = sum(a * sweep(f, f * 2, sec) for f, a in ((261.6, 1), (392, 0.6), (523.3, 0.4)))
    write("sfx/ascend", fade(reverb(rise * np.sin(np.linspace(0, np.pi, n)), 2.0, 0.4), 0.05, 0.4))

    # Enemy presence loops: each enemy keeps making its sound so you can hear
    # where it is.
    # Slime: wet bubbling blips at random times and pitches.
    sec = 4.0
    n = int(SR * sec)
    slime = np.zeros(n)
    for _ in range(26):
        start = rng.integers(0, n - SR // 8)
        f0 = rng.uniform(250, 600)
        blip = sweep(f0, f0 * 1.8, 0.08) * env_ad(int(SR * 0.08), 0.005, 6)
        slime[start:start + len(blip)] += blip * rng.uniform(0.4, 1)
    slime = lp(slime, 1800)
    write("loop/slime", loopable(slime, 0.5), loudness=-20, loop=True)

    # Shade: a breathy whisper that swells and fades.
    sec = 6.0
    n = int(SR * sec)
    breath = bp(rng.standard_normal(n), 1500, 6000)
    lfo = 0.55 + 0.45 * np.sin(2 * np.pi * 0.5 * t(sec)) * np.sin(2 * np.pi * 0.17 * t(sec))
    write("loop/shade", loopable(breath * lfo, 1.0), loudness=-22, loop=True)

    # Ogre: heavy footfalls with a low wheeze between them.
    sec = 4.0
    n = int(SR * sec)
    ogre = 0.25 * lp(rng.standard_normal(n), 300) * (0.5 + 0.5 * np.sin(2 * np.pi * 0.5 * t(sec)))
    for k in range(4):
        st = int(SR * k)
        m = int(SR * 0.4)
        ogre[st:st + m] += (np.sin(2 * np.pi * 45 * t(0.4)) + 0.5 * lp(rng.standard_normal(m), 200)) * env_ad(m, 0.004, 8)
    write("loop/ogre", ogre, loudness=-18, loop=True)

    # The road: wind in grass. Pink noise through a slowly moving band.
    sec = 20.0
    n = int(SR * sec)
    noise = pink(n, rng)
    gust = 0.6 + 0.4 * np.sin(2 * np.pi * 0.07 * t(sec) + 1) * np.sin(2 * np.pi * 0.031 * t(sec))
    wind = 0.6 * lp(noise, 700) * gust + 0.25 * bp(noise, 1500, 4000) * gust ** 2
    write("amb/road", loopable(wind, 3.0), loudness=-28, loop=True)

    # Heaven: a soft choir-like pad of detuned voices on a major chord.
    sec = 16.0
    n = int(SR * sec)
    pad = np.zeros(n)
    for f in (130.81, 196.0, 261.63, 329.63, 392.0):
        for det in (-0.6, 0.0, 0.7):
            ph = rng.uniform(0, 2 * np.pi)
            vib = 0.003 * np.sin(2 * np.pi * rng.uniform(4, 5.5) * t(sec))
            pad += signal.sawtooth(2 * np.pi * np.cumsum((f + det) * (1 + vib)) / SR + ph)
    pad = lp(pad, 1400) * (0.75 + 0.25 * np.sin(2 * np.pi * 0.125 * t(sec)))
    write("amb/heaven", loopable(reverb(pad, 2.5, 0.35)[:n], 3.0), loudness=-26, loop=True)


# ---------- CC0 clips ----------

# (key, file inside the pack)
CC0_CLIPS = [
    ("sfx/swing1", "battle/swing.wav"),
    ("sfx/swing2", "battle/swing2.wav"),
    ("sfx/swing3", "battle/swing3.wav"),
    ("sfx/draw", "battle/sword-unsheathe.wav"),
    ("sfx/buy", "inventory/coin.wav"),
    ("sfx/slime_cry1", "NPC/slime/slime1.wav"),
    ("sfx/slime_cry2", "NPC/slime/slime4.wav"),
    ("sfx/slime_cry3", "NPC/slime/slime7.wav"),
    ("sfx/shade_cry1", "NPC/shade/shade1.wav"),
    ("sfx/shade_cry2", "NPC/shade/shade5.wav"),
    ("sfx/shade_cry3", "NPC/shade/shade9.wav"),
    ("sfx/ogre_cry1", "NPC/ogre/ogre1.wav"),
    ("sfx/ogre_cry2", "NPC/ogre/ogre3.wav"),
    ("sfx/ogre_cry3", "NPC/ogre/ogre5.wav"),
]


def make_cc0(pack):
    for key, rel in CC0_CLIPS:
        src = os.path.join(pack, rel)
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
            tmp = f.name
        # Decode to mono at our rate, then trim and level-match like the
        # synthesized sounds.
        subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", src, "-ac", "1",
                        "-ar", str(SR), tmp], check=True)
        _, x = wavfile.read(tmp)
        os.unlink(tmp)
        x = x.astype(np.float64) / 32768.0
        loud = np.nonzero(np.abs(x) > 0.003)[0]
        if len(loud):
            x = x[max(0, loud[0] - 44):loud[-1] + 441]
        write(key, fade(x, 0.001, 0.015))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pack", required=True, help='the unzipped "RPG Sound Pack" folder')
    args = ap.parse_args()
    make_synth()
    make_cc0(args.pack)
    print("sounds written to", os.path.normpath(OUT))


if __name__ == "__main__":
    main()
