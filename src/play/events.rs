//! What each event in the world sounds like, and the few that are spoken.

use hearken::{Bus, Ctx, Sound};

use super::{record_depth, shown_health, Play};
use crate::bag::GearChoice;
use crate::progress::clear_run;
use crate::relics::RelicChoice;
use crate::shop::Shop;
use crate::story;
use crate::summary::Summary;
use crate::world::{Attack, Ev, Target, MAX_POTIONS};
use crate::{music, Progress, Waning};

impl Play {
    fn target_x(&self, t: Target) -> f32 {
        match t {
            Target::Enemy(i) => self.world.enemies[i].x,
            Target::Boss => self.world.boss.as_ref().map_or(self.world.x, |b| b.x),
        }
    }

    fn sfx(&self, ctx: &mut Ctx<Waning>, key: &str) -> Sound {
        ctx.audio.play(key, Bus::EFFECTS)
    }

    fn sfx_at(&self, ctx: &mut Ctx<Waning>, key: &str, x: f32) -> Sound {
        ctx.audio.play_at(key, Self::at(x), Bus::EFFECTS)
    }

    fn say_health(&self, ctx: &mut Ctx<Waning>, cut: bool) {
        if self.world.health > 0.0 {
            ctx.say_event(format!("Health {}.", shown_health(self.world.health)), cut);
        }
    }

