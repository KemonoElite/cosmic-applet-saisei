// SPDX-License-Identifier: GPL-3.0-only

//! Now Playing: what's playing, in the panel, opening into a Now Playing
//! card like the one in the macOS menu bar, drawn with COSMIC's theme as its
//! own applets are.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use cosmic::{
    Element, Task, app,
    applet::{menu_button, padded_control},
    cosmic_config,
    iced::{
        self, Alignment, Length, Rectangle, Subscription,
        core::text::{Ellipsize, EllipsizeHeightLimit},
        window,
    },
    surface, theme,
    widget::{
        self, button, divider,
        rectangle_tracker::{RectangleTracker, RectangleUpdate, rectangle_tracker_subscription},
        space, text,
    },
};

use crate::{
    config::{NowPlayingConfig, Toggle},
    fl,
    media::{self, Control, Media},
    widgets::{Bars, Marquee, accent, alpha, icon},
};

/// A paused player stays on the applet this long, then leaves it.
const LINGER: Duration = Duration::from_secs(5 * 60);
/// How wide the title gets on the panel, at most.
const TITLE_WIDTH: f32 = 240.0;
/// The cover in the popup, as big as the card is high.
const COVER: f32 = 112.0;

const GO_BACK: &str = "media-skip-backward-symbolic";
const GO_NEXT: &str = "media-skip-forward-symbolic";
const PAUSE: &str = "media-playback-pause-symbolic";
const PLAY: &str = "media-playback-start-symbolic";

pub struct NowPlaying {
    core: app::Core,
    popup: Option<window::Id>,
    media: Vec<Media>,
    /// The player picked in the popup, if not the one playing.
    selected: Option<String>,
    /// When each player was seen paused.
    paused_since: HashMap<String, Instant>,
    /// Where the progress is being dragged to, in microseconds.
    seeking: Option<f32>,
    config: NowPlayingConfig,
    config_handler: Option<cosmic_config::Config>,
    /// The popup's settings page, instead of the card.
    customizing: bool,
    /// Where the applet is in the panel, for the popup to open under it
    /// however wide the applet has grown.
    rectangle_tracker: Option<RectangleTracker<u32>>,
    rectangle: Option<Rectangle>,
}

#[derive(Clone, Debug)]
pub enum Message {
    TogglePopup,
    CloseRequested(window::Id),
    Rectangle(RectangleUpdate<u32>),
    Media(Vec<Media>),
    Control(Control),
    SelectPlayer(String),
    Seek(f32),
    SeekDone,
    Customize(bool),
    Set(Toggle, bool),
    ConfigChanged(NowPlayingConfig),
    /// Time passing: the progress moves, a paused player leaves.
    Tick,
    Ignore,
}

fn run<F: Future<Output = zbus::Result<()>> + Send + 'static>(
    what: &'static str,
    future: F,
) -> app::Task<Message> {
    Task::future(async move {
        if let Err(err) = future.await {
            tracing::warn!("cannot {what}: {err}");
        }
        cosmic::action::app(Message::Ignore)
    })
}

fn one_line() -> Ellipsize {
    Ellipsize::End(EllipsizeHeightLimit::Lines(1))
}

