//! The story: what the Keeper says in heaven, and what each boss fight
//! opens and closes with.

use crate::progress::Progress;
use crate::world::BossKind;

/// Said the first time you reach heaven.
pub const OPENING: &str =
    "Heaven's Lamp is going dark. At the bottom of the world, the Hollow King \
is drinking its light. The Keeper has sent Lights down to stop him before, and none came back. \
You are the last. The Keeper pours everything the Lamp has left into you. It is more strength \
than you will ever have again: every gate below takes some of it.";

/// Said as you enter the arena.
pub fn boss_intro(kind: BossKind) -> &'static str {
    match kind {
        BossKind::Shepherd => {
            "A giant waits on the hill with a crook as long as a tree. The Shepherd tended \
             heaven's gardens before he went down. He does not know you."
        }
        BossKind::Mother => {
            "Weeping fills the glade. The Weeping Mother was sent down long ago. She lost her \
             strength one gate at a time, and she grieves it still."
        }
        BossKind::Ferryman => {
            "A boat waits on the black water. The Ferryman stands in it with his oar raised. \
             He carried every Light before you across. He will not carry you."
        }
        BossKind::King => {
            "The Silent City ends at a throne. The Hollow King sits on it, and the light of \
             every Light before you flickers inside him. He was the first one the Keeper sent."
        }
    }
}

/// Said when the boss falls below half health.
pub fn boss_enraged(kind: BossKind) -> &'static str {
    match kind {
        BossKind::Shepherd => "The Shepherd roars and swings faster.",
        BossKind::Mother => "The Weeping Mother screams. Her wails come two at a time.",
        BossKind::Ferryman => "The Ferryman beats the water with his oar. The waves come faster.",
        BossKind::King => {
            "The Hollow King rises from his throne. The stolen light in him burns hotter."
        }
    }
}

/// Said when the boss dies.
pub fn boss_defeat(kind: BossKind) -> &'static str {
    match kind {
        BossKind::Shepherd => {
            "The Shepherd falls to his knees. A little of the old light leaves him and rises \
             past you toward heaven."
        }
        BossKind::Mother => {
            "The weeping stops. For a moment the glade is quiet enough to hear water far below."
        }
        BossKind::Ferryman => {
            "The oar sinks. The boat drifts to the far shore, and the stair goes down from there."
        }
        BossKind::King => {
            "The Hollow King breaks apart. The light inside him rises: the Shepherd's, the \
             Mother's, the Ferryman's, every Light he drank. You have almost nothing left. It \
             is enough. Far above, the Lamp burns again."
        }
    }
}

/// The milestones the Keeper speaks to, in story order. Each is spoken
/// once, the first time you return to heaven with it reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Beat {
    FirstFall,
    MetShepherd,
    BeatShepherd,
    MetMother,
    BeatMother,
    MetFerryman,
    BeatFerryman,
    MetKing,
    Won,
}

impl Beat {
    pub const ALL: [Beat; 9] = [
        Beat::FirstFall,
        Beat::MetShepherd,
        Beat::BeatShepherd,
        Beat::MetMother,
        Beat::BeatMother,
        Beat::MetFerryman,
        Beat::BeatFerryman,
        Beat::MetKing,
        Beat::Won,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Beat::FirstFall => "first_fall",
            Beat::MetShepherd => "met_shepherd",
            Beat::BeatShepherd => "beat_shepherd",
            Beat::MetMother => "met_mother",
            Beat::BeatMother => "beat_mother",
            Beat::MetFerryman => "met_ferryman",
            Beat::BeatFerryman => "beat_ferryman",
            Beat::MetKing => "met_king",
            Beat::Won => "won",
        }
    }

    pub fn reached(self, p: &Progress) -> bool {
        let met = |k: BossKind| p.met.iter().any(|m| m == k.key());
        let beat = |k: BossKind| p.beaten.iter().any(|m| m == k.key());
        match self {
            Beat::FirstFall => p.runs > 0,
            Beat::MetShepherd => met(BossKind::Shepherd),
            Beat::BeatShepherd => beat(BossKind::Shepherd),
            Beat::MetMother => met(BossKind::Mother),
            Beat::BeatMother => beat(BossKind::Mother),
            Beat::MetFerryman => met(BossKind::Ferryman),
            Beat::BeatFerryman => beat(BossKind::Ferryman),
            Beat::MetKing => met(BossKind::King),
            Beat::Won => p.wins > 0,
        }
    }

    /// The Keeper's words.
    pub fn line(self) -> &'static str {
        match self {
            Beat::FirstFall => {
                "The Keeper: You came back. Everyone does, the first time. The souls you freed \
                 came up with you, and here they are grace. Spend it, and go down again."
            }
            Beat::MetShepherd => {
                "The Keeper: So you found the Shepherd. When he lifts his crook, get out of \
                 reach or jump. When he lowers his head, he is about to charge."
            }
            Beat::BeatShepherd => {
                "The Keeper: The Lamp flickered when the Shepherd fell. Some of his light came \
                 home. I had not thought any of them could still be saved."
            }
            Beat::MetMother => {
                "The Keeper: The Mother was my friend. She went down to find her daughter, \
                 the second Light, and never came back up. Listen for where she reappears."
            }
            Beat::BeatMother => {
                "The Keeper: She is resting now. If her daughter is anywhere, she is with the \
                 Hollow King."
            }
            Beat::MetFerryman => {
                "The Keeper: The Ferryman took every Light across the black water, me among \
                 them, once. Yes. I went down too, long ago, and came back with nothing. That \
                 is how I became the Keeper."
            }
            Beat::BeatFerryman => {
                "The Keeper: Past the water is the Cinder Stair, and the Silent City, and him. \
                 You will have very little left by then. Spend your grace well."
            }
            Beat::MetKing => {
                "The Keeper: You saw him. The first Light, my teacher. He went down at full \
                 strength and refused to grow weak, so he took the strength of everyone who \
                 came after. Do not become him."
            }
            Beat::Won => {
                "The Keeper: The Lamp is full again. You did it with almost nothing left, \
                 which he never could. Go down whenever you like. The world below is still \
                 there, and so am I."
            }
        }
    }
}

/// The newest beat reached but not yet heard, if any.
pub fn unheard(p: &Progress) -> Option<Beat> {
    Beat::ALL
        .into_iter()
        .rev()
        .find(|b| b.reached(p) && !p.heard.iter().any(|h| h == b.id()))
}

/// The newest beat reached, to say again on request.
pub fn latest(p: &Progress) -> Option<Beat> {
    Beat::ALL.into_iter().rev().find(|b| b.reached(p))
}
