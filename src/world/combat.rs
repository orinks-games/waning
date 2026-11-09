//! Swinging, heavy blows, blocking, and every blow that lands on you.

use super::{
    Bolt, Ev, Input, Mode, Perk, Relic, Target, World, ATTACK_COOLDOWN, BLOCK_SHARE, BODY,
    BOLT_SPEED, HEAVY_CHARGE, HEAVY_FACTOR, NOTICE_RANGE, PARRY_WINDOW, REACH, STRIKE_RANGE,
    STRIKE_REACH,
};

impl World {
    /// A press swings at once. Holding on afterwards charges a heavy blow,
    /// let go when the key comes up.
    pub(super) fn attack(&mut self, dt: f32, input: Input, out: &mut Vec<Ev>) {
        if self.dashing() {
            return;
        }
        if input.attack && self.cooldown <= 0.0 && !self.blocking() {
            self.swing(false, out);
            self.charge = 0.0;
            return;
        }
        if self.charge < 0.0 {
            return;
        }
        if input.attack_held && !self.blocking() {
            let before = self.charge;
            self.charge += dt;
            if before < HEAVY_CHARGE && self.charge >= HEAVY_CHARGE {
                out.push(Ev::Charged);
            }
        } else {
            let ready = self.charge >= HEAVY_CHARGE;
            self.charge = -1.0;
            if ready && !self.blocking() {
                self.swing(true, out);
            }
        }
    }

    /// Your power, which the gates take from, plus your weapon, which they
    /// do not.
    fn blow(&self) -> f32 {
        let edge = if self.has(Relic::EmberEdge) {
            1.25
        } else {
            1.0
        };
        self.power * edge + self.weapon.as_ref().map_or(0.0, |g| g.value)
    }

    pub(super) fn perk(&self, perk: Perk) -> f32 {
        self.charm
            .as_ref()
            .filter(|g| g.perk == Some(perk))
            .map_or(0.0, |g| g.value)
    }

