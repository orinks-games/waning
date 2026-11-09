use hearken::{Headless, InputEvent, Key};
use waning::world::{Mode, World};
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

#[test]
fn the_title_leads_to_heaven_and_heaven_to_a_descent() {
    let mut h = Headless::<Waning>::new(Progress::default(), Title::new());
    assert_eq!(h.spoken(), ["Waning.", "Play. 1 of 4."]);
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "heaven");
    assert_eq!(h.spoken(), ["Heaven. 0 grace.", "Descend. 1 of 6."]);
    assert!(h.played().contains(&"amb/heaven".to_string()));
    h.clear();
    h.press(Key::Down);
    assert_eq!(h.spoken(), ["Might, level 0 of 5, next costs 25. 2 of 6."]);
    h.press(Key::Return);
    assert_eq!(h.spoken().last().unwrap(), "You need 25 more grace.");
    h.press(Key::Home);
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "play");
    let said = h.spoken();
    assert!(
        said[0].starts_with("You carry all of heaven's strength down with you"),
        "{said:?}"
    );
    assert!(
        said[0].contains("The Bright Fields. Area 1 of 8."),
        "{said:?}"
    );
    assert!(h.played().contains(&"amb/road".to_string()));
}

#[test]
fn walking_right_steps_and_meets_a_slime() {
    let mut h = Headless::<Waning>::new(Progress::default(), Title::new());
    h.press(Key::Return);
    h.press(Key::Return);
    h.clear();
    hold(&mut h, Key::Right, 3.0);
    let played = h.played();
    assert!(
        played.iter().any(|k| k.starts_with("sfx/step")),
        "{played:?}"
    );
    assert!(played.contains(&"loop/slime".to_string()), "{played:?}");
    assert!(
        events(&h).contains(&"Slime ahead.".to_string()),
        "{:?}",
        events(&h)
    );
    h.clear();
    h.press(Key::S);
    let status = &h.spoken()[0];
    assert!(
        status.starts_with("Health 100 of 100. Power 60. 0 souls. The Bright Fields,"),
        "{status}"
    );
}

#[test]
fn falling_sends_your_souls_to_heaven_as_grace() {
    let mut world = World::new(&Progress::default(), 5);
    world.health = 1.0;
    world.souls = 40;
    world.enemies.truncate(1);
    world.enemies[0].x = 1.4;
    world.enemies[0].mode = Mode::WindUp(0.05);
    let mut h = Headless::<Waning>::new(Progress::default(), Play::new(world));
    h.clear();
    for _ in 0..10 {
        h.frame(1.0 / 60.0);
    }
    assert_eq!(h.app.top_name(), "heaven");
    assert_eq!(h.app.ctx.game.grace, 40);
    assert_eq!(
        h.spoken()[..2],
        [
            "You fall in the Bright Fields. 40 souls rise with you.",
            "Heaven. 40 grace."
        ]
    );
    assert!(h.played().contains(&"sfx/death".to_string()));
    h.clear();
    h.press(Key::Down);
    h.press(Key::Return);
    assert_eq!(
        h.spoken().last().unwrap(),
        "Might is level 1. 15 grace left."
    );
    assert_eq!(h.app.ctx.game.might, 1);
}

#[test]
fn a_blow_kills_a_slime_at_full_strength_and_pays_souls() {
    let mut world = World::new(&Progress::default(), 2);
    world.enemies.truncate(1);
    world.enemies[0].x = 1.5;
    world.enemies[0].mode = Mode::Approach;
    let mut h = Headless::<Waning>::new(Progress::default(), Play::new(world));
    h.clear();
    h.press(Key::Space);
    h.frame(1.0 / 60.0);
    let played = h.played();
    for key in ["sfx/hit", "sfx/enemy_die", "sfx/soul"] {
        assert!(played.contains(&key.to_string()), "{key} in {played:?}");
    }
    h.clear();
    h.press(Key::D);
    assert_eq!(h.spoken(), ["No enemies left in this area."]);
}

#[test]
fn heaven_remembers_progress_between_screens() {
    let p = Progress {
        grace: 100,
        loaded: true,
        ..Progress::default()
    };
    let mut h = Headless::<Waning>::new(p, Heaven::new());
    assert_eq!(h.spoken()[0], "Heaven. 100 grace.");
    h.press(Key::Down);
    h.press(Key::Down);
    h.press(Key::Return); // Vigor
    assert_eq!(h.app.ctx.game.vigor, 1);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "title");
    h.press(Key::Return);
    assert_eq!(h.spoken().last().unwrap(), "Descend. 1 of 6.");
    assert_eq!(h.app.ctx.game.grace, 75);
}
