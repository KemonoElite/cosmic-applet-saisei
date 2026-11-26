// SPDX-License-Identifier: GPL-3.0-only

//! Now Playing for the COSMIC panel, laid out as the applets in
//! pop-os/cosmic-applets: a library with `run()`, Fluent translations, its
//! settings in cosmic-config.

mod app;
mod config;
mod localize;
mod media;
mod power;
mod recording;
mod widgets;

pub const APP_ID: &str = "dev.cosmic_ext.CosmicAppletNowPlaying";

pub fn run() -> cosmic::iced::Result {
    localize::localize();
    cosmic::applet::run::<app::NowPlaying>(())
}
