use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::screens::{HelpScreen, SettingsScreen};
use hearken::Ctx;

use crate::{Heaven, Waning, HELP};

pub struct Title {
    menu: MenuCore<Self, Waning>,
}

impl Title {
    pub fn new() -> Self {
        Title {
            menu: MenuCore::new("Waning"),
        }
    }
}

impl Default for Title {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for Title {
    type Game = Waning;

    fn name(&self) -> &'static str {
        "title"
    }

    fn menu(&self) -> &MenuCore<Self, Waning> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Waning> {
        &mut self.menu
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Waning>) -> Vec<MenuItem<Self, Waning>> {
        vec![
            MenuItem::new("Play", |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                ctx.reset_to(Heaven::new())
            })
            .help("Go to heaven, where each run starts."),
            MenuItem::new("Settings", |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                ctx.push(SettingsScreen::<Waning>::new())
            })
            .help("Speech, volume and the controls."),
            MenuItem::new("Help", |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                ctx.push(HelpScreen::<Waning>::new(HELP))
            })
            .help("How to play."),
            MenuItem::new("Quit", |_s: &mut Self, ctx: &mut Ctx<Waning>| ctx.quit())
                .help("Close the game."),
        ]
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Waning>) {
        ctx.say("Press Enter on Quit to leave.");
    }
}
