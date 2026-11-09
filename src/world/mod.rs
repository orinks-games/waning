//! The rules of one descent, with no sound or speech: the play screen turns
//! the events this emits into both, and tests drive it directly.

mod boss;
mod combat;
mod enemy;
mod loot;
mod motion;
mod relic;

pub use boss::{Attack, Boss, BossKind, BossMode};
pub use enemy::{Enemy, Kind, Mode};
pub use loot::{Drop, Gear, Perk, Rarity, Refusal, Slot, Ware, WareKind};
pub use relic::Relic;

use serde::{Deserialize, Serialize};

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

const fn area(name: &'static str, region: usize) -> Stage {
    Stage {
        name,
        region,
        boss: None,
    }
}

const fn arena(name: &'static str, region: usize, boss: BossKind) -> Stage {
    Stage {
        name,
        region,
        boss: Some(boss),
    }
}

/// Every stage of a descent, top to bottom: two areas and a boss in each
/// region. Passing the last gate ends the run.
pub const STAGES: [Stage; 12] = [
    area("the Bright Fields", 0),
    area("the Ash Road", 0),
    arena("the Shepherd's Hill", 0, BossKind::Shepherd),
    area("the Hollow Wood", 1),
    area("the Thorn Path", 1),
    arena("the Weeping Glade", 1, BossKind::Mother),
    area("the Drowned Steps", 2),
    area("the Black Water", 2),
    arena("the Ferry", 2, BossKind::Ferryman),
    area("the Cinder Stair", 3),
    area("the Silent City", 3),
    arena("the Hollow Throne", 3, BossKind::King),
];

/// Metres from the start of an area to its gate.
pub const AREA_LENGTH: f32 = 80.0;
/// A boss arena is shorter: there is nowhere to go but through.
pub const ARENA_LENGTH: f32 = 30.0;
pub const WALK_SPEED: f32 = 4.0;
/// Running speed, as a share of walking.
pub const RUN_FACTOR: f32 = 1.75;
/// Seconds in the air after a jump.
pub const AIR_TIME: f32 = 0.75;
pub const DASH_SPEED: f32 = 16.0;
pub const DASH_TIME: f32 = 0.22;
pub const DASH_COOLDOWN: f32 = 0.9;
/// Seconds a dash keeps you out of harm's way.
pub const DASH_GUARD: f32 = 0.3;
pub const ATTACK_COOLDOWN: f32 = 0.45;
/// How long the attack key is held for a heavy blow.
pub const HEAVY_CHARGE: f32 = 0.6;
pub const HEAVY_FACTOR: f32 = 2.5;
/// A block raised this soon before a blow lands is a parry.
pub const PARRY_WINDOW: f32 = 0.25;
/// What a block lets through.
pub const BLOCK_SHARE: f32 = 0.35;
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
/// How close you must stand to something to use or pick it up.
pub const USE_RANGE: f32 = 2.0;
/// Each gate takes this share of your power and health, before Steadfast.
pub const BASE_WANE: f32 = 0.15;
/// What one level of Steadfast takes off the wane.
pub const STEADFAST_STEP: f32 = 0.025;
pub const MAX_POTIONS: u32 = 3;
/// A potion heals this share of your health.
pub const POTION_HEAL: f32 = 0.35;
/// A fall into a pit costs this share of your health.
pub const FALL_COST: f32 = 0.12;
/// Seconds between one rise of a spike trap and the next.
pub const SPIKE_CYCLE: f32 = 3.2;
/// Seconds between a spike trap's click and its rise.
pub const SPIKE_WARN: f32 = 0.8;
pub const SPIKE_REACH: f32 = 0.9;
/// Gold that is worth one soul when a run ends.
pub const GOLD_PER_SOUL: u32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureKind {
    /// Heals.
    Well,
    /// Takes a quarter of your health and offers a relic.
    Altar,
    /// Gold and sometimes gear, or a mimic.
    Chest,
    /// Sells potions, healing, gear and a relic for gold.
    Merchant,
}

