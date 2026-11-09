//! Gold, potions, gear and the merchant, and the wells, altars and chests
//! along the way.

use serde::{Deserialize, Serialize};

use super::{
    pick, Enemy, Ev, Feature, FeatureKind, Kind, Mode, Relic, World, MAX_POTIONS, POTION_HEAL,
    USE_RANGE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Slot {
    Weapon,
    Armor,
    Charm,
}

impl Slot {
    pub fn name(self) -> &'static str {
        match self {
            Slot::Weapon => "Weapon",
            Slot::Armor => "Armor",
            Slot::Charm => "Charm",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Rare,
    Legendary,
}

impl Rarity {
    pub fn name(self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Rare => "Rare",
            Rarity::Legendary => "Legendary",
        }
    }
    fn tier(self) -> usize {
        self as usize
    }
}

/// What a charm does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Perk {
    /// A chance, in percent, of a blow striking twice as hard.
    Crit,
    /// Health back for each blow that lands.
    Leech,
    /// Percent more gold.
    Fortune,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gear {
    pub slot: Slot,
    pub rarity: Rarity,
    /// Set on charms only.
    pub perk: Option<Perk>,
    /// Damage for a weapon, percent of each blow for armor, and the perk's
    /// amount for a charm.
    pub value: f32,
    pub noun: String,
}

impl Gear {
    /// "Rare mail shirt"
    pub fn name(&self) -> String {
        format!("{} {}", self.rarity.name(), self.noun)
    }

    /// "adds 12 damage to every blow"
    pub fn describe(&self) -> String {
        let v = self.value.round() as u32;
        match (self.slot, self.perk) {
            (Slot::Weapon, _) => format!("adds {v} damage to every blow"),
            (Slot::Armor, _) => format!("takes {v} percent off every blow"),
            (_, Some(Perk::Crit)) => format!("{v} percent of your blows strike twice as hard"),
            (_, Some(Perk::Leech)) => format!("every blow you land heals {v}"),
            (_, Some(Perk::Fortune)) | (_, None) => format!("enemies drop {v} percent more gold"),
        }
    }

    /// "Rare mail shirt: takes 16 percent off every blow"
    pub fn line(&self) -> String {
        format!("{}: {}", self.name(), self.describe())
    }
}

/// Gear lying on the ground.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Drop {
    pub id: u32,
    pub x: f32,
    pub gear: Gear,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WareKind {
    Potion,
    /// Heals you to full.
    Mend,
    Gear(Gear),
    Relic(Relic),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ware {
    pub kind: WareKind,
    pub price: u32,
    /// Potions never sell out; everything else sells once.
    pub sold: bool,
}

/// Why the merchant would not sell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Gold still needed.
    Short(u32),
    /// You carry all the potions you can, or your health is full.
    Full,
    Sold,
}

const WEAPONS: [&str; 4] = ["knife", "sword", "axe", "greatsword"];
const ARMOR: [&str; 4] = ["leather coat", "mail shirt", "scale coat", "plate"];

impl World {
    fn gold_find(&mut self, base: f32) -> u32 {
        let fortune = 1.0 + self.perk(Perk::Fortune) / 100.0;
        let depth = 1.0 + 0.15 * self.stage as f32;
        (base * depth * fortune * self.rng.range(0.8, 1.2)).round() as u32
    }

    /// New gear for this depth, at least `floor` rare.
    fn roll_gear(&mut self, floor: Rarity) -> Gear {
        let depth = self.stage as f32;
        let roll = self.rng.unit();
        let legendary = 0.03 + 0.01 * depth;
        let rare = legendary + 0.25 + 0.02 * depth;
        let rarity = if roll < legendary {
            Rarity::Legendary
        } else if roll < rare {
            Rarity::Rare
        } else {
            Rarity::Common
        }
        .max(floor);
        let tier = rarity.tier();
        let region = self.region();
        let slot = pick(
            self.rng.unit(),
            &[(0.4, Slot::Weapon), (0.75, Slot::Armor), (1.0, Slot::Charm)],
        );
        let (perk, value, noun) = match slot {
            Slot::Weapon => {
                let v = (3.0 + 1.0 * depth) * [1.0, 1.6, 2.4][tier];
                (None, v.round(), WEAPONS[region].to_string())
            }
            Slot::Armor => {
                let v = ([6.0, 12.0, 20.0][tier] + 0.4 * depth).min(35.0);
                (None, v.round(), ARMOR[region].to_string())
            }
            Slot::Charm => {
                let perk = pick(
                    self.rng.unit(),
                    &[(0.4, Perk::Crit), (0.7, Perk::Leech), (1.0, Perk::Fortune)],
                );
                let (v, noun) = match perk {
                    Perk::Crit => ([8.0, 14.0, 22.0][tier], "hawk charm"),
                    Perk::Leech => ([1.0, 2.0, 4.0][tier], "leech charm"),
                    Perk::Fortune => ([25.0, 50.0, 90.0][tier], "lucky coin"),
                };
                (Some(perk), v, noun.to_string())
            }
        };
        Gear {
            slot,
            rarity,
            perk,
            value,
            noun,
        }
    }

    fn drop_gear(&mut self, gear: Gear, x: f32, out: &mut Vec<Ev>) {
        let id = self.id();
        let x = x.clamp(0.5, self.length() - 1.0);
        self.drops.push(Drop { id, x, gear });
        out.push(Ev::Dropped { id, x });
    }

    /// Gold always, now and then a potion, and gear from elites, mimics and
    /// the odd ordinary enemy.
    pub(super) fn enemy_loot(&mut self, kind: Kind, elite: bool, x: f32, out: &mut Vec<Ev>) {
        if kind == Kind::Spawn {
            return;
        }
        let base = kind.gold() * if elite { 3.0 } else { 1.0 };
        let amount = self.gold_find(base);
        self.gold += amount;
        out.push(Ev::Gold { amount, x });
        if self.potions < MAX_POTIONS && self.rng.unit() < 0.05 {
            self.potions += 1;
            out.push(Ev::Potion { x });
        }
        let floor = if kind == Kind::Mimic {
            Some(Rarity::Rare)
        } else if elite || self.rng.unit() < 0.07 {
            Some(Rarity::Common)
        } else {
            None
        };
        if let Some(floor) = floor {
            let gear = self.roll_gear(floor);
            self.drop_gear(gear, x, out);
        }
    }

    /// A boss leaves rare gear or better, and a pile of gold.
    pub(super) fn boss_loot(&mut self, x: f32, out: &mut Vec<Ev>) {
        let amount = self.gold_find(60.0);
        self.gold += amount;
        out.push(Ev::Gold { amount, x });
        let gear = self.roll_gear(Rarity::Rare);
        self.drop_gear(gear, x, out);
    }

    pub(super) fn drink(&mut self, out: &mut Vec<Ev>) {
        if self.potions == 0 {
            out.push(Ev::NoPotions);
        } else if self.health >= self.max_health {
            out.push(Ev::FullHealth);
        } else {
            self.potions -= 1;
            let healed = (POTION_HEAL * self.max_health).min(self.max_health - self.health);
            self.health += healed;
            out.push(Ev::Drank);
            out.push(Ev::Healed(healed));
        }
    }

    /// The gear slot `slot` holds.
    pub fn worn(&self, slot: Slot) -> Option<&Gear> {
        match slot {
            Slot::Weapon => self.weapon.as_ref(),
            Slot::Armor => self.armor.as_ref(),
            Slot::Charm => self.charm.as_ref(),
        }
    }

    fn worn_mut(&mut self, slot: Slot) -> &mut Option<Gear> {
        match slot {
            Slot::Weapon => &mut self.weapon,
            Slot::Armor => &mut self.armor,
            Slot::Charm => &mut self.charm,
        }
    }

    /// Puts on `gear` and returns what it replaced.
    fn wear(&mut self, gear: Gear) -> Option<Gear> {
        self.worn_mut(gear.slot).replace(gear)
    }

    /// Takes the gear on the ground with `id`, leaving what it replaces in
    /// its place.
    pub fn equip(&mut self, id: u32, out: &mut Vec<Ev>) {
        let Some(i) = self.drops.iter().position(|d| d.id == id) else {
            return;
        };
        let drop = self.drops.remove(i);
        out.push(Ev::DropGone { id });
        if let Some(old) = self.wear(drop.gear) {
            self.drop_gear(old, drop.x, out);
        }
    }

    pub fn drop_by_id(&self, id: u32) -> Option<&Drop> {
        self.drops.iter().find(|d| d.id == id)
    }

    /// Uses the nearest thing in reach: gear at your feet, or a well,
    /// altar, chest or merchant.
    pub(super) fn use_here(&mut self, out: &mut Vec<Ev>) {
        let px = self.x;
        let drop = self
            .drops
            .iter()
            .map(|d| (d.id, (d.x - px).abs()))
            .filter(|&(_, d)| d <= USE_RANGE)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let feature = self
            .features
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.used)
            .map(|(i, f)| (i, (f.x - px).abs()))
            .filter(|&(_, d)| d <= USE_RANGE)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match (drop, feature) {
            (Some((id, dd)), f) if f.is_none_or(|(_, fd)| dd <= fd) => {
                let slot = self.drop_by_id(id).map(|d| d.gear.slot);
                if slot.is_some_and(|s| self.worn(s).is_none()) {
                    let gear = self.drop_by_id(id).map(|d| d.gear.clone());
                    self.equip(id, out);
                    if let Some(gear) = gear {
                        out.push(Ev::Equipped(gear));
                    }
                } else {
                    out.push(Ev::GearOffer { id });
                }
            }
            (_, Some((i, _))) => self.use_feature(i, out),
            _ => {}
        }
    }

    fn use_feature(&mut self, i: usize, out: &mut Vec<Ev>) {
        let Feature { kind, x, .. } = self.features[i].clone();
        if kind != FeatureKind::Merchant {
            self.features[i].used = true;
        }
        match kind {
            FeatureKind::Well => {
                let healed = (0.35 * self.max_health).min(self.max_health - self.health);
                self.health += healed;
                out.push(Ev::Healed(healed));
            }
            FeatureKind::Altar => {
                self.health = (self.health * 0.75).max(1.0);
                out.push(Ev::Altar);
                let offer = self.offer();
                if !offer.is_empty() {
                    out.push(Ev::RelicOffer(offer));
                }
            }
            FeatureKind::Chest => {
                if self.rng.unit() < 0.3 {
                    let mut m = Enemy::new(Kind::Mimic, x, self.stage, false);
                    m.mode = Mode::WindUp(Kind::Mimic.windup() * self.warn);
                    self.enemies.push(m);
                    out.push(Ev::Mimic {
                        i: self.enemies.len() - 1,
                    });
                } else {
                    out.push(Ev::Chest);
                    let souls = self.soul_share(10);
                    self.souls += souls;
                    out.push(Ev::Souls(souls));
                    let amount = self.gold_find(15.0);
                    self.gold += amount;
                    out.push(Ev::Gold { amount, x });
                    if self.rng.unit() < 0.5 {
                        let gear = self.roll_gear(Rarity::Common);
                        self.drop_gear(gear, x, out);
                    }
                }
            }
            FeatureKind::Merchant => out.push(Ev::Shop),
        }
    }

    /// What this stage's merchant sells.
    pub(super) fn stock(&mut self) {
        let region = self.region() as u32;
        let mut wares = vec![
            Ware {
                kind: WareKind::Potion,
                price: 25 + 10 * region,
                sold: false,
            },
            Ware {
                kind: WareKind::Mend,
                price: 30 + 10 * region,
                sold: false,
            },
        ];
        let gear = self.roll_gear(Rarity::Rare);
        let premium = if gear.rarity == Rarity::Legendary {
            40
        } else {
            0
        };
        wares.push(Ware {
            kind: WareKind::Gear(gear),
            price: 60 + 25 * region + premium,
            sold: false,
        });
        if let Some(&relic) = self.offer().first() {
            wares.push(Ware {
                kind: WareKind::Relic(relic),
                price: 90 + 30 * region,
                sold: false,
            });
        }
        self.shop = wares;
    }

    /// Buys ware `i` from the merchant. Gear goes on at once; what it
    /// replaces is left at your feet.
    pub fn buy(&mut self, i: usize, out: &mut Vec<Ev>) -> Result<(), Refusal> {
        let Some(ware) = self.shop.get(i).cloned() else {
            return Err(Refusal::Sold);
        };
        if ware.sold {
            return Err(Refusal::Sold);
        }
        match ware.kind {
            WareKind::Potion if self.potions >= MAX_POTIONS => return Err(Refusal::Full),
            WareKind::Mend if self.health >= self.max_health => return Err(Refusal::Full),
            _ => {}
        }
        if self.gold < ware.price {
            return Err(Refusal::Short(ware.price - self.gold));
        }
        self.gold -= ware.price;
        match ware.kind {
            WareKind::Potion => self.potions += 1,
            WareKind::Mend => {
                self.health = self.max_health;
                self.shop[i].sold = true;
            }
            WareKind::Gear(gear) => {
                self.shop[i].sold = true;
                if let Some(old) = self.wear(gear) {
                    let x = self.x;
                    self.drop_gear(old, x, out);
                }
            }
            WareKind::Relic(r) => {
                self.shop[i].sold = true;
                self.take(r);
            }
        }
        Ok(())
    }
}
