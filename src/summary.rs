//! The end of a run: what it found, before heaven.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx};

use crate::world::{World, STAGES};
use crate::{music, Heaven, Waning};

pub struct Summary {
    menu: MenuCore<Self, Waning>,
    won: bool,
    lines: Vec<String>,
}

impl Summary {
    /// `grace` is what the run brought home: its souls and its gold.
    pub fn new(w: &World, won: bool, grace: u32) -> Self {
        let rise = if grace == 1 { "rises" } else { "rise" };
        let title = if won {
            format!(
                "You pass the last gate. {} {rise} with you",
                crate::world::souls(grace)
            )
        } else {
            format!(
                "You fall in {}. {} {rise} with you",
                w.stage_name(),
                crate::world::souls(grace)
            )
        };
        let mut lines = vec![format!("Souls gathered: {}", w.souls)];
        if w.gold > 0 {
            lines.push(format!(
                "Gold carried: {}, worth {}",
                w.gold,
                crate::world::souls(w.gold_souls())
            ));
        }
        lines.push(format!(
            "Reached: stage {} of {}",
            w.stage + 1,
            STAGES.len()
        ));
        lines.push(format!("Kills: {}", w.kills));
        let bosses: Vec<&str> = w.beaten.iter().map(|b| b.name()).collect();
        lines.push(if bosses.is_empty() {
            "No bosses beaten".to_string()
        } else {
            format!("Bosses beaten: {}", bosses.join(", "))
        });
        Summary {
            menu: MenuCore::new(title),
            won,
            lines,
        }
    }

    fn rise(ctx: &mut Ctx<Waning>) {
        ctx.audio.play("sfx/ascend", Bus::EFFECTS);
        ctx.reset_to(Heaven::new());
    }
}

impl Menu for Summary {
    type Game = Waning;

    fn name(&self) -> &'static str {
        "summary"
    }

    fn menu(&self) -> &MenuCore<Self, Waning> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Waning> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Waning>) {
        music::play(
            ctx,
            if self.won {
                "music/victory"
            } else {
                "music/summary"
            },
        );
        hearken::menu::menu_enter(self, ctx);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Waning>) -> Vec<MenuItem<Self, Waning>> {
        let mut items: Vec<MenuItem<Self, Waning>> = self
            .lines
            .iter()
            .map(|l| MenuItem::inert(l.clone()))
            .collect();
        items.push(MenuItem::new(
            "Rise to heaven",
            |_s: &mut Self, ctx: &mut Ctx<Waning>| Self::rise(ctx),
        ));
        items
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Waning>) {
        Self::rise(ctx);
    }
}
