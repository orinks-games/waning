//! Choosing a relic, after a boss or at an altar.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::Ctx;

use crate::world::Relic;
use crate::Waning;

pub struct RelicChoice {
    menu: MenuCore<Self, Waning>,
    offer: Vec<Relic>,
}

impl RelicChoice {
    pub fn new(title: &str, offer: Vec<Relic>) -> Self {
        RelicChoice {
            menu: MenuCore::new(title)
                .with_intro_help("Enter takes the relic. It lasts until this run ends."),
            offer,
        }
    }
}

impl Menu for RelicChoice {
    type Game = Waning;

    fn name(&self) -> &'static str {
        "relic choice"
    }

    fn menu(&self) -> &MenuCore<Self, Waning> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Waning> {
        &mut self.menu
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Waning>) -> Vec<MenuItem<Self, Waning>> {
        self.offer
            .iter()
            .map(|&r| {
                MenuItem::new(
                    format!("{}: {}", r.name(), r.describe()),
                    move |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                        ctx.game.picked = Some(r);
                        ctx.pop_with(format!("{} is yours", r.name()));
                    },
                )
            })
            .collect()
    }

    /// A relic is free; leaving without one is never what the player wants
    /// from a stray Escape.
    fn on_escape(&mut self, ctx: &mut Ctx<Waning>) {
        ctx.say("Choose one with Enter.");
    }
}
