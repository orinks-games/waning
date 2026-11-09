//! Walking, running, dashing, and the ground that hurts: pits and spikes.

use super::{
    BossMode, Ev, Relic, World, BODY, BOSS_BODY, DASH_COOLDOWN, DASH_GUARD, DASH_SPEED, DASH_TIME,
    FALL_COST, RUN_FACTOR, SPIKE_CYCLE, SPIKE_REACH, SPIKE_WARN, WALK_SPEED,
};

impl World {
    fn walk_speed(&self, run: bool) -> f32 {
        let mut speed = WALK_SPEED;
        if self.has(Relic::QuickStep) {
            speed *= 1.33;
        }
        if self.charge >= 0.0 {
            // Holding a heavy blow, you shuffle.
            speed *= 0.5;
        } else if run {
            speed *= RUN_FACTOR;
        }
        speed
    }

    /// The living bodies that stop you on the ground, with their sizes.
    fn bodies(&self) -> Vec<(f32, f32)> {
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
        bodies
    }

    /// Landing inside an enemy or the boss pushes you clear of it.
    pub(super) fn land_clear(&mut self) {
        for (ex, body) in self.bodies() {
            let d = ex - self.x;
            if d.abs() < body {
                let away = if d == 0.0 { -self.facing } else { -d.signum() };
                self.x = (self.x + away * (body - d.abs())).clamp(0.0, self.length());
            }
        }
    }

    pub(super) fn walk(&mut self, dt: f32, dir: i8, run: bool, out: &mut Vec<Ev>) {
        if dir == 0 {
            self.step_in = 0.0;
            return;
        }
        let dir = dir.signum() as f32;
        self.facing = dir;
        if self.blocking() {
            // Turning is all a raised block allows.
            return;
        }
        let running = run && self.charge < 0.0;
        let mut x = self.x + dir * self.walk_speed(run) * dt;
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
            // jump or dash past it.
            for (ex, body) in self.bodies() {
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
                out.push(Ev::Step { run: running });
                self.step_in = if running { 0.26 } else { 0.4 };
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

    /// A dash goes the way you hold, else the way you face.
    pub(super) fn start_dash(&mut self, dir: i8) {
        if self.dash_cd > 0.0 || self.airborne() || self.dashing() {
            return;
        }
        if dir != 0 {
            self.facing = dir.signum() as f32;
        }
        self.dash = DASH_TIME;
        self.dash_dir = self.facing;
        self.dash_cd = DASH_COOLDOWN;
        self.guard = DASH_GUARD;
        self.block = -1.0;
        self.charge = -1.0;
    }

    /// Dashing passes through enemies and over pits.
    pub(super) fn dash_move(&mut self, dt: f32) {
        let t = dt.min(self.dash);
        self.dash -= dt;
        let end = if self.gate_open() {
            self.length()
        } else {
            self.length() - 0.01
        };
        self.x = (self.x + self.dash_dir * DASH_SPEED * t).clamp(0.0, end);
        if !self.dashing() {
            self.dash = 0.0;
            self.land_clear();
        }
    }

    /// Standing over a pit drops you in; you climb back out on the side
    /// you came from.
    pub(super) fn fall_check(&mut self, out: &mut Vec<Ev>) {
        if self.airborne() || self.dashing() {
            return;
        }
        let Some(p) = self
            .pits
            .iter()
            .find(|p| self.x > p.x && self.x < p.x + p.width)
        else {
            return;
        };
        self.x = if self.facing > 0.0 {
            p.x - 0.6
        } else {
            p.x + p.width + 0.6
        };
        self.health -= FALL_COST * self.max_health;
        self.step_in = 0.5;
        out.push(Ev::Fell);
    }

    /// Each trap clicks, then rises a moment later.
    pub(super) fn spikes_act(&mut self, dt: f32, out: &mut Vec<Ev>) {
        let warn_at = SPIKE_CYCLE - SPIKE_WARN * self.warn;
        let mut hits = Vec::new();
        for s in &mut self.spikes {
            let before = s.t;
            s.t += dt;
            if before < warn_at && s.t >= warn_at {
                out.push(Ev::SpikeWarn { x: s.x });
            }
            if s.t >= SPIKE_CYCLE {
                s.t -= SPIKE_CYCLE;
                out.push(Ev::SpikeUp { x: s.x });
                if (s.x - self.x).abs() <= SPIKE_REACH {
                    hits.push(s.x);
                }
            }
        }
        for _ in hits {
            if self.evasive() {
                out.push(Ev::Dodged);
            } else {
                let damage = 8.0 + 2.0 * self.stage as f32;
                self.take_hit(damage, None, None, out);
            }
        }
    }
}
