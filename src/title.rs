use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::screens::{HelpScreen, SettingsScreen};
use hearken::Ctx;

use crate::progress::load_run;
use crate::{music, Heaven, Play, Waning, HELP};

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

    fn enter(&mut self, ctx: &mut Ctx<Waning>) {
        if !ctx.game.loaded {
            ctx.game = crate::Progress::load(&mut ctx.vault);
        }
        music::play(ctx, "music/title");
        hearken::menu::menu_enter(self, ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Waning>) -> Vec<MenuItem<Self, Waning>> {
        let mut items = Vec::new();
        if let Some(w) = load_run(&mut ctx.vault) {
            let help = format!(
                "Back to the start of {}, stage {} of 12.",
                w.stage_name(),
                w.stage + 1
            );
            items.push(
                MenuItem::new(
                    "Continue descent",
                    |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                        if let Some(w) = load_run(&mut ctx.vault) {
                            ctx.reset_to(Play::new(w));
                        }
                    },
                )
                .help(help),
            );
        }
        items.extend([
            MenuItem::new("Play", |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                ctx.reset_to(Heaven::new())
            })
            .help("Go to heaven, where each run starts."),
            MenuItem::new("Settings", |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                ctx.push(SettingsScreen::<Waning>::new())
            })
            .help("Speech, volume, the controls, warning time and game speed."),
            MenuItem::new("Help", |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                ctx.push(HelpScreen::<Waning>::new(HELP))
            })
            .help("How to play."),
            MenuItem::new("Quit", |_s: &mut Self, ctx: &mut Ctx<Waning>| ctx.quit())
                .help("Close the game."),
        ]);
        items
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Waning>) {
        ctx.say("Press Enter on Quit to leave.");
    }
}
