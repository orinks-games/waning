//! Heaven: where you start, where you return when you fall, and where grace
//! buys blessings for the next descent.

use hearken::menu::{Label, Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx};

use crate::progress::{Blessing, BuyError, Progress};
use crate::story;
use crate::world::{World, STAGES};
use crate::{music, Play, Waning};

pub struct Heaven {
    menu: MenuCore<Self, Waning>,
}

impl Heaven {
    pub fn new() -> Self {
        Heaven {
            menu: MenuCore::new("Heaven")
                .with_intro_help("Buy blessings with grace, then descend."),
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
        music::play(ctx, "music/heaven");
        self.refresh(ctx, false);
        ctx.say(format!("Heaven. {} grace.", ctx.game.grace));
        // The Keeper speaks to what you have reached, once each.
        let p = &mut ctx.game;
        let mut keeper = None;
        if !p.heard.iter().any(|h| h == "opening") {
            Progress::mark(&mut p.heard, "opening");
            keeper = Some(story::OPENING);
        } else if let Some(beat) = story::unheard(p) {
            for b in story::Beat::ALL {
                if b.reached(p) {
                    Progress::mark(&mut p.heard, b.id());
                }
            }
            keeper = Some(beat.line());
        }
        if let Some(line) = keeper {
            ctx.game.save(&mut ctx.vault);
            ctx.say_queued(line);
        }
        let current = self.current_text(ctx);
        ctx.say_queued(current);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Waning>) -> Vec<MenuItem<Self, Waning>> {
        let descend_help = if crate::progress::load_run(&mut ctx.vault).is_some() {
            "Start a new run from the top. It replaces the descent you left."
        } else {
            "Start a new run from the top."
        };
        let mut items = vec![
            MenuItem::new("Descend", |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                ctx.game.runs += 1;
                ctx.game.save(&mut ctx.vault);
                let seed = ctx.game.runs as u64 ^ (ctx.clock * 1000.0) as u64;
                let world = World::new(&ctx.game, seed);
                ctx.replace(Play::new(world));
            })
            .help(descend_help),
        ];
        items.extend(Blessing::ALL.into_iter().map(Self::blessing_row));
        items.push(
            MenuItem::new("The Keeper", |_s: &mut Self, ctx: &mut Ctx<Waning>| {
                let line = story::latest(&ctx.game).map_or(story::OPENING, |b| b.line());
                ctx.say(line);
            })
            .help("Hear the Keeper again."),
        );
        let p = &ctx.game;
        if p.deepest > 0 {
            let deepest = STAGES[(p.deepest as usize - 1).min(STAGES.len() - 1)].name;
            let runs = match p.runs {
                1 => "1 run".to_string(),
                n => format!("{n} runs"),
            };
            let wins = match p.wins {
                0 => String::new(),
                1 => ", 1 descent to the bottom".to_string(),
                n => format!(", {n} descents to the bottom"),
            };
            items.push(MenuItem::inert(format!(
                "Deepest: {deepest}, in {runs}{wins}"
            )));
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
