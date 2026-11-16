//! Text a person can select with the mouse and copy with Ctrl+C. It draws like iced's rich text (bold,
//! italic, colours, backgrounds, underline, strikethrough and links), and shows a selection.
//!
//! Each grapheme cluster is its own span, so a selection is a range of spans, and the spans' bounds
//! give the highlight. A message can have several blocks of text, so the selection lives in the app:
//! each block is told which of its spans are selected, and reports presses and pointer moves back.

use std::ops::Range;

use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse;
use iced::advanced::renderer::Renderer as _;
use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::{self, Paragraph as _, Renderer as _};
use iced::advanced::widget::text as widget_text;
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::widget::text::Span;
use iced::{border, Color, Element, Event, Length, Point, Rectangle, Size};
use unicode_segmentation::UnicodeSegmentation;

type Para = <iced::Renderer as text::Renderer>::Paragraph;

/// One stretch of styled text. It is split into one span per grapheme cluster when it is drawn.
#[derive(Debug, Clone)]
pub struct Unit {
    pub text: String,
    pub color: Color,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub underline: bool,
    pub strike: bool,
    pub background: Option<Color>,
    pub link: Option<String>,
}

/// What the pointer did on a block: pressed on a span, or moved over one while a selection is dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Press(usize),
    Over(usize),
    /// The mouse was pressed somewhere else while this block held part of the selection.
    Outside,
}

/// Splits every unit into one unit per grapheme cluster, so each can be selected on its own.
pub fn split(units: Vec<Unit>) -> Vec<Unit> {
    let mut out = Vec::new();
    for unit in units {
        for grapheme in unit.text.graphemes(true) {
            out.push(Unit {
                text: grapheme.to_string(),
                ..unit.clone()
            });
        }
    }
    out
}

/// The spans of a block that a selection from `from` to `to` covers. Each end is (block, span). `block`
/// is this block's number and `len` how many spans it has.
pub fn range_in_block(from: (usize, usize), to: (usize, usize), block: usize, len: usize) -> Option<Range<usize>> {
    let (start, end) = if from <= to { (from, to) } else { (to, from) };
    if block < start.0 || block > end.0 || len == 0 {
        return None;
    }
    let first = if block == start.0 { start.1.min(len - 1) } else { 0 };
    let last = if block == end.0 { end.1.min(len - 1) } else { len - 1 };
    (first <= last).then_some(first..last + 1)
}

/// A block of selectable text.
pub struct Selectable<'a, Message> {
    units: Vec<Unit>,
    size: f32,
    selection: Color,
    selected: Option<Range<usize>>,
    dragging: bool,
    on_step: Box<dyn Fn(Step) -> Message + 'a>,
    open: fn(String) -> Message,
}

/// A block of text whose units are already split (see [`split`]). `selected` are the spans the selection
/// covers, `dragging` says whether a selection is being dragged, `on_step` reports presses and moves,
/// and `open` turns a clicked link into a message.
#[allow(clippy::too_many_arguments)]
pub fn selectable<'a, Message: 'a>(
    units: Vec<Unit>,
    size: f32,
    selection: Color,
    selected: Option<Range<usize>>,
    dragging: bool,
    on_step: impl Fn(Step) -> Message + 'a,
    open: fn(String) -> Message,
) -> Element<'a, Message> {
    Element::new(Selectable {
        units,
        size,
        selection,
        selected,
        dragging,
        on_step: Box::new(on_step),
        open,
    })
}

fn span_of(unit: &Unit) -> Span<'static, String> {
    let mut font = if unit.code {
        iced::Font::MONOSPACE
    } else {
        iced::Font::DEFAULT
    };
    if unit.bold {
        font.weight = iced::font::Weight::Bold;
    }
    if unit.italic {
        font.style = iced::font::Style::Italic;
    }
    iced::widget::span(unit.text.clone()).font(font).color(unit.color)
}

