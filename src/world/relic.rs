//! Relics: what a run finds on the way down. They last until the run ends.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relic {
    EmberEdge,
    ThornMail,
    FeatherBoots,
    HungryBlade,
    MisersPurse,
    QuickStep,
    EchoCharm,
    WardStone,
    LastEmber,
    Anchor,
}

impl Relic {
    pub const ALL: [Relic; 10] = [
        Relic::EmberEdge,
        Relic::ThornMail,
        Relic::FeatherBoots,
        Relic::HungryBlade,
        Relic::MisersPurse,
        Relic::QuickStep,
        Relic::EchoCharm,
        Relic::WardStone,
        Relic::LastEmber,
        Relic::Anchor,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Relic::EmberEdge => "Ember Edge",
            Relic::ThornMail => "Thorn Mail",
            Relic::FeatherBoots => "Feather Boots",
            Relic::HungryBlade => "Hungry Blade",
            Relic::MisersPurse => "Miser's Purse",
            Relic::QuickStep => "Quick Step",
            Relic::EchoCharm => "Echo Charm",
            Relic::WardStone => "Ward Stone",
            Relic::LastEmber => "Last Ember",
            Relic::Anchor => "Anchor",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Relic::EmberEdge => "Your blows deal a quarter more damage.",
            Relic::ThornMail => "Anything that strikes you takes a third of your power in return.",
            Relic::FeatherBoots => "Jumps last longer.",
            Relic::HungryBlade => "Each kill heals 4 health.",
            Relic::MisersPurse => "Enemies drop half again as many souls.",
            Relic::QuickStep => "You walk a third faster.",
            Relic::EchoCharm => "Every third swing strikes twice.",
            Relic::WardStone => "The first blow in each area is turned aside.",
            Relic::LastEmber => "Once, a killing blow leaves you at 1 health instead.",
            Relic::Anchor => "The next gate takes none of your strength.",
        }
    }
}