    pub(super) fn handle(&mut self, ctx: &mut Ctx<Waning>, ev: Ev) {
        match ev {
            Ev::Step { run } => {
                let surface = if self.world.region() < 2 {
                    "grass"
                } else {
                    "stone"
                };
                self.stride = (self.stride + 1) % 6;
                let key = format!("sfx/step_{surface}{}", self.stride + 1);
                let s = self.sfx(ctx, &key);
                ctx.audio.set_volume(s, if run { 1.0 } else { 0.75 });
            }
            Ev::Wall => {
                self.sfx(ctx, "sfx/wall");
            }
            Ev::Bump => {
                self.sfx(ctx, "sfx/bump");
            }
            Ev::Swing => {
                let key = self.variant("sfx/swing", 4);
                self.sfx(ctx, &key);
            }
            Ev::Charged => {
                self.sfx(ctx, "sfx/charged");
            }
            Ev::Heavy => {
                self.sfx(ctx, "sfx/heavy");
            }
            Ev::Jump => {
                self.sfx(ctx, "sfx/jump");
            }
            Ev::Land => {
                self.sfx(ctx, "sfx/land");
            }
            Ev::Dash => {
                self.sfx(ctx, "sfx/dash");
            }
            Ev::Hit {
                target,
                killed,
                heavy,
                crit,
            } => self.hit(ctx, target, killed, heavy, crit),
            Ev::WindUp { i } => {
                let e = &self.world.enemies[i];
                let (x, elite, base) = (e.x, e.elite, format!("sfx/{}_cry", e.kind.key()));
                let key = self.variant(&base, 3);
                let s = self.sfx_at(ctx, &key, x);
                if elite {
                    ctx.audio.set_pitch(s, 0.85);
                }
            }
            Ev::Staggered { i } => {
                let x = self.world.enemies[i].x;
                self.sfx_at(ctx, "sfx/stagger", x);
            }
            Ev::Struck { .. } => {
                let key = self.variant("sfx/hurt", 3);
                self.sfx(ctx, &key);
                self.say_health(ctx, true);
            }
            Ev::Dodged => {}
            Ev::Blocked => {
                self.sfx(ctx, "sfx/block");
            }
            Ev::Parried => {
                self.sfx(ctx, "sfx/parry");
            }
            Ev::Warded => {
                self.sfx(ctx, "sfx/ward");
            }
            Ev::Thorns => {
                self.sfx(ctx, "sfx/thorns");
            }
            Ev::Ember => {
                self.sfx(ctx, "sfx/ember");
            }
            Ev::Souls(_) => {
                self.sfx(ctx, "sfx/soul");
            }
            Ev::Gold { x, .. } => {
                self.sfx_at(ctx, "sfx/coins", x);
            }
            Ev::Potion { x } => {
                self.sfx_at(ctx, "sfx/potion", x);
                ctx.say_event(
                    format!("Potion, {} of {MAX_POTIONS}.", self.world.potions),
                    false,
                );
            }
            Ev::Healed(_) => {
                self.sfx(ctx, "sfx/heal");
                self.say_health(ctx, false);
            }
            Ev::Drank => {
                self.sfx(ctx, "sfx/drink");
            }
            Ev::NoPotions => ctx.say("No potions."),
            Ev::FullHealth => ctx.say("Your health is full."),
            Ev::Dropped { x, .. } => {
                self.sfx_at(ctx, "sfx/loot_drop", x);
            }
            Ev::DropGone { .. } => {}
            Ev::Equipped(gear) => {
                self.sfx(ctx, "sfx/equip");
                ctx.say(format!("You wear the {}.", gear.line()));
            }
            Ev::GearOffer { id } => self.offer_gear(ctx, id),
            Ev::Fell => {
                self.sfx(ctx, "sfx/fall");
                self.say_health(ctx, true);
            }
            Ev::SpikeWarn { x } => {
                self.sfx_at(ctx, "sfx/spike_warn", x);
            }
            Ev::SpikeUp { x } => {
                self.sfx_at(ctx, "sfx/spike_up", x);
            }
            Ev::Fired { .. } => {}
            Ev::BoltGone { id, x, hit } => {
                if let Some(s) = self.bolts.remove(&id) {
                    ctx.audio.stop(s, 80);
                }
                if hit {
                    self.sfx_at(ctx, "sfx/bolt_hit", x);
                }
            }
            Ev::BossWindUp(attack) => {
                let Some(b) = self.world.boss.as_ref() else {
                    return;
                };
                let key = match attack {
                    Attack::Slam => format!("boss/{}_cry", b.kind.key()),
                    other => format!("boss/{}", other.key()),
                };
                let x = b.x;
                self.sfx_at(ctx, &key, x);
            }
            Ev::BossSlam | Ev::BossChargeEnd => {
                let x = self.target_x(Target::Boss);
                self.sfx_at(ctx, "boss/slam", x);
            }
            Ev::BossVanish { x } => {
                self.sfx_at(ctx, "boss/vanish", x);
            }
            Ev::BossAppear { x } => {
                self.sfx_at(ctx, "boss/appear", x);
            }
            Ev::Summoned(_) => {}
            Ev::BossEnraged => {
                if let Some(b) = self.world.boss.as_ref() {
                    let (key, x, kind) = (format!("boss/{}_cry", b.kind.key()), b.x, b.kind);
                    self.sfx_at(ctx, &key, x);
                    ctx.say(story::boss_enraged(kind));
                }
            }
            Ev::BossDefeated(kind) => {
                let x = self.target_x(Target::Boss);
                let key = format!("boss/{}_die", kind.key());
                self.sfx_at(ctx, &key, x);
                music::stop(ctx, 3000);
                self.playing = None;
                Progress::mark(&mut ctx.game.beaten, kind.key());
                ctx.game.save(&mut ctx.vault);
                ctx.say(story::boss_defeat(kind));
            }
            Ev::Mimic { i } => {
                let x = self.world.enemies[i].x;
                self.sfx_at(ctx, "sfx/mimic", x);
            }
            Ev::Chest => {
                self.sfx(ctx, "sfx/chest_open");
            }
            Ev::Altar => {
                self.sfx(ctx, "sfx/altar");
            }
            Ev::Shop => self.open_shop(ctx),
            Ev::RelicOffer(offer) => {
                self.sfx(ctx, "sfx/relic");
                let title = if self.world.boss.is_some() {
                    "The fallen leave a gift. Choose a relic"
                } else {
                    "The altar takes its price. Choose a relic"
                };
                if let Some(pilot) = self.autopilot.as_mut() {
                    pilot.offered(offer.clone());
                } else {
                    ctx.queued_speech(|ctx| ctx.push(RelicChoice::new(title, offer)));
                }
            }
            Ev::Gate {
                power_before,
                anchored,
            } => self.through_gate(ctx, power_before, anchored),
            Ev::Won => self.finish(ctx, true),
            Ev::Died => self.finish(ctx, false),
        }
    }

