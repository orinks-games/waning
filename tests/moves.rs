//! Running, dashing, heavy blows, blocks, pits, potions, gear and the
//! merchant, played against the world directly.

use hearken::{Headless, Key};
use waning::world::{
    Ev, FeatureKind, Input, Kind, Mode, Slot, WareKind, World, HEAVY_CHARGE, MAX_POTIONS,
};
use waning::{Progress, Title, Waning};

const DT: f32 = 1.0 / 60.0;

/// A first area with nothing in it.
fn empty(seed: u64) -> World {
    let mut w = World::new(&Progress::default(), seed);
    w.enemies.clear();
    w.features.clear();
    w.pits.clear();
    w.spikes.clear();
    w
}

fn run_for(w: &mut World, seconds: f32, input: Input) -> Vec<Ev> {
    let mut evs = Vec::new();
    for _ in 0..(seconds / DT) as usize {
        evs.extend(w.step(DT, input));
    }
    evs
}

fn walk_right() -> Input {
    Input {
        walk: 1,
        ..Input::default()
    }
}

#[test]
fn running_covers_more_ground_with_quicker_steps() {
    let mut walker = empty(1);
    let walked = run_for(&mut walker, 2.0, walk_right());
    let mut runner = empty(1);
    let ran = run_for(
        &mut runner,
        2.0,
        Input {
            run: true,
            ..walk_right()
        },
    );
    assert!(runner.x > walker.x * 1.6, "{} vs {}", runner.x, walker.x);
    let steps = |evs: &[Ev]| evs.iter().filter(|e| matches!(e, Ev::Step { .. })).count();
    assert!(steps(&ran) > steps(&walked));
    assert!(ran.contains(&Ev::Step { run: true }));
}

#[test]
fn a_dash_passes_through_an_enemy_and_out_of_its_blow() {
    let mut w = World::new(&Progress::default(), 3);
    w.enemies.truncate(1);
    w.x = 10.0;
    w.enemies[0].x = 11.5;
    w.enemies[0].mode = Mode::WindUp(0.1);
    let health = w.health;
    let mut evs = w.step(
        DT,
        Input {
            dash: true,
            walk: 1,
            ..Input::default()
        },
    );
    evs.extend(run_for(&mut w, 0.5, Input::default()));
    assert!(w.x > 12.5, "dashed past the slime: {}", w.x);
    assert_eq!(w.health, health);
    assert!(evs.contains(&Ev::Dodged), "{evs:?}");
}

#[test]
fn a_held_attack_lands_a_heavy_blow_that_breaks_a_wind_up() {
    let mut w = World::new(&Progress::default(), 3);
    w.enemies.truncate(1);
    w.enemies[0].kind = Kind::Ogre;
    w.enemies[0].health = 1000.0;
    w.enemies[0].max_health = 1000.0;
    w.enemies[0].x = 1.5;
    w.enemies[0].mode = Mode::Recover(5.0);
    let held = Input {
        attack_held: true,
        ..Input::default()
    };
    let mut evs = w.step(
        DT,
        Input {
            attack: true,
            ..held
        },
    );
    evs.extend(run_for(&mut w, HEAVY_CHARGE + 0.05, held));
    assert!(evs.contains(&Ev::Charged), "{evs:?}");
    w.enemies[0].mode = Mode::WindUp(1.0);
    w.enemies[0].x = w.x + 1.5;
    let before = w.enemies[0].health;
    let evs = w.step(DT, Input::default());
    assert!(evs.contains(&Ev::Heavy), "{evs:?}");
    assert!(evs.contains(&Ev::Staggered { i: 0 }), "{evs:?}");
    assert!(
        before - w.enemies[0].health > 2.0 * w.power,
        "a heavy blow hits hard"
    );
}

#[test]
fn a_late_block_takes_part_of_a_blow_and_a_timely_one_all_of_it() {
    for (raised_for, parried) in [(0.1, true), (1.0, false)] {
        let mut w = World::new(&Progress::default(), 3);
        w.enemies.truncate(1);
        w.enemies[0].x = 1.5;
        w.enemies[0].mode = Mode::Recover(10.0);
        let block = Input {
            block: true,
            ..Input::default()
        };
        run_for(&mut w, raised_for, block);
        w.enemies[0].mode = Mode::WindUp(0.01);
        let health = w.health;
        let evs = run_for(&mut w, 0.05, block);
        if parried {
            assert!(evs.contains(&Ev::Parried), "{evs:?}");
            assert_eq!(w.health, health);
            assert!(matches!(w.enemies[0].mode, Mode::Recover(_)));
        } else {
            assert!(evs.contains(&Ev::Blocked), "{evs:?}");
            let taken = health - w.health;
            assert!(taken > 0.0 && taken < w.enemies[0].damage * 0.5, "{taken}");
        }
    }
}

