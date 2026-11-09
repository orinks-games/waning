//! A simple player for recordings and balance tests: walks right, swings at
//! what is in reach, jumps most warnings, and takes the first relic offered.

use crate::world::{BossMode, Ev, Gear, Input, Mode, Relic, WareKind, World};

pub struct Autopilot {
    judged: Option<(usize, u32)>,
    dodge: bool,
    luck: u64,
    pick: Option<Relic>,
    /// Where it last pressed Use, so it presses once per thing.
    used_at: f32,
    /// One warning in this many is missed.
    pub miss_one_in: u64,
    counter: u32,
}

impl Autopilot {
    pub fn new(seed: u64) -> Self {
        Autopilot {
            judged: None,
            dodge: true,
            luck: seed | 1,
            pick: None,
            used_at: -100.0,
            miss_one_in: 5,
            counter: 0,
        }
    }

    pub fn offered(&mut self, offer: Vec<Relic>) {
        self.pick = offer.first().copied();
    }

    pub fn take_pick(&mut self) -> Option<Relic> {
        self.pick.take()
    }

    /// Answers what a run asks of the player: relics, gear and the
    /// merchant, where it stocks up on potions and nothing else.
    pub fn react(&mut self, w: &mut World, ev: &Ev) {
        match ev {
            Ev::RelicOffer(offer) => self.offered(offer.clone()),
            Ev::GearOffer { id } => {
                let found = w.drop_by_id(*id).map(|d| d.gear.clone());
                if let Some(found) = found {
                    if self.wants(&found, w.worn(found.slot)) {
                        w.equip(*id, &mut Vec::new());
                    }
                }
            }
            Ev::Shop => {
                if let Some(i) = w.shop.iter().position(|s| s.kind == WareKind::Potion) {
                    while w.buy(i, &mut Vec::new()).is_ok() {}
                }
            }
            _ => {}
        }
    }

    /// Takes found gear when it is rarer, or as rare and stronger.
    pub fn wants(&self, found: &Gear, worn: Option<&Gear>) -> bool {
        worn.is_none_or(|w| {
            found.rarity > w.rarity
                || (found.rarity == w.rarity && found.perk == w.perk && found.value > w.value)
        })
    }

    fn roll(&mut self, id: (usize, u32)) -> bool {
        if self.judged != Some(id) {
            self.judged = Some(id);
            self.luck = self
                .luck
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.dodge = !(self.luck >> 33).is_multiple_of(self.miss_one_in.max(1));
        }
        self.dodge
    }

    fn ready_to_use(&mut self, x: f32) -> bool {
        if (self.used_at - x).abs() < 0.01 {
            return false;
        }
        self.used_at = x;
        true
    }

    pub fn input(&mut self, w: &World) -> Input {
        let mut input = Input {
            walk: 1,
            ..Input::default()
        };
        for f in w.features.iter().filter(|f| !f.used) {
            if (f.x - w.x).abs() < 1.5 && self.ready_to_use(f.x) {
                input.use_ = true;
            }
        }
        for d in &w.drops {
            if (d.x - w.x).abs() < 1.5 && self.ready_to_use(d.x) {
                input.use_ = true;
            }
        }
        if w.potions > 0 && w.health < 0.35 * w.max_health {
            input.drink = true;
        }
        // Jumps a pit from just before its edge.
        for p in &w.pits {
            let d = p.x - w.x;
            if (0.0..0.5).contains(&d) {
                input.jump = true;
            }
        }
        // Bolts: jump as one arrives.
        for b in &w.bolts {
            let d = b.x - w.x;
            if !b.passed && d * b.dir < 0.0 && d.abs() < 1.6 && self.roll((1000 + b.id as usize, 0))
            {
                input.jump = true;
            }
        }
        if let Some(b) = w.boss.as_ref().filter(|b| b.alive) {
            let d = b.x - w.x;
            if b.present() && d.abs() < 2.6 {
                input.walk = 0;
                input.attack = true;
            } else if !b.present() {
                input.walk = 0;
            } else {
                input.walk = d.signum() as i8;
            }
            match b.mode {
                BossMode::WindUp(crate::world::Attack::Slam, t) if t < 0.35 => {
                    self.counter += 1;
                    if self.roll((999, b.next as u32)) {
                        input.jump = true;
                    }
                }
                BossMode::Charging { .. } if d.abs() < 2.5 && self.roll((998, b.next as u32)) => {
                    input.jump = true;
                }
                _ => {}
            }
        }
        if let Some((i, d)) = w.nearest() {
            let e = &w.enemies[i];
            if d.abs() < 2.0 {
                input.walk = 0;
                input.attack = true;
                if d * w.facing < 0.0 {
                    input.walk = d.signum() as i8;
                }
            }
            if let Mode::WindUp(t) = e.mode {
                if t < 0.35 && !e.kind.ranged() && self.roll((i, w.stage as u32)) {
                    input.jump = true;
                }
            }
        }
        input
    }
}
