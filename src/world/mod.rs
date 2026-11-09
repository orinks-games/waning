//! The rules of one descent, with no sound or speech: the play screen turns
//! the events this emits into both, and tests drive it directly.

mod boss;
mod enemy;
mod relic;

pub use boss::{Attack, Boss, BossKind, BossMode};
pub use enemy::{Enemy, Kind, Mode};
pub use relic::Relic;

use crate::progress::Progress;
use crate::rng::Rng;

pub struct Stage {
    pub name: &'static str,
    /// Index into `REGIONS`.
    pub region: usize,
    pub boss: Option<BossKind>,
}

pub const REGIONS: [&str; 4] = [
    "the Bright Fields",
    "the Hollow Wood",
    "the Drowned Steps",
    "the Cinder Stair",
];

/// Every stage of a descent, top to bottom: two areas and a boss in each
/// region. Passing the last gate ends the run.
pub const STAGES: [Stage; 12] = [
    Stage {
        name: "the Bright Fields",
        region: 0,
        boss: None,
    },
    Stage {
        name: "the Ash Road",
        region: 0,
        boss: None,
    },
    Stage {
        name: "the Shepherd's Hill",
        region: 0,
        boss: Some(BossKind::Shepherd),
    },
    Stage {
        name: "the Hollow Wood",
        region: 1,
        boss: None,
    },
    Stage {
        name: "the Thorn Path",
        region: 1,
        boss: None,
    },
    Stage {
        name: "the Weeping Glade",
        region: 1,
        boss: Some(BossKind::Mother),
    },
    Stage {
        name: "the Drowned Steps",
        region: 2,
        boss: None,
    },
    Stage {
        name: "the Black Water",
        region: 2,
        boss: None,
    },
    Stage {
        name: "the Ferry",
        region: 2,
        boss: Some(BossKind::Ferryman),
    },
    Stage {
        name: "the Cinder Stair",
        region: 3,
        boss: None,
    },
    Stage {
        name: "the Silent City",
        region: 3,
        boss: None,
    },
    Stage {
        name: "the Hollow Throne",
        region: 3,
        boss: Some(BossKind::King),
    },
];

/// Metres from the start of an area to its gate.
pub const AREA_LENGTH: f32 = 80.0;
/// A boss arena is shorter: there is nowhere to go but through.
pub const ARENA_LENGTH: f32 = 30.0;
pub const WALK_SPEED: f32 = 4.0;
/// Seconds in the air after a jump.
pub const AIR_TIME: f32 = 0.75;
pub const ATTACK_COOLDOWN: f32 = 0.45;
/// How far ahead a swing reaches.
pub const REACH: f32 = 2.2;
/// How close an enemy must be to notice you.
pub const NOTICE_RANGE: f32 = 14.0;
/// How close an enemy comes before it winds up.
pub const STRIKE_RANGE: f32 = 1.6;
/// How far a strike reaches when it lands.
pub const STRIKE_REACH: f32 = 2.4;
/// The closest you can stand to a living enemy while on the ground.
pub const BODY: f32 = 1.0;
/// A boss is bigger.
pub const BOSS_BODY: f32 = 1.4;
/// How far a boss's slam reaches.
pub const SLAM_REACH: f32 = 3.0;
pub const CHARGE_SPEED: f32 = 11.0;
pub const BOLT_SPEED: f32 = 7.0;
/// How close you must stand to a well, altar or chest to use it.
pub const USE_RANGE: f32 = 2.0;
/// Each gate takes this share of your power and health, before Steadfast.
pub const BASE_WANE: f32 = 0.15;
/// What one level of Steadfast takes off the wane.
pub const STEADFAST_STEP: f32 = 0.025;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureKind {
    /// Heals.
    Well,
    /// Takes a quarter of your health and offers a relic.
    Altar,
    /// Souls, or a mimic.
    Chest,
}