#[derive(Default)]
struct State {
    paragraph: Para,
    /// The text the paragraph was built from, to tell when it must be built again.
    texts: Vec<String>,
    hovered: Option<usize>,
    /// The link pressed on, while the press has not moved. Releasing on it opens it.
    pressed_link: Option<usize>,
}

/// Joins rectangles that sit on the same line and touch, so a run of code is one chip, not one per
/// character.
fn merged(mut rects: Vec<Rectangle>) -> Vec<Rectangle> {
    rects.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    let mut out: Vec<Rectangle> = Vec::new();
    for rect in rects {
        match out.last_mut() {
            Some(last) if (last.y - rect.y).abs() < 1.0 && rect.x <= last.x + last.width + 1.5 => {
                let right = (last.x + last.width).max(rect.x + rect.width);
                last.width = right - last.x;
                last.height = last.height.max(rect.height);
            }
            _ => out.push(rect),
        }
    }
    out
}

/// The span under `point`, or the nearest one when the point is between spans, at a line's end, or
/// below the last line. Selecting over several lines needs this, because the pointer is rarely right on
/// a character.
fn span_at(paragraph: &Para, span_count: usize, point: Point) -> Option<usize> {
    if let Some(index) = paragraph.hit_span(point) {
        return Some(index);
    }
    let mut best: Option<(f32, usize)> = None;
    for index in 0..span_count {
        for rect in paragraph.span_bounds(index) {
            let dy = if point.y < rect.y {
                rect.y - point.y
            } else if point.y > rect.y + rect.height {
                point.y - (rect.y + rect.height)
            } else {
                0.0
            };
            let dx = if point.x < rect.x {
                rect.x - point.x
            } else if point.x > rect.x + rect.width {
                point.x - (rect.x + rect.width)
            } else {
                0.0
            };
            // A line above or below counts for more than a step across the same line.
            let score = dy * 1000.0 + dx;
            if best.is_none_or(|(b, _)| score < b) {
                best = Some((score, index));
            }
        }
    }
    best.map(|(_, index)| index)
}

