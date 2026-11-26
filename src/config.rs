// SPDX-License-Identifier: GPL-3.0-only

//! What the applet shows, kept in cosmic-config as the other applets keep
//! theirs, and watched with `Core::watch_config`, so the applet on each
//! screen follows a change made on another.

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TitleWidth {
    Small,
    #[default]
    Medium,
    Large,
}

impl TitleWidth {
    pub const ALL: [TitleWidth; 3] = [TitleWidth::Small, TitleWidth::Medium, TitleWidth::Large];

    pub fn pixels(self) -> f32 {
        match self {
            TitleWidth::Small => 100.0,
            TitleWidth::Medium => 160.0,
            TitleWidth::Large => 240.0,
        }
    }
}

/// A setting with an on/off switch in the popup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    ShowCover,
    ShowTitle,
    ShowBars,
    Media,
    Recording,
    Charging,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CosmicConfigEntry)]
#[version = 1]
#[serde(default)]
pub struct NowPlayingConfig {
    /// On the panel: the cover, the title, and the bars.
    pub show_cover: bool,
    pub show_title: bool,
    pub title_width: TitleWidth,
    pub show_bars: bool,
    /// The activities the applet follows.
    pub media: bool,
    pub recording: bool,
    pub charging: bool,
}

impl Default for NowPlayingConfig {
    fn default() -> Self {
        Self {
            show_cover: true,
            show_title: true,
            title_width: TitleWidth::default(),
            show_bars: true,
            media: true,
            recording: true,
            charging: true,
        }
    }
}

impl NowPlayingConfig {
    pub fn handler() -> Option<cosmic_config::Config> {
        cosmic_config::Config::new(crate::APP_ID, Self::VERSION)
            .inspect_err(|err| tracing::warn!("cannot open the settings: {err}"))
            .ok()
    }

    /// As saved, each setting not saved yet as it is at first.
    pub fn load(handler: Option<&cosmic_config::Config>) -> Self {
        handler.map_or_else(Self::default, |handler| {
            Self::get_entry(handler).unwrap_or_else(|(errors, config)| {
                for err in errors.into_iter().filter(cosmic_config::Error::is_err) {
                    tracing::warn!("cannot read a setting: {err}");
                }
                config
            })
        })
    }

    pub fn get(&self, toggle: Toggle) -> bool {
        match toggle {
            Toggle::ShowCover => self.show_cover,
            Toggle::ShowTitle => self.show_title,
            Toggle::ShowBars => self.show_bars,
            Toggle::Media => self.media,
            Toggle::Recording => self.recording,
            Toggle::Charging => self.charging,
        }
    }

    /// Changes a setting and saves it; without a handler, for this session.
    pub fn set(&mut self, handler: Option<&cosmic_config::Config>, toggle: Toggle, value: bool) {
        let Some(handler) = handler else {
            match toggle {
                Toggle::ShowCover => self.show_cover = value,
                Toggle::ShowTitle => self.show_title = value,
                Toggle::ShowBars => self.show_bars = value,
                Toggle::Media => self.media = value,
                Toggle::Recording => self.recording = value,
                Toggle::Charging => self.charging = value,
            }
            return;
        };
        let saved = match toggle {
            Toggle::ShowCover => self.set_show_cover(handler, value),
            Toggle::ShowTitle => self.set_show_title(handler, value),
            Toggle::ShowBars => self.set_show_bars(handler, value),
            Toggle::Media => self.set_media(handler, value),
            Toggle::Recording => self.set_recording(handler, value),
            Toggle::Charging => self.set_charging(handler, value),
        };
        if let Err(err) = saved {
            tracing::warn!("cannot save {toggle:?}: {err}");
        }
    }

    pub fn set_width(&mut self, handler: Option<&cosmic_config::Config>, width: TitleWidth) {
        match handler {
            Some(handler) => {
                if let Err(err) = self.set_title_width(handler, width) {
                    tracing::warn!("cannot save the title's width: {err}");
                }
            }
            None => self.title_width = width,
        }
    }
}