impl FeatureKind {
    pub fn key(self) -> &'static str {
        match self {
            FeatureKind::Well => "well",
            FeatureKind::Altar => "altar",
            FeatureKind::Chest => "chest",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            FeatureKind::Well => "Well",
            FeatureKind::Altar => "Altar",
            FeatureKind::Chest => "Chest",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Feature {
    pub kind: FeatureKind,
    pub x: f32,
    pub used: bool,
}

#[derive(Clone, Debug)]
pub struct Bolt {
    pub id: u32,
    pub x: f32,
    pub dir: f32,
    pub damage: f32,
    pub from_boss: bool,
    /// Set once it has gone past you, so it cannot hit twice.
    pub passed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Enemy(usize),
    Boss,
}

/// What happened this step, for sound and speech.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Step,
    /// You walked into the edge of the area, or a gate the boss still holds.
    Wall,
    /// You walked into an enemy or the boss and could not pass.
    Bump,
    Swing,
    Jump,
    Land,
    Hit {
        target: Target,
        killed: bool,
    },
    /// Enemy `i` cried out: it strikes or throws when its wind-up ends.
    WindUp {
        i: usize,
    },
    /// Something struck you for `damage`.
    Struck {
        damage: f32,
    },
    /// A blow missed: you were in the air or out of reach.
    Dodged,
    /// The Ward Stone turned a blow aside.
    Warded,
    /// Thorn Mail hurt what struck you.
    Thorns,
    /// The Last Ember kept you alive.
    Ember,
    Souls(u32),
    Healed(f32),
    /// A bolt was thrown.
    Fired {
        id: u32,
    },
    /// A bolt hit you (`hit`) or left the area, at `x`.
    BoltGone {
        id: u32,
        x: f32,
        hit: bool,
    },
    BossWindUp(Attack),
    BossSlam,
    BossChargeEnd,
    BossVanish {
        x: f32,
    },
    BossAppear {
        x: f32,
    },
    /// The boss called up spawn: their indexes into `World::enemies`.
    Summoned(Vec<usize>),
    /// The boss fell below half health.
    BossEnraged,
    BossDefeated(BossKind),
    /// A chest turned out to be enemy `i`.
    Mimic {
        i: usize,
    },
    /// A chest held souls (reported separately as `Souls`).
    Chest,
    /// You paid the altar.
    Altar,
    /// Pick one of these (see `World::take`).
    RelicOffer(Vec<Relic>),
    /// You passed a gate and the next stage began. `anchored` when the
    /// Anchor kept the wane off.
    Gate {
        power_before: f32,
        anchored: bool,
    },
    /// You passed the last gate.
    Won,
    Died,
}

/// The input for one step.
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    /// -1 walks left, 1 walks right, 0 stands.
    pub walk: i8,
    pub attack: bool,
    pub jump: bool,
    /// Use the well, altar or chest you stand at.
    pub use_: bool,
}

#[derive(Clone, Debug)]
pub struct World {
    pub stage: usize,
    pub x: f32,
    /// -1 or 1: the way you last walked, and the way you swing.
    pub facing: f32,
    pub health: f32,
    pub max_health: f32,
    pub power: f32,
    /// The power you started the run with, for how weak a blow sounds.
    pub start_power: f32,
    pub wane: f32,
    pub mercy: f32,
    pub souls: u32,
    pub kills: u32,
    pub air: f32,
    pub cooldown: f32,
    pub step_in: f32,
    pub enemies: Vec<Enemy>,
    pub boss: Option<Boss>,
    pub feature: Option<Feature>,
    pub bolts: Vec<Bolt>,
    pub relics: Vec<Relic>,
    /// Bosses beaten this run.
    pub beaten: Vec<BossKind>,
    pub over: bool,
    swings: u32,
    ward_ready: bool,
    ember_used: bool,
    anchor_ready: bool,
    next_id: u32,
    rng: Rng,
}

