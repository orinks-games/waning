//! Waning: an audio sidescroller. You descend from heaven at full strength,
//! every gate you pass takes some of it, and when you fall the souls you
//! gathered buy blessings for the next descent.

pub mod autopilot;
mod bag;
mod heaven;
mod music;
mod options;
mod play;
mod progress;
mod relics;
mod rng;
mod shop;
pub mod story;
mod summary;
mod title;
pub mod world;

use hearken::screens::SettingsRow;
use hearken::{Action, Chord, ControllerButton, Ctx, Key, PadChord};

pub use heaven::Heaven;
pub use options::Options;
pub use play::Play;
pub use progress::{Blessing, Progress};
pub use title::Title;

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

pub struct Waning;

impl hearken::Game for Waning {
    const TITLE: &'static str = "Waning";
    const DATA_DIR: &'static str = "Waning";
    const ENV_PREFIX: &'static str = "WANING";
    type Event = ();
    type Data = Progress;
    type Settings = Options;
    type Action = Control;
    type Category = hearken::NoCategory;

    fn settings_rows(_ctx: &Ctx<Self>, rows: Vec<SettingsRow<Self>>) -> Vec<SettingsRow<Self>> {
        options::rows(rows)
    }
}

/// The controls the player can move to other keys and buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Control {
    Left,
    Right,
    Attack,
    Jump,
    Dash,
    Block,
    Drink,
    Use,
    Inventory,
    Status,
    Nearest,
}

impl Action for Control {
    fn all() -> &'static [Self] {
        &[
            Control::Left,
            Control::Right,
            Control::Attack,
            Control::Jump,
            Control::Dash,
            Control::Block,
            Control::Drink,
            Control::Use,
            Control::Inventory,
            Control::Status,
            Control::Nearest,
        ]
    }
    fn id(self) -> &'static str {
        match self {
            Control::Left => "left",
            Control::Right => "right",
            Control::Attack => "attack",
            Control::Jump => "jump",
            Control::Dash => "dash",
            Control::Block => "block",
            Control::Drink => "drink",
            Control::Use => "use",
            Control::Inventory => "inventory",
            Control::Status => "status",
            Control::Nearest => "nearest",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Control::Left => "Walk left",
            Control::Right => "Walk right",
            Control::Attack => "Attack",
            Control::Jump => "Jump",
            Control::Dash => "Dash",
            Control::Block => "Block",
            Control::Drink => "Drink a potion",
            Control::Use => "Use or pick up",
            Control::Inventory => "Inventory",
            Control::Status => "Status",
            Control::Nearest => "Nearest enemy",
        }
    }
    fn default_keys(self) -> &'static [Chord] {
        match self {
            Control::Left => const { &[Chord::plain(Key::Left)] },
            Control::Right => const { &[Chord::plain(Key::Right)] },
            Control::Attack => const { &[Chord::plain(Key::Space)] },
            Control::Jump => const { &[Chord::plain(Key::Up)] },
            Control::Dash => const { &[Chord::plain(Key::Z)] },
            Control::Block => const { &[Chord::plain(Key::X)] },
            Control::Drink => const { &[Chord::plain(Key::Q)] },
            Control::Use => const { &[Chord::plain(Key::Down)] },
            Control::Inventory => const { &[Chord::plain(Key::I)] },
            Control::Status => const { &[Chord::plain(Key::S)] },
            Control::Nearest => const { &[Chord::plain(Key::D)] },
        }
    }
    fn default_pad(self) -> &'static [PadChord] {
        match self {
            Control::Left => const { &[PadChord::plain(ControllerButton::DPadLeft)] },
            Control::Right => const { &[PadChord::plain(ControllerButton::DPadRight)] },
            Control::Attack => const { &[PadChord::plain(ControllerButton::X)] },
            Control::Jump => const { &[PadChord::plain(ControllerButton::A)] },
            Control::Dash => const { &[PadChord::plain(ControllerButton::B)] },
            Control::Block => const { &[PadChord::plain(ControllerButton::LeftShoulder)] },
            Control::Drink => const { &[PadChord::plain(ControllerButton::DPadUp)] },
            Control::Use => const { &[PadChord::plain(ControllerButton::DPadDown)] },
            Control::Inventory => const { &[PadChord::plain(ControllerButton::Y)] },
            Control::Status => const { &[PadChord::plain(ControllerButton::LeftStick)] },
            Control::Nearest => const { &[PadChord::plain(ControllerButton::RightStick)] },
        }
    }
}