#[test]
fn a_pit_costs_health_unless_you_jump_it() {
    let mut w = empty(4);
    w.pits.push(waning::world::Pit { x: 5.0, width: 2.0 });
    let health = w.health;
    let evs = run_for(&mut w, 2.0, walk_right());
    assert!(evs.contains(&Ev::Fell), "{evs:?}");
    assert!(w.health < health);
    assert!(w.x < 5.0, "climbed back out on the near side: {}", w.x);

    let mut w = empty(4);
    w.pits.push(waning::world::Pit { x: 5.0, width: 2.0 });
    w.x = 4.7;
    let mut evs = w.step(
        DT,
        Input {
            jump: true,
            ..walk_right()
        },
    );
    evs.extend(run_for(&mut w, 1.0, walk_right()));
    assert!(!evs.contains(&Ev::Fell), "{evs:?}");
    assert!(w.x > 7.0);
}

#[test]
fn spikes_click_before_they_rise() {
    let mut w = empty(4);
    w.spikes.push(waning::world::Spikes { x: 0.0, t: 0.0 });
    let evs = run_for(&mut w, 3.5, Input::default());
    let warn = evs.iter().position(|e| matches!(e, Ev::SpikeWarn { .. }));
    let up = evs.iter().position(|e| matches!(e, Ev::SpikeUp { .. }));
    assert!(warn.is_some() && up.is_some() && warn < up, "{evs:?}");
    assert!(evs.iter().any(|e| matches!(e, Ev::Struck { .. })));
}

#[test]
fn a_potion_heals_and_an_empty_flask_says_so() {
    let mut w = empty(1);
    w.health = 30.0;
    let drink = Input {
        drink: true,
        ..Input::default()
    };
    let evs = w.step(DT, drink);
    assert!(evs.contains(&Ev::Drank));
    assert_eq!(w.potions, 0);
    assert!(w.health > 60.0);
    assert!(w.step(DT, drink).contains(&Ev::NoPotions));
}

#[test]
fn an_elite_always_leaves_gear_and_an_empty_slot_takes_it() {
    let mut w = World::new(&Progress::default(), 6);
    w.enemies.truncate(1);
    w.features.clear();
    w.enemies[0] = waning::world::Enemy::new(Kind::Slime, 1.5, 0, true);
    w.enemies[0].health = 1.0;
    let evs = w.step(
        DT,
        Input {
            attack: true,
            ..Input::default()
        },
    );
    assert!(evs.iter().any(|e| matches!(e, Ev::Gold { .. })), "{evs:?}");
    assert_eq!(w.drops.len(), 1, "{evs:?}");
    let slot = w.drops[0].gear.slot;
    let evs = run_for(
        &mut w,
        0.05,
        Input {
            use_: true,
            ..Input::default()
        },
    );
    assert!(evs.iter().any(|e| matches!(e, Ev::Equipped(_))), "{evs:?}");
    assert!(w.worn(slot).is_some());
    assert!(w.drops.is_empty());
}

#[test]
fn the_merchant_sells_potions_for_gold() {
    let mut w = World::new(&Progress::default(), 2);
    // The second area has a merchant.
    w.enemies.clear();
    w.x = w.length();
    w.step(DT, Input::default());
    assert_eq!(w.stage, 1);
    let m = w
        .features
        .iter()
        .find(|f| f.kind == FeatureKind::Merchant)
        .expect("a merchant in the second area")
        .x;
    w.enemies.clear();
    w.pits.clear();
    w.x = m;
    let evs = w.step(
        DT,
        Input {
            use_: true,
            ..Input::default()
        },
    );
    assert!(evs.contains(&Ev::Shop), "{evs:?}");
    let potion = w
        .shop
        .iter()
        .position(|s| s.kind == WareKind::Potion)
        .unwrap();
    assert!(w.buy(potion, &mut Vec::new()).is_err(), "no gold yet");
    w.gold = 500;
    while w.buy(potion, &mut Vec::new()).is_ok() {}
    assert_eq!(w.potions, MAX_POTIONS);
    let gear = w
        .shop
        .iter()
        .position(|s| matches!(s.kind, WareKind::Gear(_)))
        .unwrap();
    assert!(w.buy(gear, &mut Vec::new()).is_ok());
    assert!([Slot::Weapon, Slot::Armor, Slot::Charm]
        .iter()
        .any(|s| w.worn(*s).is_some()));
}

#[test]
fn a_saved_run_comes_back_from_the_title() {
    let mut w = World::new(&Progress::default(), 8);
    w.gold = 77;
    let json = serde_json::to_string(&w).unwrap();
    let back: World = serde_json::from_str(&json).unwrap();
    assert_eq!(back.gold, 77);
    assert_eq!(back.enemies.len(), w.enemies.len());

    // Starting a descent saves it; the title then offers to continue.
    let mut h = Headless::<Waning>::new(Progress::default(), Title::new());
    h.press(Key::Return);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "play");
    h.press(Key::Escape);
    h.clear();
    h.app.ctx.reset_to(Title::new());
    h.frame(DT as f64);
    assert_eq!(h.spoken()[..2], ["Waning.", "Continue descent. 1 of 5."]);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "play");
}