/// A place in the song, as the macOS player shows it: 01:07, 1:02:03.
fn clock(micros: f32) -> String {
    let seconds = (micros.max(0.0) / 1_000_000.0) as u64;
    let (h, m, s) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Secondary text, as the artist under the title: the theme's text, dimmed.
fn dimmed() -> theme::Text {
    theme::Text::Color(alpha(theme::active().cosmic().on_bg_color().into(), 0.65))
}

fn applet_icon() -> widget::icon::Handle {
    widget::icon::from_svg_bytes(
        &include_bytes!(
            "../data/icons/scalable/apps/dev.cosmic_ext.CosmicAppletNowPlaying-symbolic.svg"
        )[..],
    )
    .symbolic(true)
}

/// The divider between a popup's sections, as cosmic-applets draws it.
fn divided<'a>() -> Element<'a, Message> {
    let space = theme::active().cosmic().spacing;
    padded_control(divider::horizontal::default())
        .padding([space.space_xxs, space.space_s])
        .into()
}

/// A toggle switch in the card's corner, opening Customize: drawn in the
/// theme's text color, as the play buttons are.
fn settings_button<'a>() -> Element<'a, Message> {
    let space = theme::active().cosmic().spacing;
    let handle =
        widget::icon::from_svg_bytes(&include_bytes!("../data/customize-symbolic.svg")[..])
            .symbolic(true);
    button::icon(handle)
        .icon_size(16)
        .padding(space.space_xxxs)
        .tooltip(fl!("customize"))
        .on_press(Message::Customize(true))
        .into()
}

/// A cover, rounded as the theme rounds things, or a note where there's none.
/// Small ones, in the panel, less round.
fn cover<'a>(media: &Media, side: f32) -> Element<'a, Message> {
    let radii = theme::active().cosmic().corner_radii;
    let radius = if side < 32.0 {
        radii.radius_xs
    } else {
        radii.radius_s
    };
    match &media.art {
        Some(path) => widget::image(widget::image::Handle::from_path(path))
            .width(Length::Fixed(side))
            .height(Length::Fixed(side))
            .content_fit(iced::ContentFit::Cover)
            .border_radius(radius)
            .into(),
        None => widget::container(icon("audio-x-generic-symbolic", (side * 0.4) as u16))
            .center(Length::Fixed(side))
            .class(theme::Container::Card)
            .into(),
    }
}

impl NowPlaying {
    /// The player shown: the one picked, or the one playing.
    fn shown_media(&self) -> Option<&Media> {
        self.selected
            .as_ref()
            .and_then(|service| self.media.iter().find(|m| &m.service == service))
            .or_else(|| self.media.first())
    }

    /// Playing, or paused not long ago.
    fn is_live(&self, media: &Media) -> bool {
        media.playing
            || self
                .paused_since
                .get(&media.service)
                .is_some_and(|since| since.elapsed() < LINGER)
    }

    /// The player on the panel: playing, or paused not long ago.
    fn live_media(&self) -> Option<&Media> {
        self.shown_media().filter(|m| self.is_live(m))
    }

    /// What the applet shows in the panel, beside each other.
    fn panel_parts(&self) -> Vec<Element<'_, Message>> {
        let applet = &self.core.applet;
        let size = applet.suggested_size(true).0;
        let side = f32::from(size);
        let horizontal = applet.is_horizontal();
        let mut parts: Vec<Element<'_, Message>> = Vec::with_capacity(3);
        let Some(media) = self.live_media() else {
            return parts;
        };
        let config = &self.config;
        if config.show_cover {
            parts.push(cover(media, side));
        }
        if horizontal && config.show_title {
            // As wide as the title, up to a point.
            let title = applet.text(media.title.clone()).ellipsize(one_line());
            parts.push(widget::container(title).max_width(TITLE_WIDTH).into());
        }
        if config.show_bars {
            parts.push(Bars::new(accent(), media.playing, side).into());
        }
        // All turned off: a note, not nothing.
        if parts.is_empty() {
            parts.push(icon("audio-x-generic-symbolic", size));
        }
        parts
    }

    fn close_popup(&mut self) -> app::Task<Message> {
        self.seeking = None;
        match self.popup.take() {
            Some(id) => surface::surface_task(surface::action::destroy_popup(id)),
            None => Task::none(),
        }
    }

    fn control(&self, control: Control) -> app::Task<Message> {
        let Some(media) = self.shown_media() else {
            return Task::none();
        };
        run(
            "control the player",
            media::control(media.service.clone(), control),
        )
    }
}