impl<Message> Selectable<'_, Message> {
    /// The bounds of the spans in `range`, merged, relative to the paragraph.
    fn regions(&self, state: &State, range: Range<usize>) -> Vec<Rectangle> {
        let rects: Vec<Rectangle> = range.flat_map(|i| state.paragraph.span_bounds(i)).collect();
        merged(rects)
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for Selectable<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> iced::Size<Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        layout::sized(limits, Length::Shrink, Length::Shrink, |limits| {
            let bounds = limits.max();
            let size = self.size;
            let font = renderer.default_font();
            let texts: Vec<String> = self.units.iter().map(|u| u.text.clone()).collect();
            let spans: Vec<Span<'static, String>> = self.units.iter().map(span_of).collect();
            let build = || text::Text {
                content: spans.as_slice(),
                bounds,
                size: iced::Pixels(size),
                line_height: text::LineHeight::default(),
                font,
                align_x: text::Alignment::Default,
                align_y: iced::alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::default(),
            };
            if state.texts != texts {
                state.paragraph = <Para as text::Paragraph>::with_spans(build());
                state.texts = texts;
            } else {
                match state.paragraph.compare(text::Text {
                    content: (),
                    bounds,
                    size: iced::Pixels(size),
                    line_height: text::LineHeight::default(),
                    font,
                    align_x: text::Alignment::Default,
                    align_y: iced::alignment::Vertical::Top,
                    shaping: text::Shaping::Advanced,
                    wrapping: text::Wrapping::default(),
                }) {
                    text::Difference::None => {}
                    text::Difference::Bounds => state.paragraph.resize(bounds),
                    text::Difference::Shape => {
                        state.paragraph = <Para as text::Paragraph>::with_spans(build());
                    }
                }
            }
            state.paragraph.min_bounds()
        })
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if !layout.bounds().intersects(viewport) {
            return;
        }
        let state = tree.state.downcast_ref::<State>();
        let origin = layout.position() - Point::ORIGIN;

        // Backgrounds of code, then the selection on top of them.
        let mut i = 0;
        while i < self.units.len() {
            let Some(background) = self.units[i].background else {
                i += 1;
                continue;
            };
            let start = i;
            while i < self.units.len() && self.units[i].background == Some(background) {
                i += 1;
            }
            for rect in self.regions(state, start..i) {
                let chip = Rectangle::new(
                    Point::new(rect.x - 4.0, rect.y - 1.0),
                    Size::new(rect.width + 8.0, rect.height + 2.0),
                );
                renderer.fill_quad(
                    Quad {
                        bounds: chip + origin,
                        border: border::rounded(3.0),
                        ..Quad::default()
                    },
                    background,
                );
            }
        }
        if let Some(range) = self.selected.clone() {
            for rect in self.regions(state, range) {
                renderer.fill_quad(
                    Quad {
                        bounds: rect + origin,
                        ..Quad::default()
                    },
                    self.selection,
                );
            }
        }

        // Underlines and strikethroughs, under the text.
        for (index, unit) in self.units.iter().enumerate() {
            if !unit.underline && !unit.strike {
                continue;
            }
            for rect in state.paragraph.span_bounds(index) {
                let color = unit.color;
                if unit.underline {
                    let line = Rectangle::new(
                        Point::new(rect.x, rect.y + rect.height - 2.0),
                        Size::new(rect.width, 1.0),
                    );
                    renderer.fill_quad(
                        Quad {
                            bounds: line + origin,
                            ..Quad::default()
                        },
                        color,
                    );
                }
                if unit.strike {
                    let line = Rectangle::new(
                        Point::new(rect.x, rect.y + rect.height * 0.5),
                        Size::new(rect.width, 1.0),
                    );
                    renderer.fill_quad(
                        Quad {
                            bounds: line + origin,
                            ..Quad::default()
                        },
                        color,
                    );
                }
            }
        }

        widget_text::draw(
            renderer,
            defaults,
            layout.bounds(),
            &state.paragraph,
            widget_text::Style::default(),
            viewport,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let inside = cursor.position_in(layout.bounds());
        let hovered = inside.and_then(|p| state.paragraph.hit_span(p));
        if hovered != state.hovered {
            state.hovered = hovered;
            shell.request_redraw();
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(index) = inside.and_then(|p| span_at(&state.paragraph, self.units.len(), p)) {
                    state.pressed_link = self.units[index].link.as_ref().map(|_| index);
                    shell.publish((self.on_step)(Step::Press(index)));
                    shell.capture_event();
                } else if self.selected.as_ref().is_some_and(|r| !r.is_empty()) {
                    shell.publish((self.on_step)(Step::Outside));
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                // A drag that starts on a link is a selection, not a click on the link.
                state.pressed_link = None;
                // A drag counts as over this block while the pointer is level with it, even past the end of
                // its lines, so the nearest character is taken.
                if self.dragging {
                    if let Some(p) = cursor.position() {
                        let bounds = layout.bounds();
                        if p.y >= bounds.y && p.y <= bounds.y + bounds.height {
                            let local = Point::new(p.x - bounds.x, p.y - bounds.y);
                            if let Some(index) = span_at(&state.paragraph, self.units.len(), local) {
                                shell.publish((self.on_step)(Step::Over(index)));
                            }
                        }
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(index) = state.pressed_link.take() {
                    if hovered == Some(index) {
                        if let Some(url) = self.units[index].link.clone() {
                            shell.publish((self.open)(url));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        match cursor
            .position_in(layout.bounds())
            .and_then(|p| state.paragraph.hit_span(p))
        {
            Some(index) if self.units[index].link.is_some() => mouse::Interaction::Pointer,
            Some(_) => mouse::Interaction::Text,
            None => mouse::Interaction::None,
        }
    }
}

#[cfg(test)]
#[path = "../tests/view/selectable.rs"]
mod tests;
