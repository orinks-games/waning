//! The game's own settings: how long warnings last, and how fast the world
//! moves.

use hearken::menu::{Label, MenuItem};
use hearken::screens::{SettingsRow, SettingsScreen};
use hearken::Ctx;
use serde::{Deserialize, Serialize};

use crate::Waning;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    /// Warnings last 40 percent longer.
    pub longer_warnings: bool,
    /// Percent of normal speed: 100, 85 or 70.
    pub speed: u32,
    /// Run without holding Shift; holding it then walks.
    pub always_run: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            longer_warnings: false,
            speed: 100,
            always_run: false,
        }
    }
}

impl Options {
    /// How much longer than usual each warning lasts.
    pub fn warn(&self) -> f32 {
        if self.longer_warnings {
            1.4
        } else {
            1.0
        }
    }

    pub fn time_scale(&self) -> f64 {
        self.speed.clamp(50, 100) as f64 / 100.0
    }
}

const SPEEDS: [u32; 3] = [100, 85, 70];

fn warnings_value(ctx: &Ctx<Waning>) -> String {
    if ctx.settings.game.longer_warnings {
        "longer".into()
    } else {
        "normal".into()
    }
}

fn speed_value(ctx: &Ctx<Waning>) -> String {
    format!("{} percent", ctx.settings.game.speed)
}

fn running_value(ctx: &Ctx<Waning>) -> String {
    if ctx.settings.game.always_run {
        "on".into()
    } else {
        "off".into()
    }
}

fn toggle_running(ctx: &mut Ctx<Waning>) {
    let o = &mut ctx.settings.game;
    o.always_run = !o.always_run;
    ctx.save_settings();
}

fn toggle_warnings(ctx: &mut Ctx<Waning>) {
    let o = &mut ctx.settings.game;
    o.longer_warnings = !o.longer_warnings;
    ctx.save_settings();
}

fn step_speed(ctx: &mut Ctx<Waning>, delta: i32) {
    let o = &mut ctx.settings.game;
    let at = SPEEDS.iter().position(|&s| s == o.speed).unwrap_or(0) as i32;
    let next = (at + delta).rem_euclid(SPEEDS.len() as i32) as usize;
    o.speed = SPEEDS[next];
    ctx.save_settings();
}

type Screen = SettingsScreen<Waning>;

pub fn rows(mut rows: Vec<SettingsRow<Waning>>) -> Vec<SettingsRow<Waning>> {
    let warnings = MenuItem::new(
        Label::dynamic(|_s: &Screen, ctx: &Ctx<Waning>| {
            format!("Warning time: {}", warnings_value(ctx))
        }),
        |_s: &mut Screen, ctx: &mut Ctx<Waning>| {
            toggle_warnings(ctx);
            ctx.say(format!("Warning time: {}.", warnings_value(ctx)));
        },
    )
    .help("Longer gives you 40 percent more time between a warning and the blow.")
    .adjustable(|_s: &mut Screen, ctx: &mut Ctx<Waning>, _delta| toggle_warnings(ctx));
    let speed = MenuItem::new(
        Label::dynamic(|_s: &Screen, ctx: &Ctx<Waning>| {
            format!("Game speed: {}", speed_value(ctx))
        }),
        |_s: &mut Screen, ctx: &mut Ctx<Waning>| {
            step_speed(ctx, 1);
            ctx.say(format!("Game speed: {}.", speed_value(ctx)));
        },
    )
    .help("Slows the whole world: you, the enemies and every warning.")
    .adjustable(|_s: &mut Screen, ctx: &mut Ctx<Waning>, delta| step_speed(ctx, delta));
    let running = MenuItem::new(
        Label::dynamic(|_s: &Screen, ctx: &Ctx<Waning>| {
            format!("Always run: {}", running_value(ctx))
        }),
        |_s: &mut Screen, ctx: &mut Ctx<Waning>| {
            toggle_running(ctx);
            ctx.say(format!("Always run: {}.", running_value(ctx)));
        },
    )
    .help("On, you run without holding Shift, and holding Shift walks.")
    .adjustable(|_s: &mut Screen, ctx: &mut Ctx<Waning>, _delta| toggle_running(ctx));
    let mut out = vec![
        SettingsRow::new("always run", running).value_first(running_value),
        SettingsRow::new("warnings", warnings).value_first(warnings_value),
        SettingsRow::new("speed", speed).value_first(speed_value),
    ];
    out.append(&mut rows);
    out
}