    fn hit(
        &mut self,
        ctx: &mut Ctx<Waning>,
        target: Target,
        killed: bool,
        heavy: bool,
        crit: bool,
    ) {
        let x = self.target_x(target);
        // A blow sounds as strong as you are: lower and quieter as you wane.
        let w = &self.world;
        let strength = (w.power / w.start_power).clamp(0.0, 1.0);
        let key = if heavy {
            "sfx/heavy_hit".to_string()
        } else {
            self.variant("sfx/hit", 3)
        };
        let hit = self.sfx_at(ctx, &key, x);
        ctx.audio.set_pitch(hit, 0.7 + 0.3 * strength);
        ctx.audio.set_volume(hit, 0.55 + 0.45 * strength);
        if crit {
            self.sfx_at(ctx, "sfx/crit", x);
        }
        match target {
            Target::Enemy(i) if killed => {
                let key = format!("sfx/{}_die", self.world.enemies[i].kind.key());
                self.sfx_at(ctx, &key, x);
                if let Some(s) = self.loops.get_mut(i) {
                    ctx.audio.stop(*s, 150);
                    *s = Sound::NONE;
                }
            }
            Target::Boss if !killed => {
                if let Some(b) = self.world.boss.as_ref() {
                    let key = format!("boss/{}_hurt", b.kind.key());
                    self.sfx_at(ctx, &key, x);
                }
            }
            _ => {}
        }
    }

    fn offer_gear(&mut self, ctx: &mut Ctx<Waning>, id: u32) {
        let Some(found) = self.world.drop_by_id(id).map(|d| d.gear.clone()) else {
            return;
        };
        if let Some(pilot) = self.autopilot.as_mut() {
            pilot.react(&mut self.world, &Ev::GearOffer { id });
            return;
        }
        let worn = self.world.worn(found.slot).cloned();
        ctx.push(GearChoice::new(id, found, worn));
    }

    fn open_shop(&mut self, ctx: &mut Ctx<Waning>) {
        if let Some(pilot) = self.autopilot.as_mut() {
            pilot.react(&mut self.world, &Ev::Shop);
            return;
        }
        ctx.audio.play("sfx/merchant", Bus::EFFECTS);
        ctx.push(Shop::new(self.world.clone()));
    }

    fn through_gate(&mut self, ctx: &mut Ctx<Waning>, power_before: f32, anchored: bool) {
        self.stop_stage_sounds(ctx);
        ctx.audio.play("sfx/gate", Bus::EFFECTS);
        let w = &self.world;
        record_depth(ctx, w.stage as u32 + 1);
        let line = if anchored {
            format!(
                "Through the gate. The Anchor holds. Power {}, health {} of {}.",
                w.power.round() as u32,
                shown_health(w.health),
                w.max_health.round() as u32,
            )
        } else {
            ctx.audio.play("sfx/wane", Bus::EFFECTS);
            format!(
                "Through the gate. Power {} to {}, health {} of {}.",
                power_before.round() as u32,
                w.power.round() as u32,
                shown_health(w.health),
                w.max_health.round() as u32,
            )
        };
        ctx.say(line);
        let stage = self.stage_line();
        ctx.say_queued(stage);
        self.begin_stage(ctx);
    }

    /// Souls and gold go home as grace, and the run's account follows.
    fn finish(&mut self, ctx: &mut Ctx<Waning>, won: bool) {
        self.stop_stage_sounds(ctx);
        let grace = self.world.souls + self.world.gold_souls();
        ctx.game.grace += grace;
        if won {
            ctx.game.wins += 1;
        }
        ctx.game.save(&mut ctx.vault);
        clear_run(&mut ctx.vault);
        ctx.audio
            .play(if won { "sfx/ascend" } else { "sfx/death" }, Bus::EFFECTS);
        ctx.replace(Summary::new(&self.world, won, grace));
    }
}
