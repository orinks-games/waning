//! A scripted player plays whole runs, to check the curve: strong at the
//! top, weaker each gate, and blessings carry you deeper.

use waning::autopilot::Autopilot;
use waning::world::{Ev, Input, Mode, World, STAGES};
use waning::Progress;

/// (stage reached, souls, won)
fn bot_run(p: &Progress, seed: u64, miss_one_in: u64) -> (usize, u32, bool) {
    let mut w = World::new(p, seed);
    let mut bot = Autopilot::new(seed.wrapping_mul(7919));
    bot.miss_one_in = miss_one_in;
    let dt = 1.0 / 60.0;
    for _ in 0..60 * 60 * 30 {
        if let Some(r) = bot.take_pick() {
            w.take(r);
        }
        let input = bot.input(&w);
        for ev in w.step(dt, input) {
            match ev {
                Ev::Died => return (w.stage, w.souls, false),
                Ev::Won => return (w.stage, w.souls, true),
                Ev::RelicOffer(offer) => bot.offered(offer),
                _ => {}
            }
        }
    }
    panic!("the run never ended: stage {}, x {}", w.stage, w.x);
}

fn average(p: &Progress, miss: u64) -> (f32, f32, f32) {
    let runs: Vec<_> = (0..40).map(|s| bot_run(p, s, miss)).collect();
    let n = runs.len() as f32;
    let stage = runs.iter().map(|r| r.0 as f32).sum::<f32>() / n;
    let souls = runs.iter().map(|r| r.1 as f32).sum::<f32>() / n;
    let wins = runs.iter().filter(|r| r.2).count() as f32 / n;
    (stage, souls, wins)
}

fn blessed(level: u32) -> Progress {
    Progress {
        might: level,
        vigor: level,
        steadfast: level,
        mercy: level.min(3),
        ..Progress::default()
    }
}

#[test]
fn blessings_carry_you_deeper() {
    let mut last = -1.0;
    for level in [0, 2, 5] {
        let (stage, souls, wins) = average(&blessed(level), 4);
        println!(
            "blessings {level}: stage {stage:.2}, souls {souls:.0}, wins {:.0}%",
            wins * 100.0
        );
        assert!(stage > last, "level {level} went no deeper");
        last = stage;
        if level == 0 {
            assert!(stage >= 1.0, "a fresh run should clear the first area");
            assert!(wins < 0.05, "a fresh run should almost never win");
        }
        if level == 5 {
            assert!(wins > 0.3, "full blessings should win often");
        }
    }
    assert!(last <= (STAGES.len() - 1) as f32);
}

#[test]
fn every_gate_takes_its_share() {
    let mut w = World::new(&Progress::default(), 1);
    let power = w.power;
    w.enemies.clear();
    let mut gate = false;
    for _ in 0..60 * 30 {
        for ev in w.step(
            1.0 / 60.0,
            Input {
                walk: 1,
                ..Input::default()
            },
        ) {
            if matches!(ev, Ev::Gate { .. }) {
                gate = true;
            }
        }
        if gate {
            break;
        }
    }
    assert!(gate);
    assert_eq!(w.stage, 1);
    assert!((w.power - power * 0.85).abs() < 0.01);
}

#[test]
fn a_jump_dodges_a_blow() {
    let mut w = World::new(&Progress::default(), 3);
    w.enemies.truncate(1);
    w.enemies[0].x = 1.5;
    w.enemies[0].mode = Mode::WindUp(0.2);
    let health = w.health;
    let mut evs = w.step(
        0.01,
        Input {
            jump: true,
            ..Input::default()
        },
    );
    for _ in 0..30 {
        evs.extend(w.step(0.01, Input::default()));
    }
    assert!(evs.contains(&Ev::Dodged), "{evs:?}");
    assert_eq!(w.health, health);
}
