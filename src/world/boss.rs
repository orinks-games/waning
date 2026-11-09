//! The four bosses: one at the bottom of each region.

use super::{Enemy, Ev, Kind, Mode, Target, World, ARENA_LENGTH, CHARGE_SPEED, SLAM_REACH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossKind {
    Shepherd,
    Mother,
    Ferryman,
    King,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attack {
    /// Walks up and brings its weapon down: jump, or be out of reach.
    Slam,
    /// Rushes across the arena: jump as it reaches you.
    Charge,
    /// Throws a bolt that travels toward you: jump as it arrives.
    Volley,
    /// Calls up two spawn.
    Summon,
    /// Vanishes and comes back somewhere else.
    Vanish,
}

impl BossKind {
    pub fn name(self) -> &'static str {
        match self {
            BossKind::Shepherd => "The Shepherd",
            BossKind::Mother => "The Weeping Mother",
            BossKind::Ferryman => "The Ferryman",
            BossKind::King => "The Hollow King",
        }
    }
    /// The sound family: `boss/<key>_loop`, `_cry`, `_hurt`, `_die`.
    pub fn key(self) -> &'static str {
        match self {
            BossKind::Shepherd => "shepherd",
            BossKind::Mother => "mother",
            BossKind::Ferryman => "ferryman",
            BossKind::King => "king",
        }
    }
    pub(crate) fn health(self) -> f32 {
        match self {
            BossKind::Shepherd => 550.0,
            BossKind::Mother => 700.0,
            BossKind::Ferryman => 850.0,
            BossKind::King => 1150.0,
        }
    }
    pub(crate) fn damage(self) -> f32 {
        match self {
            BossKind::Shepherd => 20.0,
            BossKind::Mother => 20.0,
            BossKind::Ferryman => 25.0,
            BossKind::King => 28.0,
        }
    }
    pub(crate) fn attacks(self) -> &'static [Attack] {
        match self {
            BossKind::Shepherd => &[Attack::Slam, Attack::Charge, Attack::Slam, Attack::Summon],
            BossKind::Mother => &[Attack::Vanish, Attack::Volley, Attack::Slam, Attack::Summon],
            BossKind::Ferryman => &[Attack::Volley, Attack::Slam, Attack::Charge, Attack::Volley],
            BossKind::King => &[
                Attack::Slam,
                Attack::Charge,
                Attack::Volley,
                Attack::Summon,
                Attack::Vanish,
            ],
        }
    }
    pub(crate) fn souls(self) -> u32 {
        match self {
            BossKind::Shepherd => 40,
            BossKind::Mother => 60,
            BossKind::Ferryman => 80,
            BossKind::King => 150,
        }
    }
}

