//! The rules of one descent, with no sound or speech: the play screen turns
//! the events this emits into both, and tests drive it directly.

use crate::progress::Progress;
use crate::rng::Rng;

/// The areas, top to bottom. Reaching the end of the last one ends the run.
pub const AREAS: [&str; 8] = [
    "the Bright Fields",
    "the Ash Road",
    "the Hollow Wood",
    "the Drowned Steps",
    "the Bone Quarry",
    "the Silent City",
    "the Cinder Stair",
    "the Last Gate",
];

/// Metres from the start of an area to its gate.
pub const AREA_LENGTH: f32 = 80.0;
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
/// Each gate takes this share of your power and health, before Steadfast.
pub const BASE_WANE: f32 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Slime,
    Shade,
    Ogre,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Slime => "Slime",
            Kind::Shade => "Shade",
            Kind::Ogre => "Ogre",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Kind::Slime => "slime",
            Kind::Shade => "shade",
            Kind::Ogre => "ogre",
        }
    }
    fn health(self) -> f32 {
        match self {
            Kind::Slime => 30.0,
            Kind::Shade => 45.0,
            Kind::Ogre => 140.0,
        }
    }
    fn damage(self) -> f32 {
        match self {
            Kind::Slime => 10.0,
            Kind::Shade => 14.0,
            Kind::Ogre => 28.0,
        }
    }
    fn speed(self) -> f32 {
        match self {
            Kind::Slime => 1.3,
            Kind::Shade => 2.6,
            Kind::Ogre => 0.9,
        }
    }
    /// Seconds between the cry and the blow: the time you have to jump or
    /// step back.
    pub fn windup(self) -> f32 {
        match self {
            Kind::Slime => 1.0,
            Kind::Shade => 0.7,
            Kind::Ogre => 1.3,
        }
    }
    fn souls(self) -> u32 {
        match self {
            Kind::Slime => 3,
            Kind::Shade => 5,
            Kind::Ogre => 12,
        }
    }
    /// How far a blow from you throws it back.
    fn knockback(self) -> f32 {
        match self {
            Kind::Ogre => 0.3,
            _ => 1.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Has not noticed you.
    Idle,
    Approach,
    /// Crying out; strikes when the timer runs out.
    WindUp(f32),
    /// Getting its breath back after a blow.
    Recover(f32),
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub kind: Kind,
    pub x: f32,
    pub health: f32,
    pub max_health: f32,
    pub damage: f32,
    pub mode: Mode,
    pub alive: bool,
}

/// What happened this step, for sound and speech.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Step,
    Wall,
    Swing,
    Jump,
    Land,
    /// You hit enemy `i` (index into `World::enemies`).
    Hit {
        i: usize,
        killed: bool,
    },
    /// You ran into a living enemy and could not pass.
    Blocked {
        i: usize,
    },
    Noticed {
        i: usize,
    },
    WindUp {
        i: usize,
    },
    /// Enemy `i` struck you for `damage`.
    Struck {
        i: usize,
        damage: f32,
    },
    /// Enemy `i` struck and missed: you were in the air or out of reach.
    Dodged {
        i: usize,
    },
    Souls(u32),
    /// You passed the gate at the end of the area you were in, and the
    /// wane took its share. The next area has begun.
    Gate {
        power_before: f32,
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
}

#[derive(Clone, Debug)]
pub struct World {
    pub area: usize,
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
    pub over: bool,
    rng: Rng,
}

impl World {
    pub fn new(p: &Progress, seed: u64) -> World {
        let power = 60.0 + 15.0 * p.might as f32;
        let health = 100.0 + 25.0 * p.vigor as f32;
        let mut w = World {
            area: 0,
            x: 0.0,
            facing: 1.0,
            health,
            max_health: health,
            power,
            start_power: power,
            wane: BASE_WANE - 0.03 * p.steadfast as f32,
            mercy: 0.15 * p.mercy as f32,
            souls: 0,
            kills: 0,
            air: 0.0,
            cooldown: 0.0,
            step_in: 0.0,
            enemies: Vec::new(),
            over: false,
            rng: Rng::new(seed),
        };
        w.populate();
        w
    }

    pub fn area_name(&self) -> &'static str {
        AREAS[self.area.min(AREAS.len() - 1)]
    }

    pub fn airborne(&self) -> bool {
        self.air > 0.0
    }

    /// Fills the current area with enemies: more, and tougher, the deeper
    /// you are.
    fn populate(&mut self) {
        let a = self.area;
        let count = 4 + a;
        let span = (AREA_LENGTH - 25.0) / count as f32;
        let scale_health = 1.0 + 0.2 * a as f32;
        let scale_damage = 1.0 + 0.15 * a as f32;
        self.enemies.clear();
        for n in 0..count {
            let roll = self.rng.unit();
            let kind = match a {
                0 => Kind::Slime,
                1 => {
                    if roll < 0.6 {
                        Kind::Slime
                    } else {
                        Kind::Shade
                    }
                }
                _ => {
                    let ogre = (0.15 + 0.05 * a as f32).min(0.4);
                    if roll < ogre {
                        Kind::Ogre
                    } else if roll < 0.65 {
                        Kind::Shade
                    } else {
                        Kind::Slime
                    }
                }
            };
            let x = 15.0 + span * n as f32 + self.rng.range(0.0, span * 0.6);
            let health = kind.health() * scale_health;
            self.enemies.push(Enemy {
                kind,
                x,
                health,
                max_health: health,
                damage: kind.damage() * scale_damage,
                mode: Mode::Idle,
                alive: true,
            });
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
                // Landing inside an enemy pushes you clear of it.
                if let Some((_, d)) = self.nearest() {
                    if d.abs() < BODY {
                        self.x -= d.signum() * (BODY - d.abs());
                    }
                }
            }
        } else if input.jump {
            self.air = AIR_TIME;
            out.push(Ev::Jump);
        }

        self.walk(dt, input.walk, &mut out);
        if input.attack && self.cooldown <= 0.0 {
            self.swing(&mut out);
        }
        self.enemies_act(dt, &mut out);

        if self.health <= 0.0 {
            self.health = 0.0;
            self.over = true;
            out.push(Ev::Died);
        } else if self.x >= AREA_LENGTH {
            self.pass_gate(&mut out);
        }
        out
    }

    fn walk(&mut self, dt: f32, dir: i8, out: &mut Vec<Ev>) {
        if dir == 0 {
            self.step_in = 0.0;
            return;
        }
        let dir = dir.signum() as f32;
        self.facing = dir;
        let mut x = self.x + dir * WALK_SPEED * dt;
        if x < 0.0 {
            if self.x > 0.0 || self.step_in <= 0.0 {
                out.push(Ev::Wall);
                self.step_in = 0.5;
            } else {
                self.step_in -= dt;
            }
            self.x = 0.0;
            return;
        }
        if !self.airborne() {
            // A living enemy in the way stops you, unless you jump it.
            for (i, e) in self.enemies.iter().enumerate() {
                if !e.alive {
                    continue;
                }
                let before = e.x - self.x;
                let after = e.x - x;
                if before.signum() == dir && after * dir < BODY {
                    x = e.x - dir * BODY;
                    if (x - self.x).abs() < 1e-4 && self.step_in <= 0.0 {
                        out.push(Ev::Blocked { i });
                        self.step_in = 0.6;
                    }
                }
            }
        }
        let moved = (x - self.x).abs() > 1e-4;
        self.x = x;
        if moved && !self.airborne() {
            self.step_in -= dt;
            if self.step_in <= 0.0 {
                out.push(Ev::Step);
                self.step_in = 0.4;
            }
        } else if !moved {
            self.step_in -= dt;
        }
    }

    fn swing(&mut self, out: &mut Vec<Ev>) {
        self.cooldown = ATTACK_COOLDOWN;
        out.push(Ev::Swing);
        let facing = self.facing;
        let target = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.alive)
            .map(|(i, e)| (i, (e.x - self.x) * facing))
            .filter(|&(_, ahead)| ahead > -0.3 && ahead <= REACH)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let Some((i, _)) = target else {
            return;
        };
        let e = &mut self.enemies[i];
        e.health -= self.power;
        if e.health <= 0.0 {
            e.alive = false;
            self.kills += 1;
            let souls = (e.kind.souls() as f32 * (1.0 + 0.25 * self.area as f32)).round() as u32;
            self.souls += souls;
            out.push(Ev::Hit { i, killed: true });
            out.push(Ev::Souls(souls));
        } else {
            // A blow that does not kill knocks it back; a wind-up survives it.
            e.x += facing * e.kind.knockback();
            out.push(Ev::Hit { i, killed: false });
        }
    }

    fn enemies_act(&mut self, dt: f32, out: &mut Vec<Ev>) {
        let px = self.x;
        let airborne = self.airborne();
        for (i, e) in self.enemies.iter_mut().enumerate() {
            if !e.alive {
                continue;
            }
            let d = px - e.x;
            match e.mode {
                Mode::Idle => {
                    if d.abs() <= NOTICE_RANGE {
                        e.mode = Mode::Approach;
                        out.push(Ev::Noticed { i });
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
                    e.mode = Mode::Recover(1.2);
                    if d.abs() <= STRIKE_REACH && !airborne {
                        self.health -= e.damage;
                        out.push(Ev::Struck {
                            i,
                            damage: e.damage,
                        });
                    } else {
                        out.push(Ev::Dodged { i });
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
    }

    fn pass_gate(&mut self, out: &mut Vec<Ev>) {
        let power_before = self.power;
        if self.area + 1 >= AREAS.len() {
            self.over = true;
            out.push(Ev::Won);
            return;
        }
        let keep = 1.0 - self.wane;
        self.power *= keep;
        self.max_health *= keep;
        self.health = (self.health * keep + self.mercy * self.max_health).min(self.max_health);
        self.area += 1;
        self.x = 0.0;
        self.air = 0.0;
        self.populate();
        out.push(Ev::Gate { power_before });
    }
}
