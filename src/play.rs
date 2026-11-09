//! One descent: the world's events become positioned sound and speech.

use hearken::screens::PauseScreen;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};

use crate::heaven::Heaven;
use crate::world::{Ev, Input, World, AREAS, AREA_LENGTH};
use crate::{Control, Title, Waning, HELP};

/// Sounds sit a metre ahead of you, so one level with you is not muffled
/// as if it were behind.
const AHEAD: f32 = 1.0;
/// How far away an enemy starts making its sound.
const HEARING: f32 = 25.0;
/// How often the gate rings while it is near.
const GATE_PING: f64 = 2.0;

pub struct Play {
    world: World,
    /// Each enemy's presence loop, by index.
    loops: Vec<Sound>,
    ambience: Sound,
    gate_in: f64,
    attack: bool,
    jump: bool,
    variant: usize,
    /// Plays by itself, for the demo recording.
    autopilot: Option<Autopilot>,
}

/// A simple player for recordings: walks right, swings at what is in reach,
/// and jumps most cries.
struct Autopilot {
    judged: Option<usize>,
    dodge: bool,
    luck: u64,
}

impl Autopilot {
    fn input(&mut self, w: &World) -> Input {
        let mut input = Input {
            walk: 1,
            ..Input::default()
        };
        if let Some((i, d)) = w.nearest() {
            if d.abs() < 2.0 {
                input.walk = 0;
                input.attack = true;
            }
            if let crate::world::Mode::WindUp(t) = w.enemies[i].mode {
                if self.judged != Some(i) {
                    self.judged = Some(i);
                    self.luck = self
                        .luck
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407);
                    self.dodge = !(self.luck >> 33).is_multiple_of(5);
                }
                input.jump = t < 0.35 && self.dodge;
            } else if self.judged == Some(i) {
                self.judged = None;
            }
        }
        input
    }
}

impl Play {
    pub fn new(world: World) -> Self {
        Play {
            loops: vec![Sound::NONE; world.enemies.len()],
            world,
            ambience: Sound::NONE,
            gate_in: 0.0,
            attack: false,
            jump: false,
            variant: 0,
            autopilot: None,
        }
    }

