//! Your gear: what you carry, and the choice when you find something new.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::Ctx;

use crate::world::{Gear, Slot, World, MAX_POTIONS};
use crate::Waning;

/// What you carry, one line each.
pub struct Inventory {
    menu: MenuCore<Self, Waning>,
    lines: Vec<String>,
}

impl Inventory {
    pub fn new(w: &World) -> Self {
        let mut lines = Vec::new();
        for slot in [Slot::Weapon, Slot::Armor, Slot::Charm] {
            lines.push(match w.worn(slot) {
                Some(g) => format!("{}: {}", slot.name(), g.line()),
                None => format!("{}: none", slot.name()),
            });
        }
        lines.push(format!("Potions: {} of {MAX_POTIONS}", w.potions));
        lines.push(format!(
            "Gold: {}, worth {} if you fall with it",
            w.gold,
            crate::world::souls(w.gold_souls())
        ));
        lines.push(format!("Souls: {}", w.souls));
        if w.relics.is_empty() {
            lines.push("No relics yet".into());
        }
        for r in &w.relics {
            lines.push(format!("{}: {}", r.name(), r.describe()));
        }
        Inventory {
            menu: MenuCore::new("Inventory"),
            lines,
        }
    }
}

impl Menu for Inventory {
    type Game = Waning;

    fn name(&self) -> &'static str {
        "inventory"
    }

    fn menu(&self) -> &MenuCore<Self, Waning> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Waning> {
        &mut self.menu
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Waning>) -> Vec<MenuItem<Self, Waning>> {
        self.lines
            .iter()
            .map(|l| MenuItem::inert(l.clone()))
            .collect()
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Waning>) {
        ctx.play_ui(hearken::UiSound::Back);
        ctx.pop_quiet();
    }
}

/// Gear at your feet, with gear already in its slot: take it or leave it.
pub struct GearChoice {
    menu: MenuCore<Self, Waning>,
    id: u32,
    found: Gear,
    worn: Option<Gear>,
}

impl GearChoice {
    pub fn new(id: u32, found: Gear, worn: Option<Gear>) -> Self {
        GearChoice {
            menu: MenuCore::new(found.line()),
            id,
            found,
            worn,
        }
    }
}

impl Menu for GearChoice {
    type Game = Waning;

    fn name(&self) -> &'static str {
        "gear choice"
    }

    fn menu(&self) -> &MenuCore<Self, Waning> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Waning> {
        &mut self.menu
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Waning>) -> Vec<MenuItem<Self, Waning>> {
        let take = match &self.worn {
            Some(old) => format!("Take it, and drop your {}", old.line()),
            None => "Take it".to_string(),
        };
        vec![
            MenuItem::new(take, |s: &mut Self, ctx: &mut Ctx<Waning>| {
                ctx.game.chosen_gear = Some(s.id);
                ctx.pop_with(format!("You take the {}", s.found.name()));
            }),
            MenuItem::new("Leave it", |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                ctx.pop_quiet()
            }),
        ]
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Waning>) {
        ctx.play_ui(hearken::UiSound::Back);
        ctx.pop_quiet();
    }
}