impl World {
    pub fn new(p: &Progress, seed: u64) -> World {
        let power = 60.0 + 15.0 * p.might as f32;
        let health = 100.0 + 25.0 * p.vigor as f32;
        let mut w = World {
            stage: 0,
            x: 0.0,
            facing: 1.0,
            health,
            max_health: health,
            power,
            start_power: power,
            wane: BASE_WANE - STEADFAST_STEP * p.steadfast as f32,
            mercy: 0.15 * p.mercy as f32,
            souls: 0,
            kills: 0,
            air: 0.0,
            cooldown: 0.0,
            step_in: 0.0,
            enemies: Vec::new(),
            boss: None,
            feature: None,
            bolts: Vec::new(),
            relics: Vec::new(),
            beaten: Vec::new(),
            over: false,
            swings: 0,
            ward_ready: true,
            ember_used: false,
            anchor_ready: false,
            next_id: 1,
            rng: Rng::new(seed),
        };
        w.populate();
        w
    }

    pub fn stage_info(&self) -> &'static Stage {
        &STAGES[self.stage.min(STAGES.len() - 1)]
    }

    pub fn stage_name(&self) -> &'static str {
        self.stage_info().name
    }

    pub fn region(&self) -> usize {
        self.stage_info().region
    }

    pub fn length(&self) -> f32 {
        if self.stage_info().boss.is_some() {
            ARENA_LENGTH
        } else {
            AREA_LENGTH
        }
    }

    pub fn airborne(&self) -> bool {
        self.air > 0.0
    }

    pub fn has(&self, r: Relic) -> bool {
        self.relics.contains(&r)
    }

    /// Whether the gate at the end of this stage is open: a boss holds its
    /// gate until it falls.
    pub fn gate_open(&self) -> bool {
        self.boss.as_ref().is_none_or(|b| !b.alive)
    }

    fn walk_speed(&self) -> f32 {
        WALK_SPEED
            * if self.has(Relic::QuickStep) {
                1.33
            } else {
                1.0
            }
    }

    /// Fills the current stage: enemies and a well, altar or chest in an
    /// area, the boss in an arena.
    fn populate(&mut self) {
        self.enemies.clear();
        self.bolts.clear();
        self.boss = None;
        self.feature = None;
        let stage = self.stage_info();
        if let Some(kind) = stage.boss {
            self.boss = Some(Boss::new(kind, 20.0));
            return;
        }
        let depth = self.stage;
        let region = stage.region;
        let count = 4 + depth / 2;
        let span = (AREA_LENGTH - 25.0) / count as f32;
        for n in 0..count {
            let roll = self.rng.unit();
            let kind = match region {
                0 if depth == 0 => Kind::Slime,
                0 => pick(roll, &[(0.65, Kind::Slime), (1.0, Kind::Shade)]),
                1 => pick(
                    roll,
                    &[(0.35, Kind::Slime), (0.7, Kind::Shade), (1.0, Kind::Wraith)],
                ),
                2 => pick(
                    roll,
                    &[
                        (0.2, Kind::Slime),
                        (0.5, Kind::Shade),
                        (0.75, Kind::Wraith),
                        (1.0, Kind::Ogre),
                    ],
                ),
                _ => pick(
                    roll,
                    &[(0.35, Kind::Shade), (0.65, Kind::Wraith), (1.0, Kind::Ogre)],
                ),
            };
            let x = 15.0 + span * n as f32 + self.rng.range(0.0, span * 0.6);
            self.enemies.push(Enemy::new(kind, x, depth));
        }
        let kind = pick(
            self.rng.unit(),
            &[
                (0.4, FeatureKind::Well),
                (0.65, FeatureKind::Altar),
                (1.0, FeatureKind::Chest),
            ],
        );
        let x = self.rng.range(30.0, 55.0);
        self.feature = Some(Feature {
            kind,
            x,
            used: false,
        });
    }

    /// The living enemy nearest you, and its signed distance (positive is
    /// to the right).
    pub fn nearest(&self) -> Option<(usize, f32)> {
        self.enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.alive)
            .map(|(i, e)| (i, e.x - self.x))
            .min_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
    }

    /// Three relics you do not have, for an offer.
    fn offer(&mut self) -> Vec<Relic> {
        let mut left: Vec<Relic> = Relic::ALL.into_iter().filter(|r| !self.has(*r)).collect();
        let mut out = Vec::new();
        while out.len() < 3 && !left.is_empty() {
            let i = (self.rng.unit() * left.len() as f32) as usize;
            out.push(left.swap_remove(i.min(left.len() - 1)));
        }
        out
    }

    /// Takes a relic from an offer.
    pub fn take(&mut self, r: Relic) {
        if !self.has(r) {
            self.relics.push(r);
            if r == Relic::Anchor {
                self.anchor_ready = true;
            }
        }
    }

    pub fn step(&mut self, dt: f32, input: Input) -> Vec<Ev> {
        let mut out = Vec::new();
        if self.over {
            return out;
        }
        self.cooldown = (self.cooldown - dt).max(0.0);

        // Jumping and landing.
        if self.air > 0.0 {
            self.air -= dt;
            if self.air <= 0.0 {
                self.air = 0.0;
                out.push(Ev::Land);
                self.land_clear();
            }
        } else if input.jump {
            self.air = AIR_TIME
                + if self.has(Relic::FeatherBoots) {
                    0.25
                } else {
                    0.0
                };
            out.push(Ev::Jump);
        }

        self.walk(dt, input.walk, &mut out);
        if input.attack && self.cooldown <= 0.0 {
            self.swing(&mut out);
        }
        if input.use_ {
            self.use_feature(&mut out);
        }
        self.enemies_act(dt, &mut out);
        self.boss_act(dt, &mut out);
        self.bolts_fly(dt, &mut out);

        if self.health <= 0.0 {
            self.health = 0.0;
            self.over = true;
            out.push(Ev::Died);
        } else if self.x >= self.length() {
            self.pass_gate(&mut out);
        }
        out
    }

    /// Landing inside an enemy or the boss pushes you clear of it.
    fn land_clear(&mut self) {
        let mut bodies: Vec<(f32, f32)> = self
            .enemies
            .iter()
            .filter(|e| e.alive)
            .map(|e| (e.x, BODY))
            .collect();
        if let Some(b) = self.boss.as_ref().filter(|b| b.present()) {
            bodies.push((b.x, BOSS_BODY));
        }
        for (ex, body) in bodies {
            let d = ex - self.x;
            if d.abs() < body {
                let away = if d == 0.0 { -self.facing } else { -d.signum() };
                self.x = (self.x + away * (body - d.abs())).clamp(0.0, self.length());
            }
        }
    }

    fn walk(&mut self, dt: f32, dir: i8, out: &mut Vec<Ev>) {
        if dir == 0 {
            self.step_in = 0.0;
            return;
        }
        let dir = dir.signum() as f32;
        self.facing = dir;
        let mut x = self.x + dir * self.walk_speed() * dt;
        let end = self.length();
        let mut wall = false;
        if x < 0.0 {
            x = 0.0;
            wall = true;
        } else if x >= end && !self.gate_open() {
            x = end - 0.01;
            wall = true;
        }
        let mut bumped = false;
        if !self.airborne() {
            // A living enemy or the boss in the way stops you, unless you
            // jump it.
            let mut bodies: Vec<(f32, f32)> = self
                .enemies
                .iter()
                .filter(|e| e.alive)
                .map(|e| (e.x, BODY))
                .collect();
            if let Some(b) = self.boss.as_ref().filter(|b| b.present()) {
                if !matches!(b.mode, BossMode::Charging { .. }) {
                    bodies.push((b.x, BOSS_BODY));
                }
            }
            for (ex, body) in bodies {
                let before = ex - self.x;
                let after = ex - x;
                if before.signum() == dir && after * dir < body {
                    x = ex - dir * body;
                    bumped = true;
                }
            }
        }
        let moved = (x - self.x).abs() > 1e-4;
        self.x = x;
        self.step_in -= dt;
        if moved && !self.airborne() {
            if self.step_in <= 0.0 {
                out.push(Ev::Step);
                self.step_in = 0.4;
            }
        } else if !moved && self.step_in <= 0.0 {
            if bumped {
                out.push(Ev::Bump);
                self.step_in = 0.6;
            } else if wall {
                out.push(Ev::Wall);
                self.step_in = 0.6;
            }
        }
    }

    fn blow(&self) -> f32 {
        self.power
            * if self.has(Relic::EmberEdge) {
                1.25
            } else {
                1.0
            }
    }

    fn swing(&mut self, out: &mut Vec<Ev>) {
        self.cooldown = ATTACK_COOLDOWN;
        self.swings += 1;
        out.push(Ev::Swing);
        let facing = self.facing;
        let mut damage = self.blow();
        if self.has(Relic::EchoCharm) && self.swings.is_multiple_of(3) {
            damage *= 2.0;
        }
        let enemy = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.alive)
            .map(|(i, e)| (Target::Enemy(i), (e.x - self.x) * facing))
            .filter(|&(_, ahead)| ahead > -0.3 && ahead <= REACH);
        let boss = self
            .boss
            .as_ref()
            .filter(|b| b.present())
            .map(|b| (Target::Boss, (b.x - self.x) * facing))
            .filter(|&(_, ahead)| ahead > -0.5 && ahead <= REACH + 0.6);
        let Some((target, _)) = enemy.chain(boss).min_by(|a, b| a.1.total_cmp(&b.1)) else {
            return;
        };
        match target {
            Target::Enemy(i) => self.hit_enemy(i, damage, out),
            Target::Boss => self.hit_boss(damage, out),
        }
    }

    fn soul_share(&self, base: u32) -> u32 {
        let purse = if self.has(Relic::MisersPurse) {
            1.5
        } else {
            1.0
        };
        (base as f32 * (1.0 + 0.1 * self.stage as f32) * purse).round() as u32
    }

    fn on_kill(&mut self, base_souls: u32, out: &mut Vec<Ev>) {
        self.kills += 1;
        let souls = self.soul_share(base_souls);
        if souls > 0 {
            self.souls += souls;
            out.push(Ev::Souls(souls));
        }
        if self.has(Relic::HungryBlade) && self.health < self.max_health {
            let healed = 4.0f32.min(self.max_health - self.health);
            self.health += healed;
            out.push(Ev::Healed(healed));
        }
    }

    fn hit_enemy(&mut self, i: usize, damage: f32, out: &mut Vec<Ev>) {
        let facing = self.facing;
        let e = &mut self.enemies[i];
        e.health -= damage;
        if e.health <= 0.0 {
            e.alive = false;
            let souls = e.kind.souls();
            out.push(Ev::Hit {
                target: Target::Enemy(i),
                killed: true,
            });
            self.on_kill(souls, out);
        } else {
            // A blow that does not kill knocks it back; a wind-up survives it.
            e.x = (e.x + facing * e.kind.knockback()).clamp(0.0, AREA_LENGTH);
            out.push(Ev::Hit {
                target: Target::Enemy(i),
                killed: false,
            });
        }
    }

    fn hit_boss(&mut self, damage: f32, out: &mut Vec<Ev>) {
        let Some(b) = self.boss.as_mut() else {
            return;
        };
        b.health -= damage;
        if b.health <= 0.0 {
            b.alive = false;
            let kind = b.kind;
            out.push(Ev::Hit {
                target: Target::Boss,
                killed: true,
            });
            for e in &mut self.enemies {
                e.alive = false;
            }
            for bolt in self.bolts.drain(..) {
                out.push(Ev::BoltGone {
                    id: bolt.id,
                    x: bolt.x,
                    hit: false,
                });
            }
            self.beaten.push(kind);
            out.push(Ev::BossDefeated(kind));
            self.on_kill(kind.souls(), out);
            let offer = self.offer();
            if !offer.is_empty() {
                out.push(Ev::RelicOffer(offer));
            }
            return;
        }
        out.push(Ev::Hit {
            target: Target::Boss,
            killed: false,
        });
        if !b.enraged && b.health <= b.max_health / 2.0 {
            b.enraged = true;
            out.push(Ev::BossEnraged);
        }
    }

    /// A blow lands on you, unless a relic stops it. `from` is who to hurt
    /// with Thorn Mail.
    pub(super) fn take_hit(&mut self, damage: f32, from: Option<Target>, out: &mut Vec<Ev>) {
        if self.ward_ready && self.has(Relic::WardStone) {
            self.ward_ready = false;
            out.push(Ev::Warded);
            return;
        }
        self.health -= damage;
        if self.health <= 0.0 && self.has(Relic::LastEmber) && !self.ember_used {
            self.ember_used = true;
            self.health = 1.0;
            out.push(Ev::Ember);
        }
        out.push(Ev::Struck { damage });
        if let (true, Some(from)) = (self.has(Relic::ThornMail), from) {
            out.push(Ev::Thorns);
            let thorns = self.power / 3.0;
            match from {
                Target::Enemy(i) => {
                    if self.enemies[i].alive {
                        self.hit_enemy(i, thorns, out);
                    }
                }
                Target::Boss => self.hit_boss(thorns, out),
            }
        }
    }

    fn use_feature(&mut self, out: &mut Vec<Ev>) {
        let Some(f) = self.feature.as_mut() else {
            return;
        };
        if f.used || (f.x - self.x).abs() > USE_RANGE {
            return;
        }
        f.used = true;
        let (kind, fx) = (f.kind, f.x);
        match kind {
            FeatureKind::Well => {
                let healed = (0.35 * self.max_health).min(self.max_health - self.health);
                self.health += healed;
                out.push(Ev::Healed(healed));
            }
            FeatureKind::Altar => {
                self.health = (self.health * 0.75).max(1.0);
                out.push(Ev::Altar);
                let offer = self.offer();
                if !offer.is_empty() {
                    out.push(Ev::RelicOffer(offer));
                }
            }
            FeatureKind::Chest => {
                if self.rng.unit() < 0.3 {
                    let mut m = Enemy::new(Kind::Mimic, fx, self.stage);
                    m.mode = Mode::WindUp(Kind::Mimic.windup());
                    self.enemies.push(m);
                    out.push(Ev::Mimic {
                        i: self.enemies.len() - 1,
                    });
                } else {
                    let souls = self.soul_share(10);
                    self.souls += souls;
                    out.push(Ev::Chest);
                    out.push(Ev::Souls(souls));
                }
            }
        }
    }

    fn enemies_act(&mut self, dt: f32, out: &mut Vec<Ev>) {
        let px = self.x;
        let airborne = self.airborne();
        let end = self.length();
        let mut strikes = Vec::new();
        for (i, e) in self.enemies.iter_mut().enumerate() {
            if !e.alive {
                continue;
            }
            let d = px - e.x;
            match e.mode {
                Mode::Idle => {
                    if d.abs() <= NOTICE_RANGE {
                        e.mode = Mode::Approach;
                    }
                }
                Mode::Approach if e.kind.ranged() => {
                    // Keeps between 5 and 9 metres away, then throws.
                    let speed = e.kind.speed() * dt;
                    if d.abs() > 9.0 {
                        e.x += d.signum() * speed;
                    } else if d.abs() < 5.0 {
                        e.x = (e.x - d.signum() * speed).clamp(0.0, end);
                    }
                    if (5.0..=9.5).contains(&d.abs()) || e.x <= 0.0 || e.x >= end {
                        e.mode = Mode::WindUp(e.kind.windup());
                        out.push(Ev::WindUp { i });
                    }
                }
                Mode::Approach => {
                    if d.abs() <= STRIKE_RANGE {
                        e.mode = Mode::WindUp(e.kind.windup());
                        out.push(Ev::WindUp { i });
                    } else {
                        let step = (e.kind.speed() * dt).min(d.abs() - BODY).max(0.0);
                        e.x += d.signum() * step;
                    }
                }
                Mode::WindUp(t) => {
                    let t = t - dt;
                    if t > 0.0 {
                        e.mode = Mode::WindUp(t);
                        continue;
                    }
                    if e.kind.ranged() {
                        e.mode = Mode::Recover(2.2);
                        strikes.push((i, true, e.x, d.signum(), e.damage));
                    } else {
                        e.mode = Mode::Recover(1.2);
                        let hit = d.abs() <= STRIKE_REACH && !airborne;
                        strikes.push((i, hit, e.x, 0.0, e.damage));
                    }
                }
                Mode::Recover(t) => {
                    let t = t - dt;
                    e.mode = if t > 0.0 {
                        Mode::Recover(t)
                    } else {
                        Mode::Approach
                    };
                }
            }
        }
        for (i, hit, x, dir, damage) in strikes {
            if dir != 0.0 {
                self.throw(x, dir, damage, false, out);
            } else if hit {
                self.take_hit(damage, Some(Target::Enemy(i)), out);
            } else {
                out.push(Ev::Dodged);
            }
        }
    }

    pub(super) fn throw(
        &mut self,
        x: f32,
        dir: f32,
        damage: f32,
        from_boss: bool,
        out: &mut Vec<Ev>,
    ) {
        let id = self.next_id;
        self.next_id += 1;
        self.bolts.push(Bolt {
            id,
            x,
            dir,
            damage,
            from_boss,
            passed: false,
        });
        out.push(Ev::Fired { id });
    }

    fn bolts_fly(&mut self, dt: f32, out: &mut Vec<Ev>) {
        let px = self.x;
        let airborne = self.airborne();
        let end = self.length();
        let mut hits = Vec::new();
        self.bolts.retain_mut(|b| {
            let before = b.x - px;
            b.x += b.dir * BOLT_SPEED * dt;
            let after = b.x - px;
            if !b.passed && (before.signum() != after.signum() || after.abs() < 0.3) {
                if airborne {
                    b.passed = true;
                    out.push(Ev::Dodged);
                } else {
                    hits.push((b.damage, b.from_boss));
                    out.push(Ev::BoltGone {
                        id: b.id,
                        x: b.x,
                        hit: true,
                    });
                    return false;
                }
            }
            if b.x < -5.0 || b.x > end + 5.0 {
                out.push(Ev::BoltGone {
                    id: b.id,
                    x: b.x,
                    hit: false,
                });
                return false;
            }
            true
        });
        for (damage, from_boss) in hits {
            let from = from_boss.then_some(Target::Boss);
            self.take_hit(damage, from, out);
        }
    }

    fn pass_gate(&mut self, out: &mut Vec<Ev>) {
        let power_before = self.power;
        if self.stage + 1 >= STAGES.len() {
            self.over = true;
            out.push(Ev::Won);
            return;
        }
        let anchored = self.anchor_ready;
        if anchored {
            self.anchor_ready = false;
        } else {
            let keep = 1.0 - self.wane;
            self.power *= keep;
            self.max_health *= keep;
            self.health *= keep;
        }
        self.health = (self.health + self.mercy * self.max_health).min(self.max_health);
        for bolt in self.bolts.drain(..) {
            out.push(Ev::BoltGone {
                id: bolt.id,
                x: bolt.x,
                hit: false,
            });
        }
        self.stage += 1;
        self.x = 0.0;
        self.air = 0.0;
        self.ward_ready = true;
        self.populate();
        out.push(Ev::Gate {
            power_before,
            anchored,
        });
    }
}

/// The first entry whose threshold `roll` falls under.
fn pick<T: Copy>(roll: f32, table: &[(f32, T)]) -> T {
    table
        .iter()
        .find(|(limit, _)| roll < *limit)
        .map(|(_, t)| *t)
        .unwrap_or(table[table.len() - 1].1)
}
