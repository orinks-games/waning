//! A scripted player plays whole runs, to check the curve: strong at the
//! top, weaker each gate, and blessings carry you deeper.

use waning::world::{Ev, Input, Mode, World, AREAS};
use waning::Progress;

/// Walks right, swings at anything in reach, and jumps when an enemy has
/// been winding up for a while, but misses one cry in four, as a player
/// still learning the timing would.
fn bot_run(p: &Progress, seed: u64) -> (usize, u32, bool) {
    let mut w = World::new(p, seed);
    let mut luck = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let mut judged = None;
    let mut dodge = true;
    let dt = 1.0 / 60.0;
    for _ in 0..60 * 600 {
        let mut input = Input {
            walk: 1,
            ..Input::default()
        };
        if let Some((i, d)) = w.nearest() {
            let e = &w.enemies[i];
            if d.abs() < 2.0 {
                input.walk = 0;
                input.attack = true;
            }
            if let Mode::WindUp(t) = e.mode {
                if judged != Some(i) {
                    judged = Some(i);
                    luck = luck
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    dodge = !(luck >> 33).is_multiple_of(4);
                }
                if t < 0.35 && dodge {
                    input.jump = true;
                }
            } else if judged == Some(i) {
                judged = None;
            }
        }
        for ev in w.step(dt, input) {
            match ev {
                Ev::Died => return (w.area, w.souls, false),
                Ev::Won => return (w.area, w.souls, true),
                _ => {}
            }
        }
    }
    panic!("the run never ended: area {}, x {}", w.area, w.x);
}

fn deepest(p: &Progress) -> (f32, f32) {
    let runs: Vec<_> = (0..40).map(|s| bot_run(p, s)).collect();
    let area = runs.iter().map(|r| r.0 as f32).sum::<f32>() / runs.len() as f32;
    let souls = runs.iter().map(|r| r.1 as f32).sum::<f32>() / runs.len() as f32;
    (area, souls)
}

#[test]
fn blessings_carry_you_deeper() {
    let fresh = Progress::default();
    let blessed = Progress {
        might: 3,
        vigor: 3,
        steadfast: 3,
        mercy: 2,
        ..Progress::default()
    };
    let (a0, s0) = deepest(&fresh);
    let (a1, s1) = deepest(&blessed);
    println!("fresh: area {a0:.2}, souls {s0:.1}; blessed: area {a1:.2}, souls {s1:.1}");
    assert!(a0 >= 1.0, "a fresh run should clear the first area: {a0}");
    assert!(
        a0 < (AREAS.len() - 1) as f32,
        "a fresh run should not reach the bottom"
    );
    assert!(
        a1 > a0 + 0.5,
        "blessings should carry the bot deeper: {a0} vs {a1}"
    );
}

#[test]
fn every_gate_takes_a_quarter() {
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
    assert_eq!(w.area, 1);
    assert!((w.power - power * 0.75).abs() < 0.01);
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
    assert!(evs.contains(&Ev::Dodged { i: 0 }), "{evs:?}");
    assert_eq!(w.health, health);
}
