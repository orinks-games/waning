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

    /// Grace for the next level, from level `level`: 60, 180, 360, 600, 900.
    pub fn cost(self, level: u32) -> u32 {
        60 * (level + 1) * (level + 2) / 2
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
    /// The music playing, shared by every screen.
    #[serde(skip)]
    pub jukebox: crate::music::Jukebox,
    /// The gear just chosen from what lay on the ground.
    #[serde(skip)]
    pub chosen_gear: Option<u32>,
    /// The run as the merchant left it.
    #[serde(skip)]
    pub world_back: Option<Box<crate::world::World>>,
}

/// The run in progress, as it stood when its current stage began.
const RUN_FILE: &str = "run.json";

pub fn save_run(vault: &mut hearken::seal::Vault, w: &crate::world::World) {
    match serde_json::to_vec(w) {
        Ok(bytes) => {
            if let Err(e) = vault.write(RUN_FILE, &bytes) {
                log::warn!("could not save the run: {e}");
            }
        }
        Err(e) => log::warn!("could not encode the run: {e}"),
    }
}

pub fn load_run(vault: &mut hearken::seal::Vault) -> Option<crate::world::World> {
    let bytes = vault.read(RUN_FILE).trusted()?;
    serde_json::from_slice(&bytes)
        .map_err(|e| log::warn!("could not read the saved run: {e}"))
        .ok()
}

pub fn clear_run(vault: &mut hearken::seal::Vault) {
    if load_run(vault).is_some() {
        if let Err(e) = vault.remove(RUN_FILE) {
            log::warn!("could not remove the saved run: {e}");
        }
    }
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