    /// A run that plays itself, for the demo recording.
    pub fn autopilot(world: World, seed: u64) -> Self {
        Play {
            autopilot: Some(Autopilot {
                judged: None,
                dodge: true,
                luck: seed,
            }),
            ..Play::new(world)
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    fn at(x: f32) -> Vec3 {
        Vec3::new(x, AHEAD, 0.0)
    }

    /// Rotates through a sound's numbered variants so repeats differ.
    fn variant(&mut self, base: &str, n: usize) -> String {
        self.variant = (self.variant + 1) % n;
        format!("{base}{}", self.variant + 1)
    }

    fn area_line(&self) -> String {
        format!(
            "{}. Area {} of {}. The gate is {} metres to the right.",
            sentence(self.world.area_name()),
            self.world.area + 1,
            AREAS.len(),
            AREA_LENGTH as u32
        )
    }

    fn status_line(&self) -> String {
        let w = &self.world;
        format!(
            "Health {} of {}. Power {}. {} souls. {}, {} metres to the gate.",
            shown_health(w.health),
            w.max_health.round() as u32,
            w.power.round() as u32,
            w.souls,
            sentence(w.area_name()),
            (AREA_LENGTH - w.x).max(0.0).ceil() as u32
        )
    }

    fn nearest_line(&self) -> String {
        match self.world.nearest() {
            Some((i, d)) => {
                let e = &self.world.enemies[i];
                let side = if d >= 0.0 { "right" } else { "left" };
                let metres = d.abs().round().max(1.0) as u32;
                let unit = if metres == 1 { "metre" } else { "metres" };
                format!(
                    "{}, {metres} {unit} to the {side}, {} percent health.",
                    e.kind.name(),
                    (100.0 * e.health / e.max_health).ceil() as u32
                )
            }
            None => "No enemies left in this area.".into(),
        }
    }

    fn stop_loops(&mut self, ctx: &mut Ctx<Waning>) {
        for s in self.loops.drain(..) {
            ctx.audio.stop(s, 300);
        }
    }

    /// Starts and moves each living enemy's sound, and stops the sound of
    /// any that died or are out of hearing.
    fn place_loops(&mut self, ctx: &mut Ctx<Waning>) {
        let px = self.world.x;
        for (i, e) in self.world.enemies.iter().enumerate() {
            let near = e.alive && (e.x - px).abs() <= HEARING;
            let s = self.loops[i];
            if near && s == Sound::NONE {
                let key = format!("loop/{}", e.kind.key());
                self.loops[i] = ctx
                    .audio
                    .start_loop(&key, Some(Self::at(e.x)), Bus::EFFECTS);
            } else if near {
                ctx.audio.set_position(s, Some(Self::at(e.x)));
            } else if s != Sound::NONE {
                ctx.audio.stop(s, 400);
                self.loops[i] = Sound::NONE;
            }
        }
    }

    fn handle(&mut self, ctx: &mut Ctx<Waning>, ev: Ev) {
        let w = &self.world;
        match ev {
            Ev::Step => {
                let key = self.variant("sfx/step", 3);
                ctx.audio.play(&key, Bus::EFFECTS);
            }
            Ev::Wall => {
                ctx.audio.play("sfx/wall", Bus::EFFECTS);
            }
            Ev::Swing => {
                let key = self.variant("sfx/swing", 3);
                ctx.audio.play(&key, Bus::EFFECTS);
            }
            Ev::Jump => {
                ctx.audio.play("sfx/jump", Bus::EFFECTS);
            }
            Ev::Land => {
                ctx.audio.play("sfx/land", Bus::EFFECTS);
            }
            Ev::Hit { i, killed } => {
                let x = w.enemies[i].x;
                // A blow sounds as strong as you are: lower and quieter as
                // you wane.
                let strength = (w.power / w.start_power).clamp(0.0, 1.0);
                let hit = ctx.audio.play_at("sfx/hit", Self::at(x), Bus::EFFECTS);
                ctx.audio.set_pitch(hit, 0.6 + 0.4 * strength);
                ctx.audio.set_volume(hit, 0.5 + 0.5 * strength);
                if killed {
                    ctx.audio
                        .play_at("sfx/enemy_die", Self::at(x), Bus::EFFECTS);
                    if let Some(s) = self.loops.get_mut(i) {
                        ctx.audio.stop(*s, 150);
                        *s = Sound::NONE;
                    }
                }
            }
            Ev::Blocked { i } => {
                let name = w.enemies[i].kind.name();
                ctx.say_event(format!("{name} in the way."), false);
            }
            Ev::Noticed { i } => {
                let e = &w.enemies[i];
                let side = if e.x >= w.x { "ahead" } else { "behind" };
                ctx.say_event(format!("{} {side}.", e.kind.name()), false);
            }
            Ev::WindUp { i } => {
                let e = &w.enemies[i];
                let (x, base) = (e.x, format!("sfx/{}_cry", e.kind.key()));
                let key = self.variant(&base, 3);
                ctx.audio.play_at(&key, Self::at(x), Bus::EFFECTS);
            }
            Ev::Struck { .. } => {
                ctx.audio.play("sfx/hurt", Bus::EFFECTS);
                let hp = shown_health(w.health);
                if w.health > 0.0 {
                    ctx.say_event(format!("Health {hp}."), true);
                }
            }
            Ev::Dodged { .. } => {}
            Ev::Souls(_) => {
                ctx.audio.play("sfx/soul", Bus::EFFECTS);
            }
            Ev::Gate { power_before } => {
                self.stop_loops(ctx);
                self.loops = vec![Sound::NONE; self.world.enemies.len()];
                ctx.audio.play("sfx/wane", Bus::EFFECTS);
                let w = &self.world;
                record_depth(ctx, w.area as u32 + 1);
                let line = format!(
                    "Through the gate. Power {} to {}, health {} of {}.",
                    power_before.round() as u32,
                    w.power.round() as u32,
                    shown_health(w.health),
                    w.max_health.round() as u32,
                );
                ctx.say(line);
                let area = self.area_line();
                ctx.say_queued(area);
            }
            Ev::Won => {
                self.finish(ctx, true);
            }
            Ev::Died => {
                self.finish(ctx, false);
            }
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Waning>, won: bool) {
        self.stop_loops(ctx);
        let souls = self.world.souls;
        let where_ = self.world.area_name();
        ctx.game.grace += souls;
        ctx.game.save(&mut ctx.vault);
        let line = if won {
            ctx.audio.play("sfx/gate", Bus::EFFECTS);
            format!("You pass the last gate and reach the bottom of the world. {souls} souls rise with you.")
        } else {
            ctx.audio.play("sfx/death", Bus::EFFECTS);
            format!("You fall in {where_}. {souls} souls rise with you.")
        };
        ctx.replace(Heaven::after_run(line));
    }
}

/// Health as spoken: rounded like the maximum, and never 0 while you live.
fn shown_health(health: f32) -> u32 {
    if health <= 0.0 {
        0
    } else {
        (health.round() as u32).max(1)
    }
}

fn record_depth(ctx: &mut Ctx<Waning>, area: u32) {
    if area > ctx.game.deepest {
        ctx.game.deepest = area;
        ctx.game.save(&mut ctx.vault);
    }
}

/// "the Ash Road" as the start of a sentence.
fn sentence(s: &str) -> String {
    hearken::text::sentence_start(s)
}

fn to_title(ctx: &mut Ctx<Waning>) {
    ctx.reset_to(Title::new());
}

impl State<Waning> for Play {
    fn name(&self) -> &'static str {
        "play"
    }

    fn enter(&mut self, ctx: &mut Ctx<Waning>) {
        self.ambience = ctx.audio.start_loop("amb/road", None, Bus::AMBIENCE);
        ctx.audio.play("sfx/draw", Bus::EFFECTS);
        record_depth(ctx, 1);
        let mut line = self.area_line();
        if ctx.game.runs <= 1 {
            line = format!(
                "You carry all of heaven's strength down with you, and each gate takes some of it. {line} {status} reads your status. F1 has the controls.",
                status = sentence(&ctx.control_name(Control::Status)),
            );
        }
        ctx.say(line);
    }

    fn exit(&mut self, ctx: &mut Ctx<Waning>) {
        self.stop_loops(ctx);
        ctx.audio.stop(self.ambience, 600);
    }

    fn resume(&mut self, ctx: &mut Ctx<Waning>) {
        let line = self.status_line();
        ctx.say(line);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Waning>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        match ctx.action_for(event) {
            Some(Control::Attack) => self.attack = true,
            Some(Control::Jump) => self.jump = true,
            Some(Control::Status) => {
                let line = self.status_line();
                ctx.say(line);
            }
            Some(Control::Nearest) => {
                let line = self.nearest_line();
                ctx.say(line);
            }
            _ => {
                if event.pressed() == Some(Key::Escape) {
                    ctx.push(PauseScreen::<Waning>::new(HELP, to_title));
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Waning>, dt: f64) {
        let walk = match (
            ctx.action_held(Control::Left),
            ctx.action_held(Control::Right),
        ) {
            (true, false) => -1,
            (false, true) => 1,
            _ => 0,
        };
        let mut input = Input {
            walk,
            attack: std::mem::take(&mut self.attack),
            jump: std::mem::take(&mut self.jump),
        };
        if let Some(pilot) = self.autopilot.as_mut() {
            input = pilot.input(&self.world);
        }
        let events = self.world.step(dt as f32, input);
        ctx.audio
            .set_listener(Vec3::new(self.world.x, 0.0, 0.0), 0.0);
        for ev in events {
            let ends = matches!(ev, Ev::Died | Ev::Won);
            self.handle(ctx, ev);
            if ends {
                return;
            }
        }
        self.place_loops(ctx);

        // The gate rings while it is within reach of your ears.
        let to_gate = AREA_LENGTH - self.world.x;
        self.gate_in -= dt;
        if to_gate <= 40.0 && self.gate_in <= 0.0 {
            self.gate_in = GATE_PING;
            ctx.audio
                .play_at("sfx/gate", Self::at(AREA_LENGTH), Bus::EFFECTS);
        }
    }

    fn lines(&self, _ctx: &Ctx<Waning>) -> Vec<String> {
        vec![self.status_line()]
    }

    fn help(&self, ctx: &Ctx<Waning>) -> String {
        let k = |c| ctx.control_name(c);
        format!(
            "{} and {} walk. {} attacks the way you face. {} jumps, over an enemy or out of a blow. \
             When an enemy cries out, it strikes a moment later. {} reads your status, {} the nearest enemy. \
             Escape pauses.",
            sentence(&k(Control::Left)),
            k(Control::Right),
            sentence(&k(Control::Attack)),
            sentence(&k(Control::Jump)),
            sentence(&k(Control::Status)),
            k(Control::Nearest),
        )
    }
}
