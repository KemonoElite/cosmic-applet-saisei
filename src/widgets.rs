// SPDX-License-Identifier: GPL-3.0-only

//! The bars that dance to the music, the title that scrolls when it's too
//! long, and icons in the theme's colors.

use std::{
    rc::Rc,
    sync::LazyLock,
    time::{Duration, Instant},
};

use cosmic::{
    Element,
    iced::{
        self, Border, Color, Event, Length, Point, Rectangle, Size, Vector,
        advanced::{
            Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer,
            widget::{Tree, tree},
        },
        window,
    },
    theme, widget,
};

static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

pub fn accent() -> Color {
    theme::active().cosmic().accent_color().into()
}

pub fn alpha(mut color: Color, a: f32) -> Color {
    color.a = a;
    color
}

/// An audio meter as phones show it: bars rising and falling, or resting
/// low while paused. Not the sound itself, which a panel can't hear, but its
/// look. The bars redraw themselves; the applet isn't asked to.
pub struct Bars {
    color: Color,
    playing: bool,
    count: usize,
    side: f32,
}

impl Bars {
    /// As wide as high: `side` across.
    pub fn new(color: Color, playing: bool, side: f32) -> Self {
        Self {
            color,
            playing,
            count: 4,
            side,
        }
    }

    /// A bar's height, from 0 to 1, at this time: two slow waves apiece so
    /// they never fall in step.
    fn level(&self, bar: usize, seconds: f32) -> f32 {
        if !self.playing {
            return 0.2;
        }
        let i = bar as f32;
        let first = (seconds * (5.1 + i * 1.7) + i * 1.3).sin();
        let second = (seconds * (8.3 - i * 0.9) + i * 2.9).sin();
        (0.55 + 0.28 * first + 0.17 * second).clamp(0.18, 1.0)
    }
}

impl<Message> Widget<Message, cosmic::Theme, cosmic::Renderer> for Bars {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.side), Length::Fixed(self.side))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &cosmic::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(
            Length::Fixed(self.side),
            Length::Fixed(self.side),
            Size::new(self.side, self.side),
        ))
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &cosmic::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if self.playing
            && let Event::Window(window::Event::RedrawRequested(now)) = event
        {
            // About 20 frames a second: smooth enough, and light on the panel.
            shell.request_redraw_at(*now + Duration::from_millis(50));
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut cosmic::Renderer,
        _theme: &cosmic::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let seconds = EPOCH.elapsed().as_secs_f32();
        let n = self.count as f32;
        // Bars as wide as the room between them, in the middle 80%.
        let height = bounds.height * 0.8;
        let bar = bounds.width / (2.0 * n - 1.0);
        let gap = (bounds.width - bar * n) / (n - 1.0).max(1.0);
        for i in 0..self.count {
            let h = (height * self.level(i, seconds)).max(bar);
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x + i as f32 * (bar + gap),
                        y: bounds.center_y() - h / 2.0,
                        width: bar,
                        height: h,
                    },
                    border: Border {
                        radius: (bar / 2.0).into(),
                        ..Default::default()
                    },
                    ..renderer::Quad::default()
                },
                if self.playing {
                    self.color
                } else {
                    alpha(self.color, 0.5)
                },
            );
        }
    }
}

impl<'a, Message: 'a> From<Bars> for Element<'a, Message> {
    fn from(bars: Bars) -> Self {
        Self::new(bars)
    }
}

/// A symbolic icon from the system, in the theme's text color.
pub fn icon<'a, Message: 'a>(name: &str, size: u16) -> Element<'a, Message> {
    widget::icon::from_name(name)
        .symbolic(true)
        .size(size)
        .icon()
        .into()
}

/// A symbolic icon in one of the theme's colors: its accent, warning…
pub fn tinted<'a, Message: 'a>(
    name: &str,
    size: u16,
    color: fn(&cosmic::Theme) -> Color,
) -> Element<'a, Message> {
    widget::icon::icon(widget::icon::from_name(name).symbolic(true).handle())
        .size(size)
        .class(theme::Svg::Custom(Rc::new(move |theme| {
            iced::widget::svg::Style {
                color: Some(color(theme)),
            }
        })))
        .into()
}

