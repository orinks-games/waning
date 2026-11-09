//! One piece of music at a time, shared by every screen: a screen asks for
//! its track, and the music changes only when the track does, so moving
//! between menus never cuts it off.

use hearken::{Bus, Ctx, Sound};

use crate::Waning;

pub struct Jukebox {
    key: String,
    sound: Sound,
}

impl Default for Jukebox {
    fn default() -> Self {
        Jukebox {
            key: String::new(),
            sound: Sound::NONE,
        }
    }
}

impl std::fmt::Debug for Jukebox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.key)
    }
}

impl Clone for Jukebox {
    /// A copy starts silent: only one jukebox owns the playing track.
    fn clone(&self) -> Self {
        Jukebox::default()
    }
}

/// Plays `key`, fading out what played before, and returns the key it
/// replaced. Asking for the track already playing does nothing.
pub fn play(ctx: &mut Ctx<Waning>, key: &str) -> String {
    if ctx.game.jukebox.key == key {
        return key.to_string();
    }
    let old = std::mem::take(&mut ctx.game.jukebox);
    ctx.audio.stop(old.sound, 1500);
    let sound = if key.is_empty() {
        Sound::NONE
    } else {
        ctx.audio.start_loop(key, None, Bus::MUSIC)
    };
    ctx.game.jukebox = Jukebox {
        key: key.to_string(),
        sound,
    };
    old.key
}

/// Fades the music out over `fade_ms`.
pub fn stop(ctx: &mut Ctx<Waning>, fade_ms: u32) {
    let old = std::mem::take(&mut ctx.game.jukebox);
    ctx.audio.stop(old.sound, fade_ms);
}