impl cosmic::Application for NowPlaying {
    type Message = Message;
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    const APP_ID: &'static str = crate::APP_ID;

    fn init(core: app::Core, _flags: ()) -> (Self, app::Task<Message>) {
        let config_handler = NowPlayingConfig::handler();
        let config = NowPlayingConfig::load(config_handler.as_ref());
        // COSMIC_NOW_PLAYING_PREVIEW=popup opens the popup by itself, =settings on
        // its settings, to take screenshots without clicking the panel.
        let preview = std::env::var("COSMIC_NOW_PLAYING_PREVIEW").unwrap_or_default();
        let mut tasks = Vec::new();
        if preview.contains("popup") || preview.contains("settings") {
            tasks.push(Task::future(async {
                tokio::time::sleep(Duration::from_millis(1500)).await;
                cosmic::action::app(Message::TogglePopup)
            }));
            if preview.contains("settings") {
                tasks.push(Task::future(async {
                    tokio::time::sleep(Duration::from_millis(1700)).await;
                    cosmic::action::app(Message::Customize(true))
                }));
            }
        }
        (
            Self {
                core,
                popup: None,
                media: Vec::new(),
                selected: None,
                paused_since: HashMap::new(),
                seeking: None,
                config,
                config_handler,
                customizing: false,
                rectangle_tracker: None,
                rectangle: None,
            },
            Task::batch(tasks),
        )
    }

    fn core(&self) -> &app::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut app::Core {
        &mut self.core
    }

    fn style(&self) -> Option<iced::theme::Style> {
        Some(cosmic::applet::style())
    }

