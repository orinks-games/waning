//! What carries over between runs: grace, blessings and the record.

use serde::{Deserialize, Serialize};

const FILE: &str = "progress.json";

/// The blessings heaven sells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blessing {
    Might,
    Vigor,
    Steadfast,
    Mercy,
}

impl Blessing {
    pub const ALL: [Blessing; 4] = [
        Blessing::Might,
        Blessing::Vigor,
        Blessing::Steadfast,
        Blessing::Mercy,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Blessing::Might => "Might",
            Blessing::Vigor => "Vigor",
            Blessing::Steadfast => "Steadfast",
            Blessing::Mercy => "Mercy",
        }
    }

    pub fn max_level(self) -> u32 {
        match self {
            Blessing::Mercy => 3,
            _ => 5,
        }
    }

    /// What one level does, for the row's help.
    pub fn describe(self) -> &'static str {
        match self {
            Blessing::Might => "Each level adds 15 to the power you start with.",
            Blessing::Vigor => "Each level adds 25 to the health you start with.",
            Blessing::Steadfast => {
                "Each gate takes 15 percent of your power and health. Each level lowers that by 2 and a half."
            }
            Blessing::Mercy => "Each level heals 15 percent of your health at every gate.",
        }
    }

    /// Grace for the next level, from level `level`: 40, 120, 240, 400, 600.
    pub fn cost(self, level: u32) -> u32 {
        40 * (level + 1) * (level + 2) / 2
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    pub grace: u32,
    pub might: u32,
    pub vigor: u32,
    pub steadfast: u32,
    pub mercy: u32,
    /// Runs started.
    pub runs: u32,
    /// The deepest stage reached, counted from 1. 0 before the first run.
    pub deepest: u32,
    /// Runs that passed the last gate.
    #[serde(default)]
    pub wins: u32,
    /// Bosses met and beaten, by key, on any run.
    #[serde(default)]
    pub met: Vec<String>,
    #[serde(default)]
    pub beaten: Vec<String>,
    /// Story beats the Keeper has already told.
    #[serde(default)]
    pub heard: Vec<String>,
    /// The relic just chosen from an offer, for the run to take.
    #[serde(skip)]
    pub picked: Option<crate::world::Relic>,
    /// Whether the save was read this session.
    #[serde(skip)]
    pub loaded: bool,
}

impl Progress {
    pub fn level(&self, b: Blessing) -> u32 {
        match b {
            Blessing::Might => self.might,
            Blessing::Vigor => self.vigor,
            Blessing::Steadfast => self.steadfast,
            Blessing::Mercy => self.mercy,
        }
    }

    fn level_mut(&mut self, b: Blessing) -> &mut u32 {
        match b {
            Blessing::Might => &mut self.might,
            Blessing::Vigor => &mut self.vigor,
            Blessing::Steadfast => &mut self.steadfast,
            Blessing::Mercy => &mut self.mercy,
        }
    }

    /// Buys the next level of `b`, if it is affordable and not maxed.
    pub fn buy(&mut self, b: Blessing) -> Result<u32, BuyError> {
        let level = self.level(b);
        if level >= b.max_level() {
            return Err(BuyError::Maxed);
        }
        let cost = b.cost(level);
        if self.grace < cost {
            return Err(BuyError::Short(cost - self.grace));
        }
        self.grace -= cost;
        *self.level_mut(b) += 1;
        Ok(level + 1)
    }

    pub fn mark(list: &mut Vec<String>, id: &str) {
        if !list.iter().any(|x| x == id) {
            list.push(id.to_string());
        }
    }

    pub fn load(vault: &mut hearken::seal::Vault) -> Progress {
        let mut p = vault
            .read(FILE)
            .trusted()
            .and_then(|bytes| serde_json::from_slice::<Progress>(&bytes).ok())
            .unwrap_or_default();
        p.loaded = true;
        p
    }

    pub fn save(&self, vault: &mut hearken::seal::Vault) {
        match serde_json::to_vec_pretty(self) {
            Ok(bytes) => {
                if let Err(e) = vault.write(FILE, &bytes) {
                    log::warn!("could not save progress: {e}");
                }
            }
            Err(e) => log::warn!("could not encode progress: {e}"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BuyError {
    Maxed,
    /// Grace still needed.
    Short(u32),
}
