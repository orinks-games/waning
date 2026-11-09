//! Records a run that plays itself to a WAV file, with the game's real
//! sounds and its spoken lines read by the built-in genny voice, so the
//! game can be heard without playing it.
//!
//! ```text
//! cargo run --release --example demo -- OUT.wav [SECONDS] [SEED]
//! ```
//!
//! Run it from the repository folder, so the `sounds` folder is found.

use hearken::audio::{Audio, KiraBackend, LooseClips};
use hearken::{Headless, Key};
use waning::world::World;
use waning::{Play, Progress, Title, Waning};

const RATE: u32 = 44_100;
const FPS: usize = 60;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = args.first().map(String::as_str).unwrap_or("demo.wav");
    let seconds: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(120.0);
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);

    let progress = Progress {
        loaded: true,
        runs: 1,
        ..Progress::default()
    };
    let mut h = Headless::<Waning>::new(progress.clone(), Title::new());
    let backend = KiraBackend::offline(RATE).expect("the offline mixer");
    h.app.ctx.install_audio(Audio::new(
        Box::new(backend),
        Box::new(LooseClips::at("sounds")),
    ));
    let voice = hearken_genny::voice("US male").expect("a genny voice");

    let mut mix: Vec<[f32; 2]> = Vec::new();
    let mut speech: Vec<f32> = Vec::new();
    let mut heard = h.app.ctx.transcript.len();
    let per_frame = RATE as usize / FPS;
    let dt = 1.0 / FPS as f64;

    // A beat of the title, then straight into a descent.
    h.press(Key::Down);
    h.app
        .ctx
        .reset_to(Play::autopilot(World::new(&progress, seed), seed));
    let frames = (seconds * FPS as f64) as usize;
    let mut after_fall = None;
    for n in 0..frames {
        h.frame(dt);
        if let Some(block) = h.app.ctx.audio.render(per_frame) {
            mix.extend(block);
        }
        // New spoken lines go on the speech track at the time they were
        // said, after whatever is still being read.
        let transcript = &h.app.ctx.transcript;
        for line in &transcript[heard.min(transcript.len())..] {
            let text = line.strip_prefix("[event] ").unwrap_or(line);
            let now = mix.len();
            let start = now.max(speech.len());
            // Event chatter that would start long after its moment is dropped,
            // as the game's own event voice does.
            if line.starts_with("[event] ") && start > now + RATE as usize {
                continue;
            }
            let said = hearken_genny::speak(text, &voice, RATE as f64, 1.15);
            speech.resize(start, 0.0);
            speech.extend(said);
        }
        heard = h.app.ctx.transcript.len();
        if h.app.top_name() == "heaven" && after_fall.is_none() {
            after_fall = Some(n);
        }
        if after_fall.is_some_and(|at| n > at + 12 * FPS) {
            break;
        }
    }
    // Let the last lines finish.
    while speech.len() > mix.len() {
        h.frame(dt);
        match h.app.ctx.audio.render(per_frame) {
            Some(block) => mix.extend(block),
            None => break,
        }
    }

    for (i, s) in speech.iter().enumerate() {
        if let Some(f) = mix.get_mut(i) {
            f[0] += 0.8 * s;
            f[1] += 0.8 * s;
        }
    }
    let missing = h.app.ctx.audio.missing_clips();
    if !missing.is_empty() {
        println!("missing sound files: {missing:?}");
    }
    write_wav(out, &mix);
    println!(
        "wrote {out}: {:.1} seconds, {} lines spoken",
        mix.len() as f64 / RATE as f64,
        h.app.ctx.transcript.len()
    );
    for line in &h.app.ctx.transcript {
        println!("  {line}");
    }
}

fn write_wav(path: &str, frames: &[[f32; 2]]) {
    let data_len = (frames.len() * 4) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 4).to_le_bytes());
    out.extend_from_slice(&4u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for f in frames {
        for s in f {
            out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32_767.0) as i16).to_le_bytes());
        }
    }
    std::fs::write(path, out).expect("write the output");
}