impl Attack {
    /// Seconds from the warning sound to the blow, before phase two
    /// shortens it.
    pub(crate) fn windup(self) -> f32 {
        match self {
            Attack::Slam => 1.2,
            Attack::Charge => 1.0,
            Attack::Volley => 0.8,
            Attack::Summon => 1.0,
            Attack::Vanish => 0.6,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Attack::Slam => "slam",
            Attack::Charge => "charge",
            Attack::Volley => "volley",
            Attack::Summon => "summon",
            Attack::Vanish => "vanish",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BossMode {
    /// Between attacks; counts down to the next one.
    Idle(f32),
    /// Walking up for a slam.
    Closing,
    WindUp(Attack, f32),
    /// Rushing toward `to`; `hit` once it has struck you this charge.
    Charging {
        to: f32,
        hit: bool,
    },
    /// Gone; comes back when the timer runs out.
    Vanished(f32),
}

#[derive(Clone, Debug)]
pub struct Boss {
    pub kind: BossKind,
    pub x: f32,
    pub health: f32,
    pub max_health: f32,
    pub damage: f32,
    pub mode: BossMode,
    /// Below half health: faster wind-ups and two bolts a volley.
    pub enraged: bool,
    pub alive: bool,
    pub(crate) next: usize,
}

impl Boss {
    pub fn new(kind: BossKind, x: f32) -> Boss {
        Boss {
            kind,
            x,
            health: kind.health(),
            max_health: kind.health(),
            damage: kind.damage(),
            mode: BossMode::Idle(2.5),
            enraged: false,
            alive: true,
            next: 0,
        }
    }

    pub fn present(&self) -> bool {
        self.alive && !matches!(self.mode, BossMode::Vanished(_))
    }

    pub(crate) fn windup(&self, attack: Attack) -> f32 {
        attack.windup() * if self.enraged { 0.75 } else { 1.0 }
    }
}

impl World {
    pub(super) fn boss_act(&mut self, dt: f32, out: &mut Vec<Ev>) {
        let px = self.x;
        let airborne = self.airborne();
        let Some(mut b) = self.boss.take() else {
            return;
        };
        if !b.alive {
            self.boss = Some(b);
            return;
        }
        let d = px - b.x;
        let mut slam = false;
        let mut charge_hit = false;
        let mut volley = 0;
        let mut summon = false;
        match b.mode {
            BossMode::Idle(t) => {
                // Drifts to a few metres from you between attacks.
                if d.abs() > 4.0 {
                    b.x += d.signum() * 1.0 * dt;
                }
                let t = t - dt;
                if t > 0.0 {
                    b.mode = BossMode::Idle(t);
                } else {
                    let attacks = b.kind.attacks();
                    let mut attack = attacks[b.next % attacks.len()];
                    b.next += 1;
                    let spawn = self.enemies.iter().filter(|e| e.alive).count();
                    if attack == Attack::Summon && spawn >= 2 {
                        attack = attacks[b.next % attacks.len()];
                        b.next += 1;
                    }
                    if attack == Attack::Slam {
                        b.mode = BossMode::Closing;
                    } else {
                        b.mode = BossMode::WindUp(attack, b.windup(attack));
                        out.push(Ev::BossWindUp(attack));
                    }
                }
            }
            BossMode::Closing => {
                if d.abs() <= 2.0 {
                    b.mode = BossMode::WindUp(Attack::Slam, b.windup(Attack::Slam));
                    out.push(Ev::BossWindUp(Attack::Slam));
                } else {
                    b.x += d.signum() * 2.2 * dt;
                }
            }
            BossMode::WindUp(attack, t) => {
                let t = t - dt;
                if t > 0.0 {
                    b.mode = BossMode::WindUp(attack, t);
                } else {
                    match attack {
                        Attack::Slam => {
                            slam = d.abs() <= SLAM_REACH && !airborne;
                            out.push(Ev::BossSlam);
                            if !slam {
                                out.push(Ev::Dodged);
                            }
                            b.mode = BossMode::Idle(1.8);
                        }
                        Attack::Charge => {
                            let to = if d >= 0.0 { ARENA_LENGTH - 1.0 } else { 1.0 };
                            b.mode = BossMode::Charging { to, hit: false };
                        }
                        Attack::Volley => {
                            volley = if b.enraged { 2 } else { 1 };
                            b.mode = BossMode::Idle(2.2);
                        }
                        Attack::Summon => {
                            summon = true;
                            b.mode = BossMode::Idle(2.0);
                        }
                        Attack::Vanish => {
                            out.push(Ev::BossVanish { x: b.x });
                            b.mode = BossMode::Vanished(1.6);
                        }
                    }
                }
            }
            BossMode::Charging { to, hit } => {
                let before = px - b.x;
                let step = (to - b.x).signum() * CHARGE_SPEED * dt;
                b.x += step;
                let after = px - b.x;
                let mut hit = hit;
                if !hit && (before.signum() != after.signum() || after.abs() < 0.6) {
                    hit = true;
                    if airborne {
                        out.push(Ev::Dodged);
                    } else {
                        charge_hit = true;
                    }
                }
                if (to - b.x).abs() <= CHARGE_SPEED * dt {
                    b.x = to;
                    out.push(Ev::BossChargeEnd);
                    b.mode = BossMode::Idle(2.2);
                } else {
                    b.mode = BossMode::Charging { to, hit };
                }
            }
            BossMode::Vanished(t) => {
                let t = t - dt;
                if t > 0.0 {
                    b.mode = BossMode::Vanished(t);
                } else {
                    // Comes back six metres to whichever side has room.
                    let side =
                        if px + 6.0 < ARENA_LENGTH - 1.0 && (self.rng.unit() < 0.5 || px < 7.0) {
                            1.0
                        } else {
                            -1.0
                        };
                    b.x = (px + side * 6.0).clamp(1.0, ARENA_LENGTH - 1.0);
                    out.push(Ev::BossAppear { x: b.x });
                    b.mode = BossMode::Idle(1.2);
                }
            }
        }
        let (bx, damage) = (b.x, b.damage);
        self.boss = Some(b);
        if slam || charge_hit {
            self.take_hit(damage, Some(Target::Boss), out);
        }
        for n in 0..volley {
            let dir = if px >= bx { 1.0 } else { -1.0 };
            // A second bolt follows three metres behind the first.
            let x = bx - dir * 3.0 * n as f32;
            self.throw(x, dir, damage * 0.7, true, out);
        }
        if summon {
            let mut made = Vec::new();
            for side in [-2.0, 2.0] {
                let x = (bx + side).clamp(0.5, ARENA_LENGTH - 0.5);
                let mut e = Enemy::new(Kind::Spawn, x, self.stage);
                e.mode = Mode::Approach;
                self.enemies.push(e);
                made.push(self.enemies.len() - 1);
            }
            out.push(Ev::Summoned(made));
        }
    }
}
