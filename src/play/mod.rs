//! One descent: the world's events become positioned sound and speech.

use std::collections::HashMap;

use hearken::screens::PauseScreen;
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};

mod events;
mod loops;

use crate::bag::Inventory;
use crate::progress::save_run;
use crate::story;
use crate::world::{BossKind, Ev, Input, World, MAX_POTIONS, STAGES};
use crate::{music, Control, Progress, Title, Waning, HELP};

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
    bolts: HashMap<u32, Sound>,
    /// The shimmer of each piece of gear on the ground, by id.
    drops: HashMap<u32, Sound>,
    /// The wind up out of each pit, and the sound of each well, altar,
    /// chest or merchant, by index.
    pits: Vec<Sound>,
    features: Vec<Sound>,
    boss_loop: Sound,
    charge_loop: Sound,
    ambience: Sound,
    /// Which region's music and ambience are playing, and whether it is
    /// the boss music.
    playing: Option<(usize, bool)>,
    gate_in: f64,
    attack: bool,
    jump: bool,
    use_: bool,
    dash: bool,
    drink: bool,
    variant: usize,
    stride: usize,
    /// Plays by itself, for the demo recording.
    autopilot: Option<crate::autopilot::Autopilot>,
}

impl Play {
    pub fn new(world: World) -> Self {
        Play {
            loops: Vec::new(),
            bolts: HashMap::new(),
            drops: HashMap::new(),
            pits: Vec::new(),
            features: Vec::new(),
            world,
            boss_loop: Sound::NONE,
            charge_loop: Sound::NONE,
            ambience: Sound::NONE,
            playing: None,
            gate_in: 0.0,
            attack: false,
            jump: false,
            use_: false,
            dash: false,
            drink: false,
            variant: 0,
            stride: 0,
            autopilot: None,
        }
    }

