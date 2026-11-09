//! A simple player for recordings and balance tests: walks right, swings at
//! what is in reach, jumps most warnings, and takes the first relic offered.

use crate::world::{BossMode, Input, Mode, Relic, World};

pub struct Autopilot {
    judged: Option<(usize, u32)>,
    dodge: bool,
    luck: u64,
    pick: Option<Relic>,
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

    pub fn input(&mut self, w: &World) -> Input {
        let mut input = Input {
            walk: 1,
            ..Input::default()
        };
        if let Some(f) = w.feature.as_ref().filter(|f| !f.used) {
            if (f.x - w.x).abs() < 1.5 {
                input.use_ = true;
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