    fn update(&mut self, message: Message) -> app::Task<Message> {
        match message {
            Message::TogglePopup => {
                if self.popup.is_some() {
                    return self.close_popup();
                }
                self.customizing = false;
                return surface::surface_task(surface::action::app_popup(
                    |_| Default::default(),
                    |app: &mut Self| {
                        let new_id = window::Id::unique();
                        app.popup.replace(new_id);
                        let mut popup_settings = app.core.applet.get_popup_settings(
                            app.core.main_window_id().unwrap(),
                            new_id,
                            None,
                            None,
                            None,
                        );
                        // Under the applet, however wide it is.
                        if let Some(r) = app.rectangle {
                            popup_settings.positioner.anchor_rect = iced::Rectangle {
                                x: r.x as i32,
                                y: r.y as i32,
                                width: r.width as i32,
                                height: r.height as i32,
                            };
                        }
                        popup_settings
                    },
                    None,
                ));
            }
            Message::CloseRequested(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                    self.seeking = None;
                }
            }
            Message::Rectangle(update) => match update {
                RectangleUpdate::Rectangle((_, rectangle)) => self.rectangle = Some(rectangle),
                RectangleUpdate::Init(tracker) => self.rectangle_tracker = Some(tracker),
            },
            Message::Media(media) => {
                let now = Instant::now();
                for m in &media {
                    if m.playing {
                        self.paused_since.remove(&m.service);
                    } else {
                        self.paused_since.entry(m.service.clone()).or_insert(now);
                    }
                }
                self.paused_since
                    .retain(|service, _| media.iter().any(|m| &m.service == service));
                // A player that starts playing takes the applet.
                if let Some(selected) = &self.selected {
                    let started = media.iter().any(|m| {
                        m.playing
                            && &m.service != selected
                            && !self
                                .media
                                .iter()
                                .any(|old| old.playing && old.service == m.service)
                    });
                    if started {
                        self.selected = None;
                    }
                }
                self.media = media;
            }
            Message::Control(control) => {
                // Shown at once, before the player says so.
                if let Control::PlayPause = control
                    && let Some(service) = self.shown_media().map(|m| m.service.clone())
                    && let Some(media) = self.media.iter_mut().find(|m| m.service == service)
                {
                    media.position = media.position_now();
                    media.at = Instant::now();
                    media.playing = !media.playing;
                    if media.playing {
                        self.paused_since.remove(&service);
                    } else {
                        self.paused_since.insert(service, Instant::now());
                    }
                }
                return self.control(control);
            }
            Message::SelectPlayer(service) => self.selected = Some(service),
            Message::Seek(position) => self.seeking = Some(position),
            Message::SeekDone => {
                let Some(position) = self.seeking.take() else {
                    return Task::none();
                };
                let Some(service) = self.shown_media().map(|m| m.service.clone()) else {
                    return Task::none();
                };
                let Some(media) = self.media.iter_mut().find(|m| m.service == service) else {
                    return Task::none();
                };
                media.position = position as i64;
                media.at = Instant::now();
                if let Some(track) = media.track.clone() {
                    return run("seek", media::seek(service, track, position as i64));
                }
            }
            Message::Customize(customizing) => self.customizing = customizing,
            Message::Set(toggle, value) => {
                self.config.set(self.config_handler.as_ref(), toggle, value);
            }
            Message::ConfigChanged(config) => self.config = config,
            // Only to draw again.
            Message::Tick => {}
            Message::Ignore => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            self.core
                .watch_config::<NowPlayingConfig>(crate::APP_ID)
                .map(|update| {
                    for err in update.errors.iter().filter(|err| err.is_err()) {
                        tracing::error!(?err, "Error watching config");
                    }
                    Message::ConfigChanged(update.config)
                }),
            rectangle_tracker_subscription(0).map(|update| Message::Rectangle(update.1)),
            media::subscription().map(Message::Media),
        ];
        let lingering = self
            .paused_since
            .values()
            .any(|since| since.elapsed() < LINGER);
        let progressing = self.popup.is_some() && self.shown_media().is_some_and(|m| m.playing);
        let every = if progressing {
            Some(Duration::from_millis(500))
        } else if lingering {
            // Only to let the player go, in its time.
            Some(Duration::from_secs(5))
        } else {
            None
        };
        if let Some(every) = every {
            subscriptions.push(iced::time::every(every).map(|_| Message::Tick));
        }
        Subscription::batch(subscriptions)
    }

    fn view(&self) -> Element<'_, Message> {
        let applet = &self.core.applet;
        let parts = self.panel_parts();
        let button = if parts.is_empty() {
            // Nothing going on: its icon, as the other applets'.
            applet.icon_button_from_handle(applet_icon())
        } else {
            let (major, minor) = applet.suggested_padding(true);
            let spacing = theme::active().cosmic().spacing.space_xs;
            let size = applet.suggested_size(true).0;
            // As COSMIC's text buttons: as high as the panel's icons with
            // their room, never higher, for the panel not to grow.
            if applet.is_horizontal() {
                let row = widget::row::with_children(parts)
                    .spacing(spacing)
                    .align_y(Alignment::Center);
                button::custom(
                    widget::container(row).center_y(Length::Fixed(f32::from(size + 2 * minor))),
                )
                .padding([0, major])
            } else {
                let column = widget::column::with_children(parts)
                    .spacing(spacing)
                    .align_x(Alignment::Center);
                button::custom(
                    widget::container(column).center_x(Length::Fixed(f32::from(size + 2 * minor))),
                )
                .padding([major, 0])
            }
            .class(theme::Button::AppletIcon)
        }
        .on_press_down(Message::TogglePopup);

        // The middle button plays or pauses, as on other players' icons.
        let area = widget::mouse_area(button).on_middle_press(Message::Control(Control::PlayPause));
        let applet_button: Element<'_, Message> = match self.rectangle_tracker.as_ref() {
            Some(tracker) => tracker.container(0, area).into(),
            None => area.into(),
        };
        // In the panel's overflow popup, it is laid out there.
        if applet
            .suggested_bounds
            .as_ref()
            .is_some_and(|c| c.width > 0. && c.height > 0.)
        {
            applet_button
        } else {
            applet.autosize_window(applet_button).into()
        }
    }

    fn view_window(&self, id: window::Id) -> Element<'_, Message> {
        if self.popup != Some(id) {
            return widget::text("").into();
        }
        let content = if self.customizing {
            self.settings_page()
        } else {
            self.now_playing_page()
        };
        self.core.applet.popup_container(content).into()
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::CloseRequested(id))
    }
}

