//! Every positioned loop, kept in step with the world.

use hearken::{Bus, Ctx, Sound};

use super::{Play, HEARING};
use crate::world::BossMode;
use crate::Waning;

/// Starts, moves or stops one loop: playing while `near`, at `x`.
fn keep(ctx: &mut Ctx<Waning>, s: &mut Sound, near: bool, key: &str, x: f32, fade: u32) {
    if near && *s == Sound::NONE {
        *s = ctx.audio.start_loop(key, Some(Play::at(x)), Bus::EFFECTS);
    } else if near {
        ctx.audio.set_position(*s, Some(Play::at(x)));
    } else if *s != Sound::NONE {
        ctx.audio.stop(*s, fade);
        *s = Sound::NONE;
    }
}

impl Play {
    /// Starts, moves and stops every positioned loop to match the world.
    pub(super) fn place_loops(&mut self, ctx: &mut Ctx<Waning>) {
        let w = &self.world;
        let px = w.x;
        self.loops.resize(w.enemies.len(), Sound::NONE);
        for (i, e) in w.enemies.iter().enumerate() {
            let near = e.alive && (e.x - px).abs() <= HEARING;
            let fresh = self.loops[i] == Sound::NONE;
            let key = format!("loop/{}", e.kind.key());
            keep(ctx, &mut self.loops[i], near, &key, e.x, 400);
            if near && fresh && e.elite {
                // An elite sounds bigger: lower and louder.
                ctx.audio.set_pitch(self.loops[i], 0.8);
            }
        }
        for b in &w.bolts {
            let key = if b.from_boss {
                "loop/boss_bolt"
            } else {
                "loop/bolt"
            };
            let s = self.bolts.entry(b.id).or_insert(Sound::NONE);
            keep(ctx, s, true, key, b.x, 80);
        }

        // Gear on the ground shimmers until it is picked up.
        self.drops.retain(|id, s| {
            let here = w.drops.iter().any(|d| d.id == *id);
            if !here {
                ctx.audio.stop(*s, 200);
            }
            here
        });
        for d in &w.drops {
            let s = self.drops.entry(d.id).or_insert(Sound::NONE);
            keep(ctx, s, (d.x - px).abs() <= HEARING, "loop/loot", d.x, 300);
        }

        self.pits.resize(w.pits.len(), Sound::NONE);
        for (i, p) in w.pits.iter().enumerate() {
            let x = p.centre();
            keep(
                ctx,
                &mut self.pits[i],
                (x - px).abs() <= 20.0,
                "loop/pit",
                x,
                400,
            );
        }

        self.features.resize(w.features.len(), Sound::NONE);
        for (i, f) in w.features.iter().enumerate() {
            let near = !f.used && (f.x - px).abs() <= HEARING;
            let key = format!("loop/{}", f.kind.key());
            keep(ctx, &mut self.features[i], near, &key, f.x, 400);
        }

        match w.boss.as_ref() {
            Some(b) if b.present() => {
                let key = format!("boss/{}_loop", b.kind.key());
                keep(ctx, &mut self.boss_loop, true, &key, b.x, 300);
                let charging = matches!(b.mode, BossMode::Charging { .. });
                keep(
                    ctx,
                    &mut self.charge_loop,
                    charging,
                    "loop/charge",
                    b.x,
                    250,
                );
            }
            _ => {
                for s in [&mut self.boss_loop, &mut self.charge_loop] {
                    keep(ctx, s, false, "", 0.0, 300);
                }
            }
        }
    }
}
