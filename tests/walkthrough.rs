use hearken::{Headless, InputEvent, Key};
use waning::story;
use waning::world::{BossKind, Mode, Relic, World};
use waning::{Heaven, Play, Progress, Title, Waning};

fn events(h: &Headless<Waning>) -> Vec<String> {
    h.app
        .ctx
        .transcript
        .iter()
        .filter_map(|l| l.strip_prefix("[event] ").map(str::to_string))
        .collect()
}

fn hold(h: &mut Headless<Waning>, key: Key, seconds: f64) {
    h.send(InputEvent::key(key));
    let frames = (seconds * 60.0) as usize;
    for _ in 0..frames {
        h.frame(1.0 / 60.0);
    }
    h.send(InputEvent::key_up(key));
}

fn played(h: &Headless<Waning>, key: &str) -> bool {
    h.played().iter().any(|k| k == key)
}

#[test]
fn the_title_leads_to_heaven_and_heaven_to_a_descent() {
    let mut h = Headless::<Waning>::new(Progress::default(), Title::new());
    assert_eq!(h.spoken(), ["Waning.", "Play. 1 of 4."]);
    assert!(played(&h, "music/title"));
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "heaven");
    assert_eq!(
        h.spoken(),
        ["Heaven. 0 grace.", story::OPENING, "Descend. 1 of 7."]
    );
    assert!(played(&h, "music/heaven"));
    h.clear();
    h.press(Key::Down);
    assert_eq!(h.spoken(), ["Might, level 0 of 5, next costs 60. 2 of 7."]);
    h.press(Key::Return);
    assert_eq!(h.spoken().last().unwrap(), "You need 60 more grace.");
    h.press(Key::Home);
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "play");
    assert_eq!(h.spoken(), ["The Bright Fields."]);
    assert!(played(&h, "music/region1"));
    assert!(played(&h, "amb/region1"));
}

#[test]
fn walking_right_steps_and_hears_a_slime_without_being_told() {
    let mut h = Headless::<Waning>::new(Progress::default(), Title::new());
    h.press(Key::Return);
    h.press(Key::Return);
    h.clear();
    hold(&mut h, Key::Right, 3.0);
    let keys = h.played();
    assert!(
        keys.iter().any(|k| k.starts_with("sfx/step_grass")),
        "{keys:?}"
    );
    assert!(played(&h, "loop/slime"), "{keys:?}");
    assert!(
        events(&h).is_empty(),
        "nothing is said about what can be heard: {:?}",
        events(&h)
    );
    h.clear();
    h.press(Key::S);
    let status = &h.spoken()[0];
    assert!(
        status
            .starts_with("Health 100 of 100. Power 60. 1 of 3 potions. 0 gold. 0 souls. The Bright Fields, stage 1 of 12."),
        "{status}"
    );
}

#[test]
fn falling_sends_your_souls_to_heaven_and_the_keeper_speaks() {
    let mut world = World::new(&Progress::default(), 5);
    world.health = 1.0;
    world.souls = 60;
    world.enemies.truncate(1);
    world.enemies[0].x = 1.4;
    world.enemies[0].mode = Mode::WindUp(0.05);
    let p = Progress {
        runs: 1,
        loaded: true,
        heard: vec!["opening".into()],
        ..Progress::default()
    };
    let mut h = Headless::<Waning>::new(p, Play::new(world));
    h.clear();
    for _ in 0..10 {
        h.frame(1.0 / 60.0);
    }
    assert_eq!(h.app.top_name(), "summary");
    assert_eq!(h.app.ctx.game.grace, 60);
    assert_eq!(
        h.spoken()[..2],
        [
            "You fall in the Bright Fields. 60 souls rise with you.",
            "Souls gathered: 60. 1 of 5.",
        ]
    );
    assert!(played(&h, "sfx/death"));
    assert!(played(&h, "music/summary"));
    h.clear();
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "heaven");
    assert_eq!(
        h.spoken()[..2],
        ["Heaven. 60 grace.", story::Beat::FirstFall.line()]
    );
    h.clear();
    h.press(Key::Down);
    h.press(Key::Return);
    assert_eq!(
        h.spoken().last().unwrap(),
        "Might is level 1. 0 grace left."
    );
    assert_eq!(h.app.ctx.game.might, 1);
}