    /// A run that plays itself, for the demo recording.
    pub fn autopilot(world: World, seed: u64) -> Self {
        Play {
            autopilot: Some(crate::autopilot::Autopilot::new(seed)),
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

    fn stage_line(&self) -> String {
        format!("{}.", sentence_start(self.world.stage_name()))
    }

    fn status_line(&self) -> String {
        let w = &self.world;
        let mut line = format!(
            "Health {} of {}. Power {}. {} of {MAX_POTIONS} potions. {} gold. {} souls. {}, stage {} of {}.",
            shown_health(w.health),
            w.max_health.round() as u32,
            w.power.round() as u32,
            w.potions,
            w.gold,
            w.souls,
            sentence_start(w.stage_name()),
            w.stage + 1,
            STAGES.len(),
        );
        if w.gate_open() {
            let metres = (w.length() - w.x).max(0.0).ceil() as u32;
            line.push_str(&format!(
                " The gate is {metres} {} to the right.",
                metres_word(metres)
            ));
        }
        for f in w.features.iter().filter(|f| !f.used) {
            line.push_str(&format!(" {}.", place(f.kind.name(), f.x - w.x)));
        }
        if let Some(d) = w
            .drops
            .iter()
            .min_by(|a, b| (a.x - w.x).abs().total_cmp(&(b.x - w.x).abs()))
        {
            line.push_str(&format!(" {}.", place("Gear", d.x - w.x)));
        }
        line
    }

    fn nearest_line(&self) -> String {
        let w = &self.world;
        let boss = w.boss.as_ref().filter(|b| b.alive).map(|b| {
            (
                b.kind.name().to_string(),
                b.x - w.x,
                b.health / b.max_health,
                b.present(),
            )
        });
        let enemy = w.nearest().map(|(i, d)| {
            let e = &w.enemies[i];
            (e.name(), d, e.health / e.max_health, true)
        });
        let pick = match (boss, enemy) {
            (Some(b), Some(e)) if e.1.abs() < b.1.abs() => Some(e),
            (Some(b), _) => Some(b),
            (None, e) => e,
        };
        match pick {
            Some((name, _, health, false)) => format!(
                "{name} has vanished, {} percent health.",
                (100.0 * health).ceil() as u32
            ),
            Some((name, d, health, true)) => format!(
                "{}, {} percent health.",
                place(&name, d),
                (100.0 * health).ceil() as u32
            ),
            None => "No enemies left here.".into(),
        }
    }

    // ---- sound upkeep ----

    /// The region's music and ambience, or the boss music in an arena.
    fn set_scene(&mut self, ctx: &mut Ctx<Waning>) {
        let region = self.world.region();
        let boss = self.world.stage_info().boss;
        let want = (region, boss.is_some());
        if self.playing == Some(want) {
            return;
        }
        let track = match boss {
            Some(BossKind::King) => "music/king".to_string(),
            Some(_) => "music/boss".to_string(),
            None => format!("music/region{}", region + 1),
        };
        music::play(ctx, &track);
        if self.playing.map(|p| p.0) != Some(region) {
            ctx.audio.stop(self.ambience, 1500);
            let amb = format!("amb/region{}", region + 1);
            self.ambience = ctx.audio.start_loop(&amb, None, Bus::AMBIENCE);
        }
        self.playing = Some(want);
    }

    fn stop_stage_sounds(&mut self, ctx: &mut Ctx<Waning>) {
        for s in self.loops.drain(..) {
            ctx.audio.stop(s, 300);
        }
        for (_, s) in self.bolts.drain().chain(self.drops.drain()) {
            ctx.audio.stop(s, 100);
        }
        for s in self.pits.drain(..).chain(self.features.drain(..)) {
            ctx.audio.stop(s, 300);
        }
        for s in [&mut self.boss_loop, &mut self.charge_loop] {
            ctx.audio.stop(*s, 300);
            *s = Sound::NONE;
        }
    }

    /// The start of a stage: its name, and for an arena the boss's story.
    fn begin_stage(&mut self, ctx: &mut Ctx<Waning>) {
        self.set_scene(ctx);
        if self.autopilot.is_none() {
            save_run(&mut ctx.vault, &self.world);
        }
        if let Some(kind) = self.world.stage_info().boss {
            ctx.audio.play("boss/gong", Bus::EFFECTS);
            Progress::mark(&mut ctx.game.met, kind.key());
            ctx.game.save(&mut ctx.vault);
            ctx.say_queued(story::boss_intro(kind));
        }
    }
}

/// "Well, 12 metres to the right."
fn place(name: &str, d: f32) -> String {
    let metres = d.abs().round().max(1.0) as u32;
    let side = if d >= 0.0 { "right" } else { "left" };
    format!("{name}, {metres} {} to the {side}", metres_word(metres))
}

fn metres_word(n: u32) -> &'static str {
    if n == 1 {
        "metre"
    } else {
        "metres"
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

fn record_depth(ctx: &mut Ctx<Waning>, stage: u32) {
    if stage > ctx.game.deepest {
        ctx.game.deepest = stage;
        ctx.game.save(&mut ctx.vault);
    }
}

fn to_title(ctx: &mut Ctx<Waning>) {
    ctx.reset_to(Title::new());
}

impl State<Waning> for Play {
    fn name(&self) -> &'static str {
        "play"
    }

    fn enter(&mut self, ctx: &mut Ctx<Waning>) {
        ctx.audio.play("sfx/draw", Bus::EFFECTS);
        record_depth(ctx, self.world.stage as u32 + 1);
        let line = self.stage_line();
        ctx.say(line);
        self.begin_stage(ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Waning>) {
        self.stop_stage_sounds(ctx);
        ctx.audio.stop(self.ambience, 800);
    }

    fn resume(&mut self, ctx: &mut Ctx<Waning>) {
        let line = self.status_line();
        ctx.say(line);
    }

    /// Back from choosing a relic or gear, or from the merchant.
    fn resume_with(&mut self, ctx: &mut Ctx<Waning>, result: &str) {
        if let Some(r) = ctx.game.picked.take() {
            self.world.take(r);
        }
        if let Some(w) = ctx.game.world_back.take() {
            self.world = *w;
        }
        if let Some(id) = ctx.game.chosen_gear.take() {
            let mut out = Vec::new();
            self.world.equip(id, &mut out);
            ctx.audio.play("sfx/equip", Bus::EFFECTS);
            for ev in out {
                self.handle(ctx, ev);
            }
        }
        ctx.say(hearken::text::end_sentence(result));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Waning>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        match ctx.action_for(event) {
            Some(Control::Attack) => self.attack = true,
            Some(Control::Jump) => self.jump = true,
            Some(Control::Use) => self.use_ = true,
            Some(Control::Dash) => self.dash = true,
            Some(Control::Drink) => self.drink = true,
            Some(Control::Status) => {
                let line = self.status_line();
                ctx.say(line);
            }
            Some(Control::Nearest) => {
                let line = self.nearest_line();
                ctx.say(line);
            }
            Some(Control::Inventory) => ctx.push(Inventory::new(&self.world)),
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
        let options = &ctx.settings.game;
        let dt = dt * options.time_scale();
        self.world.warn = options.warn();
        let run = ctx.held.mods().shift != options.always_run;
        let mut input = Input {
            walk,
            run,
            attack: std::mem::take(&mut self.attack),
            attack_held: ctx.action_held(Control::Attack),
            jump: std::mem::take(&mut self.jump),
            dash: std::mem::take(&mut self.dash),
            block: ctx.action_held(Control::Block),
            drink: std::mem::take(&mut self.drink),
            use_: std::mem::take(&mut self.use_),
        };
        if let Some(pilot) = self.autopilot.as_mut() {
            if let Some(r) = pilot.take_pick() {
                self.world.take(r);
            }
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

        // The gate rings while it is near and open.
        let w = &self.world;
        let to_gate = w.length() - w.x;
        self.gate_in -= dt;
        if w.gate_open() && to_gate <= 40.0 && self.gate_in <= 0.0 {
            self.gate_in = GATE_PING;
            ctx.audio
                .play_at("sfx/gate", Self::at(w.length()), Bus::EFFECTS);
        }
    }

    fn lines(&self, _ctx: &Ctx<Waning>) -> Vec<String> {
        vec![self.status_line()]
    }

    fn help(&self, ctx: &Ctx<Waning>) -> String {
        let k = |c| ctx.control_name(c);
        format!(
            "{} and {} walk; hold Shift to run. {} attacks the way you face; hold it for a heavy blow. \
             {} jumps, {} dashes, and holding {} blocks. {} drinks a potion. \
             {} uses a well, altar, chest or merchant, or picks up gear. \
             {} reads your status, {} the nearest enemy, {} your inventory. Escape pauses.",
            sentence_start(&k(Control::Left)),
            k(Control::Right),
            sentence_start(&k(Control::Attack)),
            sentence_start(&k(Control::Jump)),
            k(Control::Dash),
            k(Control::Block),
            sentence_start(&k(Control::Drink)),
            sentence_start(&k(Control::Use)),
            sentence_start(&k(Control::Status)),
            k(Control::Nearest),
            k(Control::Inventory),
        )
    }
}
