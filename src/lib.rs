//! Waning: an audio sidescroller. You descend from heaven at full strength,
//! every gate you pass takes some of it, and when you fall the souls you
//! gathered buy blessings for the next descent.

pub mod autopilot;
mod heaven;
mod play;
mod progress;
mod relics;
mod rng;
pub mod story;
mod title;
pub mod world;

use hearken::{Action, Chord, ControllerButton, Key, PadChord};

pub use heaven::Heaven;
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
    type Settings = ();
    type Action = Control;
    type Category = hearken::NoCategory;
}

/// The controls the player can move to other keys and buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Control {
    Left,
    Right,
    Attack,
    Jump,
    Use,
    Status,
    Nearest,
    Relics,
}

impl Action for Control {
    fn all() -> &'static [Self] {
        &[
            Control::Left,
            Control::Right,
            Control::Attack,
            Control::Jump,
            Control::Use,
            Control::Status,
            Control::Nearest,
            Control::Relics,
        ]
    }
    fn id(self) -> &'static str {
        match self {
            Control::Left => "left",
            Control::Right => "right",
            Control::Attack => "attack",
            Control::Jump => "jump",
            Control::Use => "use",
            Control::Status => "status",
            Control::Nearest => "nearest",
            Control::Relics => "relics",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Control::Left => "Walk left",
            Control::Right => "Walk right",
            Control::Attack => "Attack",
            Control::Jump => "Jump",
            Control::Use => "Use",
            Control::Status => "Status",
            Control::Nearest => "Nearest enemy",
            Control::Relics => "Relics",
        }
    }
    fn default_keys(self) -> &'static [Chord] {
        match self {
            Control::Left => const { &[Chord::plain(Key::Left)] },
            Control::Right => const { &[Chord::plain(Key::Right)] },
            Control::Attack => const { &[Chord::plain(Key::Space)] },
            Control::Jump => const { &[Chord::plain(Key::Up)] },
            Control::Use => const { &[Chord::plain(Key::Down)] },
            Control::Status => const { &[Chord::plain(Key::S)] },
            Control::Nearest => const { &[Chord::plain(Key::D)] },
            Control::Relics => const { &[Chord::plain(Key::R)] },
        }
    }
    fn default_pad(self) -> &'static [PadChord] {
        match self {
            Control::Left => const { &[PadChord::plain(ControllerButton::DPadLeft)] },
            Control::Right => const { &[PadChord::plain(ControllerButton::DPadRight)] },
            Control::Attack => const { &[PadChord::plain(ControllerButton::X)] },
            Control::Jump => const { &[PadChord::plain(ControllerButton::A)] },
            Control::Use => const { &[PadChord::plain(ControllerButton::DPadDown)] },
            Control::Status => const { &[PadChord::plain(ControllerButton::Y)] },
            Control::Nearest => const { &[PadChord::plain(ControllerButton::B)] },
            Control::Relics => const { &[PadChord::plain(ControllerButton::LeftShoulder)] },
        }
    }
}
