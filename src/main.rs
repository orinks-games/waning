#![cfg_attr(windows, windows_subsystem = "windows")]
//! `waning`: the game's entry point.

fn main() {
    std::process::exit(hearken::run::<waning::Waning>(
        waning::Progress::default(),
        || Box::new(waning::Title::new()),
    ));
}