impl NowPlaying {
    /// The card, and under it, with more than one player open, the others to
    /// pick from. Customize is the toggle in the card's corner.
    fn now_playing_page(&self) -> Element<'_, Message> {
        let space = theme::active().cosmic().spacing;
        let content = widget::column::with_capacity(3);
        match self.shown_media() {
            // The players listed under the card get room under them.
            Some(media) if self.media.len() > 1 => content
                .extend(self.now_playing(media))
                .padding([0, 0, space.space_xs, 0])
                .into(),
            Some(media) => content.extend(self.now_playing(media)).into(),
            None => content.push(empty_section()).into(),
        }
    }

    /// The card from the macOS menu bar: the cover on the left; on the
    /// right the title with the bars and the Customize toggle beside it, the artist, the
    /// controls, the progress, and under it the time gone and the time left.
    fn now_playing<'a>(&'a self, media: &'a Media) -> Vec<Element<'a, Message>> {
        let space = theme::active().cosmic().spacing;

        let artist = if media.artist.is_empty() {
            fl!("unknown-artist")
        } else {
            media.artist.clone()
        };
        let heading = widget::row::with_children(vec![
            widget::column::with_children(vec![
                // Too long for their room, title and artist scroll instead of
                // being cut short.
                Marquee::new(
                    text::heading(media.title.clone())
                        .size(16.0)
                        .wrapping(iced::widget::text::Wrapping::None),
                )
                .into(),
                Marquee::new(
                    text::body(artist)
                        .class(dimmed())
                        .wrapping(iced::widget::text::Wrapping::None),
                )
                .into(),
            ])
            .spacing(space.space_xxxs)
            .width(Length::Fill)
            .into(),
            widget::row::with_children(vec![
                Bars::new(alpha(accent(), 0.9), media.playing, 16.0).into(),
                settings_button(),
            ])
            .spacing(space.space_xxs)
            .align_y(Alignment::Center)
            .into(),
        ])
        .spacing(space.space_xs)
        .align_y(Alignment::Start);

        // Flat, as in the menu bar: no circle, the middle one bigger.
        let skip = |name: &'static str, message: Option<Message>| -> Element<'a, Message> {
            button::icon(widget::icon::from_name(name).symbolic(true))
                .icon_size(24)
                .padding(space.space_xxs)
                .on_press_maybe(message)
                .into()
        };
        let play = button::icon(
            widget::icon::from_name(if media.playing { PAUSE } else { PLAY }).symbolic(true),
        )
        .icon_size(32)
        .padding(space.space_xxs)
        .on_press(Message::Control(Control::PlayPause));
        let controls = widget::row::with_children(vec![
            space::horizontal().width(Length::Fill).into(),
            skip(
                GO_BACK,
                media
                    .can_previous
                    .then_some(Message::Control(Control::Previous)),
            ),
            space::horizontal().width(Length::Fill).into(),
            play.into(),
            space::horizontal().width(Length::Fill).into(),
            skip(
                GO_NEXT,
                media.can_next.then_some(Message::Control(Control::Next)),
            ),
            space::horizontal().width(Length::Fill).into(),
        ])
        .align_y(Alignment::Center);

        let mut details = vec![heading.into(), controls.into()];
        // Streams have no length: no progress to show.
        if media.length > 0 {
            let length = media.length as f32;
            let position = self
                .seeking
                .unwrap_or(media.position_now() as f32)
                .min(length);
            details.push(
                widget::column::with_children(vec![
                    widget::slider(0.0..=length, position, Message::Seek)
                        .on_release(Message::SeekDone)
                        .width(Length::Fill)
                        .into(),
                    widget::row::with_children(vec![
                        text::caption(clock(position)).class(dimmed()).into(),
                        space::horizontal().width(Length::Fill).into(),
                        text::caption(format!("\u{2212}{}", clock(length - position)))
                            .class(dimmed())
                            .into(),
                    ])
                    .into(),
                ])
                .spacing(space.space_xxxs)
                .into(),
            );
        }

        // Clicking the cover brings the player up.
        let card = widget::row::with_children(vec![
            widget::mouse_area(cover(media, COVER))
                .on_press(Message::Control(Control::Raise))
                .interaction(iced::mouse::Interaction::Pointer)
                .into(),
            widget::column::with_children(details)
                .spacing(space.space_xs)
                .width(Length::Fill)
                .into(),
        ])
        .spacing(space.space_s)
        .align_y(Alignment::Center);

        let mut rows: Vec<Element<'a, Message>> = vec![
            widget::container(card)
                .padding([space.space_s, space.space_m])
                .width(Length::Fill)
                .into(),
        ];

        // More than one player: each by name, to pick from.
        if self.media.len() > 1 {
            let players = self
                .media
                .iter()
                .map(|m| {
                    let pick = Message::SelectPlayer(m.service.clone());
                    let name = if m.playing {
                        m.player.clone()
                    } else {
                        format!("{} · {}", m.player, fl!("paused"))
                    };
                    if m.service == media.service {
                        button::suggested(name).on_press(pick).into()
                    } else {
                        button::standard(name).on_press(pick).into()
                    }
                })
                .collect::<Vec<_>>();
            rows.push(divided());
            rows.push(
                padded_control(
                    widget::row::with_children(players)
                        .spacing(space.space_xxs)
                        .wrap(),
                )
                .into(),
            );
        }
        rows
    }

    /// What the applet shows on the panel, to turn on and off.
    fn settings_page(&self) -> Element<'_, Message> {
        let space = theme::active().cosmic().spacing;
        let config = &self.config;
        let toggle = |label: String, toggle: Toggle| -> Element<'_, Message> {
            padded_control(
                widget::toggler(config.get(toggle))
                    .label(label)
                    .on_toggle(move |on| Message::Set(toggle, on))
                    .width(Length::Fill),
            )
            .into()
        };
        let heading = |label: String| -> Element<'_, Message> {
            padded_control(text::caption_heading(label)).into()
        };
        let back = menu_button(
            widget::row::with_children(vec![
                icon("go-previous-symbolic", 16),
                text::body(fl!("customize")).into(),
            ])
            .spacing(space.space_xs)
            .align_y(Alignment::Center),
        )
        .on_press(Message::Customize(false));
        widget::column::with_children(vec![
            back.into(),
            divided(),
            heading(fl!("on-the-panel")),
            toggle(fl!("show-cover"), Toggle::Cover),
            toggle(fl!("show-title"), Toggle::Title),
            toggle(fl!("show-bars"), Toggle::Bars),
        ])
        .padding([8, 0])
        .into()
    }
}

/// Nothing playing: a note, with the Customize toggle in the same corner as on the card.
fn empty_section<'a>() -> Element<'a, Message> {
    let space = theme::active().cosmic().spacing;
    widget::container(
        widget::row::with_children(vec![
            icon("audio-x-generic-symbolic", 24),
            widget::column::with_children(vec![
                text::heading(fl!("nothing-playing")).into(),
                text::caption(fl!("nothing-playing-about")).into(),
            ])
            .width(Length::Fill)
            .into(),
            settings_button(),
        ])
        .spacing(space.space_s)
        .align_y(Alignment::Center),
    )
    .padding([space.space_s, space.space_m])
    .width(Length::Fill)
    .into()
}
