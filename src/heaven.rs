//! Heaven: where you start, where you return when you fall, and where grace
//! buys blessings for the next descent.

use hearken::menu::{Label, Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound};

use crate::progress::{Blessing, BuyError};
use crate::world::{World, AREAS};
use crate::{Play, Waning};

pub struct Heaven {
    menu: MenuCore<Self, Waning>,
    ambience: Sound,
    /// Said once, before the menu, when you arrive from a run.
    arrival: Option<String>,
}

impl Heaven {
    pub fn new() -> Self {
        Heaven {
            menu: MenuCore::new("Heaven")
                .with_intro_help("Buy blessings with grace, then descend."),
            ambience: Sound::NONE,
            arrival: None,
        }
    }

    pub fn after_run(line: String) -> Self {
        Heaven {
            arrival: Some(line),
            ..Heaven::new()
        }
    }

    fn blessing_row(b: Blessing) -> MenuItem<Self, Waning> {
        MenuItem::new(
            Label::dynamic(move |_s: &Self, ctx: &Ctx<Waning>| {
                let level = ctx.game.level(b);
                let max = b.max_level();
                if level >= max {
                    format!("{}, level {level} of {max}, complete", b.name())
                } else {
                    format!(
                        "{}, level {level} of {max}, next costs {}",
                        b.name(),
                        b.cost(level)
                    )
                }
            }),
            move |_s: &mut Self, ctx: &mut Ctx<Waning>| match ctx.game.buy(b) {
                Ok(level) => {
                    ctx.game.save(&mut ctx.vault);
                    ctx.audio.play("sfx/buy", Bus::UI);
                    let grace = ctx.game.grace;
                    ctx.say(format!(
                        "{} is level {level}. {grace} grace left.",
                        b.name()
                    ));
                }
                Err(BuyError::Maxed) => {
                    ctx.say(format!("{} is complete.", b.name()));
                }
                Err(BuyError::Short(n)) => {
                    ctx.say(format!("You need {n} more grace."));
                }
            },
        )
        .help(b.describe())
    }
}

impl Default for Heaven {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for Heaven {
    type Game = Waning;

    fn name(&self) -> &'static str {
        "heaven"
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
        self.ambience = ctx.audio.start_loop("amb/heaven", None, Bus::AMBIENCE);
        self.refresh(ctx, false);
        let grace = format!("Heaven. {} grace.", ctx.game.grace);
        match self.arrival.take() {
            Some(line) => {
                ctx.audio.play("sfx/ascend", Bus::EFFECTS);
                ctx.say(line);
                ctx.say_queued(grace);
            }
            None => ctx.say(grace),
        }
        let current = self.current_text(ctx);
        ctx.say_queued(current);
    }

    fn exit(&mut self, ctx: &mut Ctx<Waning>) {
        ctx.audio.stop(self.ambience, 800);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Waning>) -> Vec<MenuItem<Self, Waning>> {
        let mut items = vec![
            MenuItem::new("Descend", |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                ctx.game.runs += 1;
                ctx.game.save(&mut ctx.vault);
                let seed = ctx.game.runs as u64 ^ (ctx.clock * 1000.0) as u64;
                let world = World::new(&ctx.game, seed);
                ctx.replace(Play::new(world));
            })
            .help("Start a new run from the top."),
        ];
        items.extend(Blessing::ALL.into_iter().map(Self::blessing_row));
        let p = &ctx.game;
        if p.deepest > 0 {
            let deepest = AREAS[(p.deepest as usize - 1).min(AREAS.len() - 1)];
            let runs = match p.runs {
                1 => "1 run".to_string(),
                n => format!("{n} runs"),
            };
            items.push(MenuItem::inert(format!("Deepest: {deepest}, in {runs}")));
        }
        items.push(
            MenuItem::new("Title menu", |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                ctx.reset_to(crate::Title::new())
            })
            .help("Back to the title menu. Your grace and blessings are saved."),
        );
        items
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Waning>) {
        ctx.play_ui(hearken::UiSound::Back);
        ctx.reset_to(crate::Title::new());
    }
}
