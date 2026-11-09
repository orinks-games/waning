//! The merchant: potions, healing, a piece of gear and a relic, for gold.

use hearken::menu::{Label, Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Bus, Ctx};

use crate::music;
use crate::world::{Refusal, Ware, WareKind, World, MAX_POTIONS};
use crate::Waning;

pub struct Shop {
    menu: MenuCore<Self, Waning>,
    /// The run, while you shop; handed back when you leave.
    world: World,
    /// The music to go back to.
    before: String,
}

impl Shop {
    pub fn new(world: World) -> Self {
        let title = format!("Merchant. You have {} gold", world.gold);
        Shop {
            menu: MenuCore::new(title)
                .with_intro_help("Enter buys, F1 says what a ware does, Escape leaves. Gear you buy goes on at once."),
            world,
            before: String::new(),
        }
    }

    fn ware_label(ware: &Ware) -> String {
        let what = match &ware.kind {
            WareKind::Potion => "Potion".to_string(),
            WareKind::Mend => "Mending".to_string(),
            WareKind::Gear(g) => g.name(),
            WareKind::Relic(r) => r.name().to_string(),
        };
        if ware.sold {
            format!("{what}, sold")
        } else {
            format!("{what}, {} gold", ware.price)
        }
    }

    fn ware_help(ware: &Ware, w: &World) -> String {
        match &ware.kind {
            WareKind::Potion => format!(
                "Heals 35 percent of your health. You carry {} of {MAX_POTIONS}.",
                w.potions
            ),
            WareKind::Mend => "Heals you to full health.".into(),
            WareKind::Gear(g) => {
                let what = sentence_start(&g.describe());
                match w.worn(g.slot) {
                    Some(old) => format!("{what}. You wear a {}.", old.line()),
                    None => format!(
                        "{what}. Your {} slot is empty.",
                        g.slot.name().to_lowercase()
                    ),
                }
            }
            WareKind::Relic(r) => format!("{} It lasts until this run ends.", r.describe()),
        }
    }

    fn leave(&mut self, ctx: &mut Ctx<Waning>) {
        ctx.game.world_back = Some(Box::new(self.world.clone()));
        ctx.pop_with(format!("{} gold left", self.world.gold));
    }
}

impl Menu for Shop {
    type Game = Waning;

    fn name(&self) -> &'static str {
        "shop"
    }

    fn menu(&self) -> &MenuCore<Self, Waning> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Waning> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Waning>) {
        self.before = music::play(ctx, "music/shop");
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Waning>) {
        let before = std::mem::take(&mut self.before);
        music::play(ctx, &before);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Waning>) -> Vec<MenuItem<Self, Waning>> {
        let mut items: Vec<MenuItem<Self, Waning>> = (0..self.world.shop.len())
            .map(|i| {
                MenuItem::new(
                    Label::dynamic(move |s: &Self, _ctx: &Ctx<Waning>| {
                        Self::ware_label(&s.world.shop[i])
                    }),
                    move |s: &mut Self, ctx: &mut Ctx<Waning>| {
                        let mut out = Vec::new();
                        let line = match s.world.buy(i, &mut out) {
                            Ok(()) => {
                                ctx.audio.play("sfx/buy", Bus::UI);
                                let gold = s.world.gold;
                                match &s.world.shop[i].kind {
                                    WareKind::Potion => format!(
                                        "Potion bought, {} of {MAX_POTIONS}. {gold} gold left.",
                                        s.world.potions
                                    ),
                                    WareKind::Mend => {
                                        format!("You are mended. {gold} gold left.")
                                    }
                                    WareKind::Gear(g) => {
                                        format!("You wear the {}. {gold} gold left.", g.name())
                                    }
                                    WareKind::Relic(r) => {
                                        format!("{} is yours. {gold} gold left.", r.name())
                                    }
                                }
                            }
                            Err(Refusal::Short(n)) => format!("You need {n} more gold."),
                            Err(Refusal::Full) => match s.world.shop[i].kind {
                                WareKind::Potion => {
                                    format!("You carry {MAX_POTIONS} potions already.")
                                }
                                _ => "Your health is full.".into(),
                            },
                            Err(Refusal::Sold) => "Sold.".into(),
                        };
                        ctx.say(line);
                    },
                )
                .help(Label::dynamic(move |s: &Self, _ctx: &Ctx<Waning>| {
                    Self::ware_help(&s.world.shop[i], &s.world)
                }))
            })
            .collect();
        items.push(MenuItem::new(
            "Leave",
            |s: &mut Self, ctx: &mut Ctx<Waning>| s.leave(ctx),
        ));
        items
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Waning>) {
        ctx.play_ui(hearken::UiSound::Back);
        self.leave(ctx);
    }
}