impl FeatureKind {
    pub fn key(self) -> &'static str {
        match self {
            FeatureKind::Well => "well",
            FeatureKind::Altar => "altar",
            FeatureKind::Chest => "chest",
            FeatureKind::Merchant => "merchant",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            FeatureKind::Well => "Well",
            FeatureKind::Altar => "Altar",
            FeatureKind::Chest => "Chest",
            FeatureKind::Merchant => "Merchant",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Feature {
    pub kind: FeatureKind,
    pub x: f32,
    pub used: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bolt {
    pub id: u32,
    pub x: f32,
    pub dir: f32,
    pub damage: f32,
    pub from_boss: bool,
    /// Set once it has gone past you, so it cannot hit twice.
    pub passed: bool,
}

/// A gap in the ground: jump or dash across it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pit {
    pub x: f32,
    pub width: f32,
}

impl Pit {
    pub fn centre(&self) -> f32 {
        self.x + self.width / 2.0
    }
}

/// Spikes that click, then rise out of the ground.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spikes {
    pub x: f32,
    /// Seconds into the current cycle.
    pub t: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Enemy(usize),
    Boss,
}

/// What happened this step, for sound and speech.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Step {
        run: bool,
    },
    /// You walked into the edge of the area, or a gate the boss still holds.
    Wall,
    /// You walked into an enemy or the boss and could not pass.
    Bump,
    Swing,
    /// The attack key has been held long enough for a heavy blow.
    Charged,
    /// A heavy blow was let go.
    Heavy,
    Jump,
    Land,
    Dash,
    Hit {
        target: Target,
        killed: bool,
        heavy: bool,
        crit: bool,
    },
    /// Enemy `i` cried out: it strikes or throws when its wind-up ends.
    WindUp {
        i: usize,
    },
    /// A heavy blow or a parry broke enemy `i`'s wind-up.
    Staggered {
        i: usize,
    },
    /// Something struck you for `damage`.
    Struck {
        damage: f32,
    },
    /// A blow missed: you were in the air, dashing or out of reach.
    Dodged,
    /// Your block took most of a blow.
    Blocked,
    /// You raised your block just in time and took nothing.
    Parried,
    /// The Ward Stone turned a blow aside.
    Warded,
    /// Thorn Mail hurt what struck you.
    Thorns,
    /// The Last Ember kept you alive.
    Ember,
    Souls(u32),
    Gold {
        amount: u32,
        x: f32,
    },
    /// A potion was found and taken.
    Potion {
        x: f32,
    },
    Healed(f32),
    Drank,
    NoPotions,
    FullHealth,
    /// Gear `id` fell to the ground.
    Dropped {
        id: u32,
        x: f32,
    },
    /// Gear `id` was picked up, or left behind for gear you took.
    DropGone {
        id: u32,
    },
    /// You put on gear for an empty slot.
    Equipped(Gear),
    /// Gear `id` is at your feet: keep yours or take it (see `World::equip`).
    GearOffer {
        id: u32,
    },
    /// You fell into a pit and climbed back out.
    Fell,
    SpikeWarn {
        x: f32,
    },
    SpikeUp {
        x: f32,
    },
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
    /// A chest opened.
    Chest,
    /// You paid the altar.
    Altar,
    /// You spoke to the merchant (see `World::shop` and `World::buy`).
    Shop,
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
    pub run: bool,
    /// The attack key went down this step.
    pub attack: bool,
    /// The attack key is down, for a heavy blow.
    pub attack_held: bool,
    pub jump: bool,
    pub dash: bool,
    /// The block key is down.
    pub block: bool,
    pub drink: bool,
    /// Use what you stand at, or pick up the gear at your feet.
    pub use_: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub stage: usize,
    pub x: f32,
    /// -1 or 1: the way you last walked, and the way you swing and block.
    pub facing: f32,
    pub health: f32,
    pub max_health: f32,
    pub power: f32,
    /// The power you started the run with, for how weak a blow sounds.
    pub start_power: f32,
    pub wane: f32,
    pub mercy: f32,
    pub souls: u32,
    pub gold: u32,
    pub potions: u32,
    pub weapon: Option<Gear>,
    pub armor: Option<Gear>,
    pub charm: Option<Gear>,
    pub kills: u32,
    pub air: f32,
    pub cooldown: f32,
    pub step_in: f32,
    pub enemies: Vec<Enemy>,
    pub boss: Option<Boss>,
    pub features: Vec<Feature>,
    pub drops: Vec<Drop>,
    pub pits: Vec<Pit>,
    pub spikes: Vec<Spikes>,
    /// What the merchant of this stage sells.
    pub shop: Vec<Ware>,
    pub bolts: Vec<Bolt>,
    pub relics: Vec<Relic>,
    /// Bosses beaten this run.
    pub beaten: Vec<BossKind>,
    /// How much longer than usual each warning lasts, from the settings.
    #[serde(default = "one")]
    pub warn: f32,
    pub over: bool,
    /// Seconds of dash left, and the way it goes.
    dash: f32,
    dash_dir: f32,
    dash_cd: f32,
    /// Seconds no blow can land.
    guard: f32,
    /// Seconds the block has been up; negative while it is down.
    block: f32,
    /// Seconds the attack key has been held since a swing; negative when
    /// it is not.
    charge: f32,
    swings: u32,
    ward_ready: bool,
    ember_used: bool,
    anchor_ready: bool,
    next_id: u32,
    rng: Rng,
}