pub fn destructive_of(theme: &cosmic::Theme) -> Color {
    theme.cosmic().destructive_color().into()
}

pub fn success_of(theme: &cosmic::Theme) -> Color {
    theme.cosmic().success_color().into()
}

/// How long a long line holds still before it scrolls, and between scrolls.
const MARQUEE_PAUSE: f32 = 2.5;
/// How fast it scrolls, in pixels a second.
const MARQUEE_SPEED: f32 = 32.0;
/// The room between the end of the line and its start coming around again.
const MARQUEE_GAP: f32 = 40.0;

/// A line that scrolls when it is wider than its room, as the title in the
/// macOS Now Playing card does: it holds still for a moment, glides left
/// until its start comes around again, and holds still again. A line that
/// fits stays put.
pub struct Marquee<'a, Message> {
    content: Element<'a, Message>,
}

impl<'a, Message> Marquee<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

/// When the line started over, and how wide it was then: a new, longer
/// title starts from the beginning.
struct MarqueeState {
    start: Instant,
    width: f32,
}

/// How far the line has scrolled by now, `cycle` being its width with the gap.
fn marquee_offset(seconds: f32, cycle: f32) -> f32 {
    let period = MARQUEE_PAUSE + cycle / MARQUEE_SPEED;
    let t = seconds % period;
    if t < MARQUEE_PAUSE {
        0.0
    } else {
        ((t - MARQUEE_PAUSE) * MARQUEE_SPEED).min(cycle)
    }
}

impl<Message> Widget<Message, cosmic::Theme, cosmic::Renderer> for Marquee<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<MarqueeState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(MarqueeState {
            start: Instant::now(),
            width: 0.0,
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &cosmic::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max = limits.max();
        // The whole line, however long; shown through its room.
        let child = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, max.height)),
        );
        let line = child.size();
        let state = tree.state.downcast_mut::<MarqueeState>();
        if (state.width - line.width).abs() > 0.5 {
            *state = MarqueeState {
                start: Instant::now(),
                width: line.width,
            };
        }
        layout::Node::with_children(
            Size::new(line.width.min(max.width), line.height),
            vec![child],
        )
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &cosmic::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let overflows = layout
            .children()
            .next()
            .is_some_and(|line| line.bounds().width > layout.bounds().width + 0.5);
        if overflows && let Event::Window(window::Event::RedrawRequested(now)) = event {
            // About 30 frames a second while it scrolls; none while it fits.
            shell.request_redraw_at(*now + Duration::from_millis(33));
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut cosmic::Renderer,
        theme: &cosmic::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(line) = layout.children().next() else {
            return;
        };
        let bounds = layout.bounds();
        let content = self.content.as_widget();
        let child = &tree.children[0];
        let width = line.bounds().width;
        if width <= bounds.width + 0.5 {
            content.draw(child, renderer, theme, style, line, cursor, viewport);
            return;
        }
        let cycle = width + MARQUEE_GAP;
        let state = tree.state.downcast_ref::<MarqueeState>();
        let offset = marquee_offset(state.start.elapsed().as_secs_f32(), cycle);
        // The line, and its start coming around after it, seen through the room.
        let everywhere = Rectangle::new(
            Point::new(bounds.x - cycle, bounds.y),
            Size::new(cycle * 3.0, bounds.height),
        );
        renderer.with_layer(bounds, |renderer| {
            for shift in [-offset, cycle - offset] {
                renderer.with_translation(Vector::new(shift, 0.0), |renderer| {
                    content.draw(child, renderer, theme, style, line, cursor, &everywhere);
                });
            }
        });
    }
}

impl<'a, Message: 'a> From<Marquee<'a, Message>> for Element<'a, Message> {
    fn from(marquee: Marquee<'a, Message>) -> Self {
        Self::new(marquee)
    }
}
