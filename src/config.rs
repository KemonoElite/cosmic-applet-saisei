// SPDX-License-Identifier: GPL-3.0-only

//! What the applet shows on the panel, kept in cosmic-config as the other
//! applets keep theirs, and watched with `Core::watch_config`, so the applet
//! on each screen follows a change made on another.

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};

/// A setting with an on/off switch in the popup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    Cover,
    Title,
    Bars,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CosmicConfigEntry)]
#[version = 1]
#[serde(default)]
pub struct SaiseiConfig {
    /// On the panel: the cover, the title, and the bars.
    pub show_cover: bool,
    pub show_title: bool,
    pub show_bars: bool,
}

impl Default for SaiseiConfig {
    fn default() -> Self {
        Self {
            show_cover: true,
            show_title: true,
            show_bars: true,
        }
    }
}

impl SaiseiConfig {
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
            Toggle::Cover => self.show_cover,
            Toggle::Title => self.show_title,
            Toggle::Bars => self.show_bars,
        }
    }

    /// Changes a setting and saves it; without a handler, for this session.
    pub fn set(&mut self, handler: Option<&cosmic_config::Config>, toggle: Toggle, value: bool) {
        let Some(handler) = handler else {
            match toggle {
                Toggle::Cover => self.show_cover = value,
                Toggle::Title => self.show_title = value,
                Toggle::Bars => self.show_bars = value,
            }
            return;
        };
        let saved = match toggle {
            Toggle::Cover => self.set_show_cover(handler, value),
            Toggle::Title => self.set_show_title(handler, value),
            Toggle::Bars => self.set_show_bars(handler, value),
        };
        if let Err(err) = saved {
            tracing::warn!("cannot save {toggle:?}: {err}");
        }
    }
}