#[test]
fn a_blow_kills_a_slime_at_full_strength_and_pays_souls() {
    let mut world = World::new(&Progress::default(), 2);
    world.enemies.truncate(1);
    world.enemies[0].x = 1.5;
    world.enemies[0].mode = Mode::Approach;
    world.features.clear();
    let mut h = Headless::<Waning>::new(Progress::default(), Play::new(world));
    h.clear();
    h.press(Key::Space);
    h.frame(1.0 / 60.0);
    for key in ["sfx/slime_die", "sfx/soul"] {
        assert!(played(&h, key), "{key} in {:?}", h.played());
    }
    assert!(h.played().iter().any(|k| k.starts_with("sfx/hit")));
    h.clear();
    h.press(Key::D);
    assert_eq!(h.spoken(), ["No enemies left here."]);
}

#[test]
fn a_fallen_boss_offers_a_relic_and_the_run_keeps_it() {
    let mut world = World::new(&Progress::default(), 9);
    // Straight to the Shepherd, nearly beaten, right in front of you.
    while world.stage_info().boss.is_none() {
        world.enemies.clear();
        world.x = world.length();
        world.step(0.01, Default::default());
    }
    let boss = world.boss.as_mut().unwrap();
    assert_eq!(boss.kind, BossKind::Shepherd);
    boss.health = 1.0;
    boss.enraged = true;
    boss.x = 1.5;
    let mut h = Headless::<Waning>::new(Progress::default(), Play::new(world));
    assert!(played(&h, "boss/gong"));
    assert!(h
        .spoken()
        .contains(&story::boss_intro(BossKind::Shepherd).to_string()));
    h.clear();
    h.press(Key::Space);
    h.frame(1.0 / 60.0);
    assert!(played(&h, "boss/shepherd_die"), "{:?}", h.played());
    assert_eq!(h.app.top_name(), "relic choice");
    let said = h.spoken();
    assert_eq!(said[0], story::boss_defeat(BossKind::Shepherd));
    assert_eq!(said[1], "The fallen leave a gift. Choose a relic.");
    assert!(h.app.ctx.game.beaten.contains(&"shepherd".to_string()));
    h.clear();
    h.press(Key::Escape);
    assert_eq!(h.spoken(), ["Choose one with Enter."]);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "play");
    let taken = h.spoken()[1].clone();
    assert!(taken.ends_with(" is yours."), "{taken}");
    let name = taken.trim_end_matches(" is yours.");
    assert!(Relic::ALL.iter().any(|r| r.name() == name));
    // The relic is listed last in the inventory.
    h.press(Key::I);
    assert_eq!(h.app.top_name(), "inventory");
    h.clear();
    h.press(Key::End);
    let last = &h.spoken()[0];
    assert!(last.starts_with(&format!("{name}: ")), "{last}");
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "play");
}

#[test]
fn heaven_remembers_progress_between_screens() {
    let p = Progress {
        grace: 200,
        loaded: true,
        heard: vec!["opening".into()],
        ..Progress::default()
    };
    let mut h = Headless::<Waning>::new(p, Heaven::new());
    assert_eq!(h.spoken()[0], "Heaven. 200 grace.");
    h.press(Key::Down);
    h.press(Key::Down);
    h.press(Key::Return); // Vigor
    assert_eq!(h.app.ctx.game.vigor, 1);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "title");
    h.press(Key::Return);
    assert_eq!(h.spoken().last().unwrap(), "Descend. 1 of 7.");
    assert_eq!(h.app.ctx.game.grace, 140);
}

#[test]
fn every_sound_a_run_plays_has_a_file() {
    let blessed = Progress {
        might: 5,
        vigor: 5,
        steadfast: 5,
        mercy: 3,
        loaded: true,
        heard: vec!["opening".into()],
        ..Progress::default()
    };
    let mut keys = std::collections::BTreeSet::new();
    for seed in 0..3 {
        let mut h = Headless::<Waning>::new(
            blessed.clone(),
            Play::autopilot(World::new(&blessed, seed), seed),
        );
        for _ in 0..60 * 60 * 15 {
            h.frame(1.0 / 60.0);
            keys.extend(h.played());
            h.sounds.clear();
            if h.app.top_name() == "heaven" {
                break;
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sounds");
    let missing: Vec<_> = keys
        .iter()
        .filter(|k| !k.starts_with("ui/"))
        .filter(|k| !root.join(format!("{k}.ogg")).exists())
        .collect();
    assert!(missing.is_empty(), "no file for {missing:?}");
    for key in ["boss/king_loop", "music/king", "sfx/gate", "loop/wraith"] {
        assert!(keys.contains(key), "a full run plays {key}: {keys:?}");
    }
}