fn one() -> f32 {
    1.0
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
            gold: 0,
            potions: 1,
            weapon: None,
            armor: None,
            charm: None,
            kills: 0,
            air: 0.0,
            cooldown: 0.0,
            step_in: 0.0,
            enemies: Vec::new(),
            boss: None,
            features: Vec::new(),
            drops: Vec::new(),
            pits: Vec::new(),
            spikes: Vec::new(),
            shop: Vec::new(),
            bolts: Vec::new(),
            relics: Vec::new(),
            beaten: Vec::new(),
            warn: 1.0,
            over: false,
            dash: 0.0,
            dash_dir: 1.0,
            dash_cd: 0.0,
            guard: 0.0,
            block: -1.0,
            charge: -1.0,
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

    pub fn dashing(&self) -> bool {
        self.dash > 0.0
    }

    pub fn blocking(&self) -> bool {
        self.block >= 0.0
    }

    /// Whether a blow aimed at you now would miss.
    pub fn evasive(&self) -> bool {
        self.airborne() || self.guard > 0.0
    }

    pub fn has(&self, r: Relic) -> bool {
        self.relics.contains(&r)
    }

    /// Whether the gate at the end of this stage is open: a boss holds its
    /// gate until it falls.
    pub fn gate_open(&self) -> bool {
        self.boss.as_ref().is_none_or(|b| !b.alive)
    }

    /// What the gold you carry is worth in souls when the run ends.
    pub fn gold_souls(&self) -> u32 {
        self.gold / GOLD_PER_SOUL
    }

    fn id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    /// Fills the current stage: enemies, hazards and something to use in an
    /// area, the boss in an arena.
    fn populate(&mut self) {
        self.enemies.clear();
        self.bolts.clear();
        self.drops.clear();
        self.pits.clear();
        self.spikes.clear();
        self.shop.clear();
        self.features.clear();
        self.boss = None;
        let stage = self.stage_info();
        if let Some(kind) = stage.boss {
            self.boss = Some(Boss::new(kind, 20.0));
            return;
        }
        let depth = self.stage;
        let region = stage.region;

        // Places taken so far, so nothing lands on top of anything else.
        let mut taken: Vec<f32> = Vec::new();
        let mut place = |rng: &mut Rng, lo: f32, hi: f32, room: f32| -> f32 {
            for _ in 0..40 {
                let x = rng.range(lo, hi);
                if taken.iter().all(|t| (t - x).abs() >= room) {
                    taken.push(x);
                    return x;
                }
            }
            let x = rng.range(lo, hi);
            taken.push(x);
            x
        };

        let kind = pick(
            self.rng.unit(),
            &[
                (0.4, FeatureKind::Well),
                (0.65, FeatureKind::Altar),
                (1.0, FeatureKind::Chest),
            ],
        );
        let x = place(&mut self.rng, 25.0, 40.0, 6.0);
        self.features.push(Feature {
            kind,
            x,
            used: false,
        });
        // The second area of each region has a merchant.
        if depth % 3 == 1 {
            let x = place(&mut self.rng, 48.0, 62.0, 6.0);
            self.features.push(Feature {
                kind: FeatureKind::Merchant,
                x,
                used: false,
            });
            self.stock();
        }

        if depth >= 1 {
            let width = 2.0 + 0.15 * region as f32;
            for _ in 0..(1 + depth / 4).min(3) {
                let x = place(&mut self.rng, 12.0, AREA_LENGTH - 8.0, 7.0);
                self.pits.push(Pit { x, width });
            }
        }
        if depth >= 3 {
            for _ in 0..(1 + (depth - 3) / 3) {
                let x = place(&mut self.rng, 12.0, AREA_LENGTH - 8.0, 6.0);
                let t = self.rng.range(0.0, SPIKE_CYCLE);
                self.spikes.push(Spikes { x, t });
            }
        }

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
            let mut x = 15.0 + span * n as f32 + self.rng.range(0.0, span * 0.6);
            // Not standing over a pit.
            for p in &self.pits {
                if x > p.x - 1.0 && x < p.x + p.width + 1.0 {
                    x = p.x + p.width + 1.5;
                }
            }
            let elite = depth >= 1 && self.rng.unit() < 0.15;
            self.enemies.push(Enemy::new(kind, x, depth, elite));
        }
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

    /// Takes a relic from an offer or the merchant.
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
        self.dash_cd = (self.dash_cd - dt).max(0.0);
        self.guard = (self.guard - dt).max(0.0);

        // Jumping and landing.
        if self.air > 0.0 {
            self.air -= dt;
            if self.air <= 0.0 {
                self.air = 0.0;
                out.push(Ev::Land);
                self.land_clear();
            }
        } else if input.jump && !self.dashing() {
            self.air = AIR_TIME
                + if self.has(Relic::FeatherBoots) {
                    0.25
                } else {
                    0.0
                };
            self.block = -1.0;
            out.push(Ev::Jump);
        }

        // The block is up while its key is held on the ground.
        if input.block && !self.airborne() && !self.dashing() {
            self.block = if self.block < 0.0 {
                0.0
            } else {
                self.block + dt
            };
        } else {
            self.block = -1.0;
        }

        if input.dash {
            self.start_dash(input.walk);
        }
        if self.dashing() {
            self.dash_move(dt);
        } else {
            self.walk(dt, input.walk, input.run, &mut out);
        }
        self.fall_check(&mut out);
        self.attack(dt, input, &mut out);
        if input.drink {
            self.drink(&mut out);
        }
        if input.use_ {
            self.use_here(&mut out);
        }
        self.enemies_act(dt, &mut out);
        self.boss_act(dt, &mut out);
        self.bolts_fly(dt, &mut out);
        self.spikes_act(dt, &mut out);

        if self.health <= 0.0 {
            self.health = 0.0;
            self.over = true;
            out.push(Ev::Died);
        } else if self.x >= self.length() {
            self.pass_gate(&mut out);
        }
        out
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
        for d in self.drops.drain(..) {
            out.push(Ev::DropGone { id: d.id });
        }
        self.stage += 1;
        self.x = 0.0;
        self.air = 0.0;
        self.dash = 0.0;
        self.ward_ready = true;
        self.populate();
        out.push(Ev::Gate {
            power_before,
            anchored,
        });
    }
}

/// "1 soul", "12 souls"
pub fn souls(n: u32) -> String {
    if n == 1 {
        "1 soul".into()
    } else {
        format!("{n} souls")
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
