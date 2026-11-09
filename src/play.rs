//! One descent: the world's events become positioned sound and speech.

use std::collections::HashMap;

use hearken::screens::PauseScreen;
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};

use crate::heaven::Heaven;
use crate::relics::RelicChoice;
use crate::story;
use crate::world::{Attack, BossKind, BossMode, Ev, Input, Target, World, STAGES};
use crate::{Control, Progress, Title, Waning, HELP};

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
    boss_loop: Sound,
    charge_loop: Sound,
    feature_loop: Sound,
    music: Sound,
    ambience: Sound,
    /// Which region's music and ambience are playing, and whether it is
    /// the boss music.
    playing: Option<(usize, bool)>,
    gate_in: f64,
    attack: bool,
    jump: bool,
    use_: bool,
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
            world,
            boss_loop: Sound::NONE,
            charge_loop: Sound::NONE,
            feature_loop: Sound::NONE,
            music: Sound::NONE,
            ambience: Sound::NONE,
            playing: None,
            gate_in: 0.0,
            attack: false,
            jump: false,
            use_: false,
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
            "Health {} of {}. Power {}. {} souls. {}, stage {} of {}.",
            shown_health(w.health),
            w.max_health.round() as u32,
            w.power.round() as u32,
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
        if let Some(f) = w.feature.as_ref().filter(|f| !f.used) {
            line.push_str(&format!(" {}.", place(f.kind.name(), f.x - w.x)));
        }
        line
    }

    fn nearest_line(&self) -> String {
        let w = &self.world;
        let boss = w.boss.as_ref().filter(|b| b.alive).map(|b| {
            (
                b.kind.name(),
                b.x - w.x,
                b.health / b.max_health,
                b.present(),
            )
        });
        let enemy = w.nearest().map(|(i, d)| {
            let e = &w.enemies[i];
            (e.kind.name(), d, e.health / e.max_health, true)
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
                place(name, d),
                (100.0 * health).ceil() as u32
            ),
            None => "No enemies left here.".into(),
        }
    }

    fn relics_line(&self) -> String {
        let names: Vec<&str> = self.world.relics.iter().map(|r| r.name()).collect();
        if names.is_empty() {
            "No relics yet.".into()
        } else {
            format!("Relics: {}.", names.join(", "))
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
        ctx.audio.stop(self.music, 1500);
        let music = match boss {
            Some(BossKind::King) => "music/king".to_string(),
            Some(_) => "music/boss".to_string(),
            None => format!("music/region{}", region + 1),
        };
        self.music = ctx.audio.start_loop(&music, None, Bus::MUSIC);
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
        for (_, s) in self.bolts.drain() {
            ctx.audio.stop(s, 100);
        }
        for s in [
            &mut self.boss_loop,
            &mut self.charge_loop,
            &mut self.feature_loop,
        ] {
            ctx.audio.stop(*s, 300);
            *s = Sound::NONE;
        }
    }

    /// The start of a stage: its name, and for an arena the boss's story.
    fn begin_stage(&mut self, ctx: &mut Ctx<Waning>) {
        self.set_scene(ctx);
        if let Some(kind) = self.world.stage_info().boss {
            ctx.audio.play("boss/gong", Bus::EFFECTS);
            Progress::mark(&mut ctx.game.met, kind.key());
            ctx.game.save(&mut ctx.vault);
            ctx.say_queued(story::boss_intro(kind));
        }
    }

    /// Starts, moves and stops every positioned loop to match the world.
    fn place_loops(&mut self, ctx: &mut Ctx<Waning>) {
        let w = &self.world;
        let px = w.x;
        self.loops.resize(w.enemies.len(), Sound::NONE);
        for (i, e) in w.enemies.iter().enumerate() {
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
        for b in &w.bolts {
            let key = if b.from_boss {
                "loop/boss_bolt"
            } else {
                "loop/bolt"
            };
            let s = *self
                .bolts
                .entry(b.id)
                .or_insert_with(|| ctx.audio.start_loop(key, Some(Self::at(b.x)), Bus::EFFECTS));
            ctx.audio.set_position(s, Some(Self::at(b.x)));
        }
        match w.boss.as_ref() {
            Some(b) if b.present() => {
                if self.boss_loop == Sound::NONE {
                    let key = format!("boss/{}_loop", b.kind.key());
                    self.boss_loop = ctx
                        .audio
                        .start_loop(&key, Some(Self::at(b.x)), Bus::EFFECTS);
                }
                ctx.audio.set_position(self.boss_loop, Some(Self::at(b.x)));
                let charging = matches!(b.mode, BossMode::Charging { .. });
                if charging && self.charge_loop == Sound::NONE {
                    self.charge_loop =
                        ctx.audio
                            .start_loop("loop/charge", Some(Self::at(b.x)), Bus::EFFECTS);
                } else if charging {
                    ctx.audio
                        .set_position(self.charge_loop, Some(Self::at(b.x)));
                } else if self.charge_loop != Sound::NONE {
                    ctx.audio.stop(self.charge_loop, 250);
                    self.charge_loop = Sound::NONE;
                }
            }
            _ => {
                for s in [&mut self.boss_loop, &mut self.charge_loop] {
                    if *s != Sound::NONE {
                        ctx.audio.stop(*s, 300);
                        *s = Sound::NONE;
                    }
                }
            }
        }
        match w.feature.as_ref() {
            Some(f) if !f.used && (f.x - px).abs() <= HEARING => {
                if self.feature_loop == Sound::NONE {
                    let key = format!("loop/{}", f.kind.key());
                    self.feature_loop =
                        ctx.audio
                            .start_loop(&key, Some(Self::at(f.x)), Bus::EFFECTS);
                }
            }
            _ => {
                if self.feature_loop != Sound::NONE {
                    ctx.audio.stop(self.feature_loop, 400);
                    self.feature_loop = Sound::NONE;
                }
            }
        }
    }

    fn target_x(&self, t: Target) -> f32 {
        match t {
            Target::Enemy(i) => self.world.enemies[i].x,
            Target::Boss => self.world.boss.as_ref().map_or(self.world.x, |b| b.x),
        }
    }

    fn handle(&mut self, ctx: &mut Ctx<Waning>, ev: Ev) {
        match ev {
            Ev::Step => {
                let surface = if self.world.region() < 2 {
                    "grass"
                } else {
                    "stone"
                };
                self.stride = (self.stride + 1) % 6;
                let key = format!("sfx/step_{surface}{}", self.stride + 1);
                ctx.audio.play(&key, Bus::EFFECTS);
            }
            Ev::Wall => {
                ctx.audio.play("sfx/wall", Bus::EFFECTS);
            }
            Ev::Bump => {
                ctx.audio.play("sfx/bump", Bus::EFFECTS);
            }
            Ev::Swing => {
                let key = self.variant("sfx/swing", 4);
                ctx.audio.play(&key, Bus::EFFECTS);
            }
            Ev::Jump => {
                ctx.audio.play("sfx/jump", Bus::EFFECTS);
            }
            Ev::Land => {
                ctx.audio.play("sfx/land", Bus::EFFECTS);
            }
            Ev::Hit { target, killed } => {
                let x = self.target_x(target);
                // A blow sounds as strong as you are: lower and quieter as
                // you wane.
                let w = &self.world;
                let strength = (w.power / w.start_power).clamp(0.0, 1.0);
                let key = self.variant("sfx/hit", 3);
                let hit = ctx.audio.play_at(&key, Self::at(x), Bus::EFFECTS);
                ctx.audio.set_pitch(hit, 0.7 + 0.3 * strength);
                ctx.audio.set_volume(hit, 0.55 + 0.45 * strength);
                match target {
                    Target::Enemy(i) if killed => {
                        let key = format!("sfx/{}_die", self.world.enemies[i].kind.key());
                        ctx.audio.play_at(&key, Self::at(x), Bus::EFFECTS);
                        if let Some(s) = self.loops.get_mut(i) {
                            ctx.audio.stop(*s, 150);
                            *s = Sound::NONE;
                        }
                    }
                    Target::Boss if !killed => {
                        if let Some(b) = self.world.boss.as_ref() {
                            let key = format!("boss/{}_hurt", b.kind.key());
                            ctx.audio.play_at(&key, Self::at(x), Bus::EFFECTS);
                        }
                    }
                    _ => {}
                }
            }
            Ev::WindUp { i } => {
                let e = &self.world.enemies[i];
                let (x, base) = (e.x, format!("sfx/{}_cry", e.kind.key()));
                let key = self.variant(&base, 3);
                ctx.audio.play_at(&key, Self::at(x), Bus::EFFECTS);
            }
            Ev::Struck { .. } => {
                let key = self.variant("sfx/hurt", 3);
                ctx.audio.play(&key, Bus::EFFECTS);
                if self.world.health > 0.0 {
                    ctx.say_event(format!("Health {}.", shown_health(self.world.health)), true);
                }
            }
            Ev::Dodged => {}
            Ev::Warded => {
                ctx.audio.play("sfx/ward", Bus::EFFECTS);
            }
            Ev::Thorns => {
                ctx.audio.play("sfx/thorns", Bus::EFFECTS);
            }
            Ev::Ember => {
                ctx.audio.play("sfx/ember", Bus::EFFECTS);
            }
            Ev::Souls(_) => {
                ctx.audio.play("sfx/soul", Bus::EFFECTS);
            }
            Ev::Healed(_) => {
                ctx.audio.play("sfx/heal", Bus::EFFECTS);
                ctx.say_event(
                    format!("Health {}.", shown_health(self.world.health)),
                    false,
                );
            }
            Ev::Fired { .. } => {}
            Ev::BoltGone { id, x, hit } => {
                if let Some(s) = self.bolts.remove(&id) {
                    ctx.audio.stop(s, 80);
                }
                if hit {
                    ctx.audio.play_at("sfx/bolt_hit", Self::at(x), Bus::EFFECTS);
                }
            }
            Ev::BossWindUp(attack) => {
                let Some(b) = self.world.boss.as_ref() else {
                    return;
                };
                let key = match attack {
                    Attack::Slam => format!("boss/{}_cry", b.kind.key()),
                    other => format!("boss/{}", other.key()),
                };
                ctx.audio.play_at(&key, Self::at(b.x), Bus::EFFECTS);
            }
            Ev::BossSlam => {
                let x = self.target_x(Target::Boss);
                ctx.audio.play_at("boss/slam", Self::at(x), Bus::EFFECTS);
            }
            Ev::BossChargeEnd => {
                let x = self.target_x(Target::Boss);
                ctx.audio.play_at("boss/slam", Self::at(x), Bus::EFFECTS);
            }
            Ev::BossVanish { x } => {
                ctx.audio.play_at("boss/vanish", Self::at(x), Bus::EFFECTS);
            }
            Ev::BossAppear { x } => {
                ctx.audio.play_at("boss/appear", Self::at(x), Bus::EFFECTS);
            }
            Ev::Summoned(_) => {}
            Ev::BossEnraged => {
                if let Some(b) = self.world.boss.as_ref() {
                    let key = format!("boss/{}_cry", b.kind.key());
                    ctx.audio.play_at(&key, Self::at(b.x), Bus::EFFECTS);
                    ctx.say(story::boss_enraged(b.kind));
                }
            }
            Ev::BossDefeated(kind) => {
                let x = self.target_x(Target::Boss);
                let key = format!("boss/{}_die", kind.key());
                ctx.audio.play_at(&key, Self::at(x), Bus::EFFECTS);
                ctx.audio.stop(self.music, 3000);
                self.music = Sound::NONE;
                self.playing = None;
                Progress::mark(&mut ctx.game.beaten, kind.key());
                ctx.game.save(&mut ctx.vault);
                ctx.say(story::boss_defeat(kind));
            }
            Ev::Mimic { i } => {
                let x = self.world.enemies[i].x;
                ctx.audio.play_at("sfx/mimic", Self::at(x), Bus::EFFECTS);
            }
            Ev::Chest => {
                let x = self.world.feature.as_ref().map_or(self.world.x, |f| f.x);
                ctx.audio
                    .play_at("sfx/chest_open", Self::at(x), Bus::EFFECTS);
            }
            Ev::Altar => {
                ctx.audio.play("sfx/altar", Bus::EFFECTS);
            }
            Ev::RelicOffer(offer) => {
                ctx.audio.play("sfx/relic", Bus::EFFECTS);
                let title = if self.world.boss.is_some() {
                    "The fallen leave a gift. Choose a relic"
                } else {
                    "The altar takes its price. Choose a relic"
                };
                if let Some(pilot) = self.autopilot.as_mut() {
                    pilot.offered(offer.clone());
                } else {
                    ctx.queued_speech(|ctx| ctx.push(RelicChoice::new(title, offer)));
                }
            }
            Ev::Gate {
                power_before,
                anchored,
            } => {
                self.stop_stage_sounds(ctx);
                ctx.audio.play("sfx/gate", Bus::EFFECTS);
                let w = &self.world;
                record_depth(ctx, w.stage as u32 + 1);
                let line = if anchored {
                    format!(
                        "Through the gate. The Anchor holds. Power {}, health {} of {}.",
                        w.power.round() as u32,
                        shown_health(w.health),
                        w.max_health.round() as u32,
                    )
                } else {
                    ctx.audio.play("sfx/wane", Bus::EFFECTS);
                    format!(
                        "Through the gate. Power {} to {}, health {} of {}.",
                        power_before.round() as u32,
                        w.power.round() as u32,
                        shown_health(w.health),
                        w.max_health.round() as u32,
                    )
                };
                ctx.say(line);
                let stage = self.stage_line();
                ctx.say_queued(stage);
                self.begin_stage(ctx);
            }
            Ev::Won => self.finish(ctx, true),
            Ev::Died => self.finish(ctx, false),
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Waning>, won: bool) {
        self.stop_stage_sounds(ctx);
        let souls = self.world.souls;
        let where_ = self.world.stage_name();
        ctx.game.grace += souls;
        if won {
            ctx.game.wins += 1;
        }
        ctx.game.save(&mut ctx.vault);
        let line = if won {
            ctx.audio.play("sfx/ascend", Bus::EFFECTS);
            format!("You pass the last gate. {souls} souls rise with you.")
        } else {
            ctx.audio.play("sfx/death", Bus::EFFECTS);
            format!("You fall in {where_}. {souls} souls rise with you.")
        };
        ctx.replace(Heaven::after_run(line));
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
        ctx.audio.stop(self.music, 800);
        ctx.audio.stop(self.ambience, 800);
    }

    fn resume(&mut self, ctx: &mut Ctx<Waning>) {
        let line = self.status_line();
        ctx.say(line);
    }

    /// Back from choosing a relic.
    fn resume_with(&mut self, ctx: &mut Ctx<Waning>, result: &str) {
        if let Some(r) = ctx.game.picked.take() {
            self.world.take(r);
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
            Some(Control::Status) => {
                let line = self.status_line();
                ctx.say(line);
            }
            Some(Control::Nearest) => {
                let line = self.nearest_line();
                ctx.say(line);
            }
            Some(Control::Relics) => {
                let line = self.relics_line();
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
            "{} and {} walk. {} attacks the way you face. {} jumps, over an enemy or out of a blow. \
             {} uses a well, altar or chest. {} reads your status, {} the nearest enemy, {} your relics. \
             Escape pauses.",
            sentence_start(&k(Control::Left)),
            k(Control::Right),
            sentence_start(&k(Control::Attack)),
            sentence_start(&k(Control::Jump)),
            sentence_start(&k(Control::Use)),
            sentence_start(&k(Control::Status)),
            k(Control::Nearest),
            k(Control::Relics),
        )
    }
}