    fn swing(&mut self, heavy: bool, out: &mut Vec<Ev>) {
        self.cooldown = if heavy { 0.6 } else { ATTACK_COOLDOWN };
        self.swings += 1;
        out.push(if heavy { Ev::Heavy } else { Ev::Swing });
        let facing = self.facing;
        let mut damage = self.blow();
        if heavy {
            damage *= HEAVY_FACTOR;
        }
        if self.has(Relic::EchoCharm) && self.swings.is_multiple_of(3) {
            damage *= 2.0;
        }
        let crit = self.rng.unit() * 100.0 < self.perk(Perk::Crit);
        if crit {
            damage *= 2.0;
        }
        let reach = REACH + if heavy { 0.6 } else { 0.0 };
        let enemies = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.alive)
            .map(|(i, e)| (Target::Enemy(i), (e.x - self.x) * facing))
            .filter(|&(_, ahead)| ahead > -0.3 && ahead <= reach);
        let boss = self
            .boss
            .as_ref()
            .filter(|b| b.present())
            .map(|b| (Target::Boss, (b.x - self.x) * facing))
            .filter(|&(_, ahead)| ahead > -0.5 && ahead <= reach + 0.6);
        let mut targets: Vec<(Target, f32)> = enemies.chain(boss).collect();
        targets.sort_by(|a, b| a.1.total_cmp(&b.1));
        // A heavy blow cuts through everything in reach; a swing, the
        // nearest.
        if !heavy {
            targets.truncate(1);
        }
        let leech = self.perk(Perk::Leech);
        for (target, _) in targets {
            match target {
                Target::Enemy(i) => self.hit_enemy(i, damage, heavy, crit, out),
                Target::Boss => self.hit_boss(damage, heavy, crit, out),
            }
            if leech > 0.0 {
                self.health = (self.health + leech).min(self.max_health);
            }
        }
    }

    pub(super) fn soul_share(&self, base: u32) -> u32 {
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

    pub(super) fn hit_enemy(
        &mut self,
        i: usize,
        damage: f32,
        heavy: bool,
        crit: bool,
        out: &mut Vec<Ev>,
    ) {
        let facing = self.facing;
        let end = self.length();
        let e = &mut self.enemies[i];
        e.health -= damage;
        if e.health <= 0.0 {
            e.alive = false;
            let (kind, elite, x) = (e.kind, e.elite, e.x);
            out.push(Ev::Hit {
                target: Target::Enemy(i),
                killed: true,
                heavy,
                crit,
            });
            let souls = kind.souls() * if elite { 3 } else { 1 };
            self.on_kill(souls, out);
            self.enemy_loot(kind, elite, x, out);
            return;
        }
        // A blow that does not kill knocks it back; only a heavy one breaks
        // a wind-up.
        let push = e.kind.knockback() * if heavy { 2.5 } else { 1.0 };
        e.x = (e.x + facing * push).clamp(0.0, end);
        out.push(Ev::Hit {
            target: Target::Enemy(i),
            killed: false,
            heavy,
            crit,
        });
        if heavy && matches!(e.mode, Mode::WindUp(_)) {
            e.mode = Mode::Recover(1.0);
            out.push(Ev::Staggered { i });
        }
    }

    pub(super) fn hit_boss(&mut self, damage: f32, heavy: bool, crit: bool, out: &mut Vec<Ev>) {
        let Some(b) = self.boss.as_mut() else {
            return;
        };
        b.health -= damage;
        if b.health <= 0.0 {
            b.alive = false;
            let (kind, x) = (b.kind, b.x);
            out.push(Ev::Hit {
                target: Target::Boss,
                killed: true,
                heavy,
                crit,
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
            self.boss_loot(x, out);
            let offer = self.offer();
            if !offer.is_empty() {
                out.push(Ev::RelicOffer(offer));
            }
            return;
        }
        out.push(Ev::Hit {
            target: Target::Boss,
            killed: false,
            heavy,
            crit,
        });
        if !b.enraged && b.health <= b.max_health / 2.0 {
            b.enraged = true;
            out.push(Ev::BossEnraged);
        }
    }

    /// A blow lands on you, unless you block it or a relic stops it. `from`
    /// is who to hurt with Thorn Mail or stagger with a parry, and `from_x`
    /// where the blow comes from; a blow with no `from_x` comes from below
    /// and cannot be blocked.
    pub(super) fn take_hit(
        &mut self,
        damage: f32,
        from: Option<Target>,
        from_x: Option<f32>,
        out: &mut Vec<Ev>,
    ) {
        let mut damage = damage;
        if let Some(fx) = from_x.filter(|_| self.blocking()) {
            if (fx - self.x) * self.facing >= -0.3 {
                if self.block <= PARRY_WINDOW {
                    out.push(Ev::Parried);
                    self.parried(from, out);
                    return;
                }
                damage *= BLOCK_SHARE;
                out.push(Ev::Blocked);
            }
        }
        if self.ward_ready && self.has(Relic::WardStone) {
            self.ward_ready = false;
            out.push(Ev::Warded);
            return;
        }
        let armor = self.armor.as_ref().map_or(0.0, |g| g.value);
        damage *= 1.0 - armor / 100.0;
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
                        self.hit_enemy(i, thorns, false, false, out);
                    }
                }
                Target::Boss => self.hit_boss(thorns, false, false, out),
            }
        }
    }

    /// A parried enemy reels; a parried boss loses its next move.
    fn parried(&mut self, from: Option<Target>, out: &mut Vec<Ev>) {
        match from {
            Some(Target::Enemy(i)) => {
                if let Some(e) = self.enemies.get_mut(i).filter(|e| e.alive) {
                    e.mode = Mode::Recover(1.8);
                    out.push(Ev::Staggered { i });
                }
            }
            Some(Target::Boss) => {
                if let Some(b) = self.boss.as_mut() {
                    b.mode = super::BossMode::Idle(2.5);
                }
            }
            None => {}
        }
    }

    pub(super) fn enemies_act(&mut self, dt: f32, out: &mut Vec<Ev>) {
        let px = self.x;
        let evasive = self.evasive();
        let end = self.length();
        let warn = self.warn;
        let mut strikes = Vec::new();
        for (i, e) in self.enemies.iter_mut().enumerate() {
            if !e.alive {
                continue;
            }
            let d = px - e.x;
            let windup = e.kind.windup() * warn;
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
                        e.mode = Mode::WindUp(windup);
                        out.push(Ev::WindUp { i });
                    }
                }
                Mode::Approach => {
                    if d.abs() <= STRIKE_RANGE {
                        e.mode = Mode::WindUp(windup);
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
                        let hit = d.abs() <= STRIKE_REACH && !evasive;
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
                self.take_hit(damage, Some(Target::Enemy(i)), Some(x), out);
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
        let id = self.id();
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

    pub(super) fn bolts_fly(&mut self, dt: f32, out: &mut Vec<Ev>) {
        let px = self.x;
        let evasive = self.evasive();
        let end = self.length();
        let mut hits = Vec::new();
        self.bolts.retain_mut(|b| {
            let before = b.x - px;
            b.x += b.dir * BOLT_SPEED * dt;
            let after = b.x - px;
            if !b.passed && (before.signum() != after.signum() || after.abs() < 0.3) {
                if evasive {
                    b.passed = true;
                    out.push(Ev::Dodged);
                } else {
                    // Where it came from, for a block.
                    hits.push((b.damage, b.from_boss, px - b.dir));
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
        for (damage, from_boss, from_x) in hits {
            let from = from_boss.then_some(Target::Boss);
            self.take_hit(damage, from, Some(from_x), out);
        }
    }
}
