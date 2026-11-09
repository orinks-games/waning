//! The enemies of the walking areas.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Slime,
    Shade,
    Ogre,
    /// Keeps its distance and throws bolts.
    Wraith,
    /// A chest that bites.
    Mimic,
    /// Called up by a boss: weaker than a slime, worth nothing.
    Spawn,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Slime => "Slime",
            Kind::Shade => "Shade",
            Kind::Ogre => "Ogre",
            Kind::Wraith => "Wraith",
            Kind::Mimic => "Mimic",
            Kind::Spawn => "Spawn",
        }
    }
    /// The sound family: `loop/<key>`, `sfx/<key>_cry1..3`, `sfx/<key>_die`.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Slime | Kind::Spawn => "slime",
            Kind::Shade => "shade",
            Kind::Ogre => "ogre",
            Kind::Wraith => "wraith",
            Kind::Mimic => "mimic",
        }
    }
    pub(crate) fn health(self) -> f32 {
        match self {
            Kind::Slime => 30.0,
            Kind::Shade => 45.0,
            Kind::Ogre => 140.0,
            Kind::Wraith => 35.0,
            Kind::Mimic => 90.0,
            Kind::Spawn => 20.0,
        }
    }
    pub(crate) fn damage(self) -> f32 {
        match self {
            Kind::Slime => 10.0,
            Kind::Shade => 14.0,
            Kind::Ogre => 28.0,
            Kind::Wraith => 12.0,
            Kind::Mimic => 22.0,
            Kind::Spawn => 8.0,
        }
    }
    pub(crate) fn speed(self) -> f32 {
        match self {
            Kind::Slime | Kind::Spawn => 1.3,
            Kind::Shade => 2.6,
            Kind::Ogre => 0.9,
            Kind::Wraith => 1.8,
            Kind::Mimic => 2.0,
        }
    }
    /// Seconds between the cry and the blow: the time you have to jump or
    /// step back.
    pub fn windup(self) -> f32 {
        match self {
            Kind::Slime | Kind::Spawn => 1.0,
            Kind::Shade => 0.7,
            Kind::Ogre => 1.3,
            Kind::Wraith => 0.8,
            Kind::Mimic => 0.8,
        }
    }
    pub(crate) fn souls(self) -> u32 {
        match self {
            Kind::Slime => 3,
            Kind::Shade => 5,
            Kind::Ogre => 12,
            Kind::Wraith => 6,
            Kind::Mimic => 25,
            Kind::Spawn => 0,
        }
    }
    /// How far a blow from you throws it back.
    pub(crate) fn knockback(self) -> f32 {
        match self {
            Kind::Ogre | Kind::Mimic => 0.3,
            _ => 1.5,
        }
    }
    /// Whether it fights from a distance.
    pub(crate) fn ranged(self) -> bool {
        self == Kind::Wraith
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Has not noticed you.
    Idle,
    Approach,
    /// Crying out; strikes or throws when the timer runs out.
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

impl Enemy {
    pub fn new(kind: Kind, x: f32, depth: usize) -> Enemy {
        let health = kind.health() * (1.0 + 0.2 * depth as f32);
        Enemy {
            kind,
            x,
            health,
            max_health: health,
            damage: kind.damage() * (1.0 + 0.15 * depth as f32),
            mode: Mode::Idle,
            alive: true,
        }
    }
}
