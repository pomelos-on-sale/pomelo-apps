//! Layout helpers shared by every page: `page`, `title`, `body`, `card_surface`,
//! `separator`, `detail_rows`, `detail_card`, `switch_row`, `usage_bar`, `toggle`,
//! `action_row`, `notice_card`, `row_style`, `toggle_style`.

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, container, rule, scrollable, text, Column, Row, Space, Stack};
use iced::{Alignment, Border, Color, Element, Length, Padding, Renderer, Shadow, Theme};

use crate::style;
use crate::Message;
use pomelo_material_symbols::Icon;
use pomelo_widgets::ThemeMode;

pub use pomelo_widgets::{confirm_dialog, ActionItem, ActionSheet, ActionStyle};

/// The element type every builder in this crate speaks.
pub(crate) type UI<'a> = Element<'a, Message>;

/// The id the page body's `scrollable` carries.
///
/// One id for every page: only one page is ever in the tree, so there is nothing to tell apart, and
/// a single name is what lets a change of section carry one [`Task`](iced::Task) that sends the new
/// page to the top. [`crate::to_top`] is where that is built.
pub(crate) const BODY: &str = "settings-body";

// =============================================================================
// Shared layout primitives
// =============================================================================

/// A top-level page widget whose root is a stable [`Stack`].
///
/// Layer 0: The scrollable page content (`head` + `body`).
/// Additional layers (e.g. modals, dialogs, toasts) can be pushed on top
/// via [`.push(...)`](Page::push) or [`.push_opt(...)`](Page::push_opt)
/// without altering the widget tree root tag, preserving child scroll state.
pub(crate) struct Page<'a> {
    stack: Stack<'a, Message>,
}

impl<'a> Page<'a> {
    /// Creates a new page whose base layer is a scrollable with `head` and `body`.
    pub fn new(head: impl Into<UI<'a>>, body: impl Into<UI<'a>>) -> Self {
        let base = scrollable(Column::with_children(vec![head.into(), body.into()]).width(Length::Fill))
            .id(BODY)
            .direction(Direction::Vertical(Scrollbar::hidden()))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset()));

        let stack = Stack::with_capacity(3)
            .width(Length::Fill)
            .height(Length::Fill)
            .push(base);

        Self { stack }
    }

    /// Pushes an overlay (e.g. a dialog, action sheet, or toast) onto the page.
    #[allow(dead_code)]
    pub fn push(mut self, overlay: impl Into<UI<'a>>) -> Self {
        self.stack = self.stack.push(overlay.into());
        self
    }

    /// Conditionally pushes an overlay if `Some`.
    pub fn push_opt(mut self, overlay: Option<impl Into<UI<'a>>>) -> Self {
        if let Some(item) = overlay {
            self.stack = self.stack.push(item.into());
        }
        self
    }
}

impl<'a> From<Page<'a>> for UI<'a> {
    fn from(page: Page<'a>) -> Self {
        page.stack.into()
    }
}

/// A page: one `scrollable` over the head and the body together, wrapped in a [`Page`] stack.
pub(crate) fn page<'a>(head: impl Into<UI<'a>>, body: impl Into<UI<'a>>) -> Page<'a> {
    Page::new(head, body)
}

/// A page that optionally hosts a modal overlay (e.g. an [`ActionSheet`] or a [`Dialog`]).
pub(crate) fn page_with_modal<'a>(
    head: impl Into<UI<'a>>,
    body: impl Into<UI<'a>>,
    modal: Option<impl Into<UI<'a>>>,
) -> Page<'a> {
    page(head, body).push_opt(modal)
}

/// A page that optionally hosts a modal [`ActionSheet`].
pub(crate) fn page_with_sheet<'a>(
    head: impl Into<UI<'a>>,
    body: impl Into<UI<'a>>,
    sheet: Option<ActionSheet<'a, Message>>,
) -> Page<'a> {
    page(head, body).push_opt(sheet.map(|s| s.view()))
}

/// The heading of a list: the list's own first line, not a bar over it.
///
/// Left-aligned, on the page's background rather than in a band of its own. It carries both gaps
/// around itself — `TITLE_TOP_GAP` above and `TITLE_GAP` below — because the body under it states
/// neither: how far a heading is from the panel's edge, and from what it heads, is the heading's
/// business.
pub(crate) fn title<'a>(title: &'static str, theme: ThemeMode) -> UI<'a> {
    container(
        text(title)
            .size(style::TITLE_FONT)
            .color(style::label_for(theme)),
    )
    .width(Length::Fill)
    .padding(Padding {
        top: style::TITLE_TOP_GAP,
        bottom: style::TITLE_GAP,
        left: style::PAGE_MARGIN,
        right: style::PAGE_MARGIN,
    })
    .into()
}

use std::borrow::Cow;

/// A grouped card container in the settings layout.
///
/// A group is always rendered with the standard card surface (rounded corners,
/// background, border), enforcing visual consistency across all settings pages.
pub(crate) struct Group<'a> {
    content: UI<'a>,
}

impl<'a> Group<'a> {
    /// Creates a group wrapping arbitrary content in the standard card surface.
    #[allow(dead_code)]
    pub(crate) fn custom(content: impl Into<UI<'a>>, theme: ThemeMode) -> Self {
        Self {
            content: card_surface(content, theme),
        }
    }

    /// Creates a group from content that is already on a card surface.
    pub(crate) fn from_surface(surface: UI<'a>) -> Self {
        Self { content: surface }
    }

    /// Returns the underlying UI element.
    pub(crate) fn view(self) -> UI<'a> {
        self.content
    }
}

/// An item that can be placed in a Settings page body.
///
/// The settings page layout is strictly constrained at the type level: only
/// groups (cards), subheadings (section titles), and description texts (footnotes)
/// are allowed in a page body. Raw widgets, arbitrary buttons, or unstyled texts
/// cannot be added directly to the page body.
pub(crate) enum PageItem<'a> {
    /// A grouped container with card styling and rounded corners.
    Group(Group<'a>),
    /// A section subheading above a group.
    Subheading(Cow<'static, str>),
    /// A descriptive footnote text below a group.
    #[allow(dead_code)]
    Description(Cow<'static, str>),
}

impl<'a> PageItem<'a> {
    #[allow(dead_code)]
    pub(crate) fn group(group: impl Into<Group<'a>>) -> Self {
        Self::Group(group.into())
    }

    pub(crate) fn subheading(text: impl Into<Cow<'static, str>>) -> Self {
        Self::Subheading(text.into())
    }

    #[allow(dead_code)]
    pub(crate) fn description(text: impl Into<Cow<'static, str>>) -> Self {
        Self::Description(text.into())
    }
}

impl<'a> From<Group<'a>> for PageItem<'a> {
    fn from(group: Group<'a>) -> Self {
        PageItem::Group(group)
    }
}

/// A section subheading (title above a group).
pub(crate) fn subheading<'a>(title: impl Into<Cow<'static, str>>) -> PageItem<'a> {
    PageItem::subheading(title)
}

/// A section description (footnote below a group).
#[allow(dead_code)]
pub(crate) fn description<'a>(text: impl Into<Cow<'static, str>>) -> PageItem<'a> {
    PageItem::description(text)
}

/// A page body: strictly accepts only [`PageItem`] elements (Group, Subheading, Description).
///
/// Standard gaps are applied between sections:
/// - Subheading to Group: `SUBHEADING_BOTTOM_GAP` (tight)
/// - Group to Description: `DESCRIPTION_TOP_GAP` (tight)
/// - Group/Description to next Section: `CARD_GAP` (spacious)
pub(crate) fn body<'a>(
    parts: Vec<PageItem<'a>>,
    theme: ThemeMode,
) -> Column<'a, Message, Theme, Renderer> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum ItemKind {
        Group,
        Subheading,
        Description,
    }

    let mut children: Vec<UI<'a>> = Vec::with_capacity(parts.len() * 2);
    let mut prev_kind: Option<ItemKind> = None;

    for item in parts {
        let (kind, element) = match item {
            PageItem::Group(group) => (ItemKind::Group, group.view()),
            PageItem::Subheading(title) => {
                let heading_view = container(
                    text(title)
                        .size(style::SUBHEADING_FONT)
                        .color(style::subheading_for(theme)),
                )
                .padding(Padding {
                    left: style::ROW_PADDING_H,
                    right: style::ROW_PADDING_H,
                    top: 0.0,
                    bottom: 0.0,
                })
                .width(Length::Fill)
                .into();
                (ItemKind::Subheading, heading_view)
            }
            PageItem::Description(desc) => {
                let desc_view = container(
                    text(desc)
                        .size(style::DESCRIPTION_FONT)
                        .color(style::footnote_for(theme)),
                )
                .padding(Padding {
                    left: style::ROW_PADDING_H,
                    right: style::ROW_PADDING_H,
                    top: 0.0,
                    bottom: 0.0,
                })
                .width(Length::Fill)
                .into();
                (ItemKind::Description, desc_view)
            }
        };

        if let Some(prev) = prev_kind {
            let gap = match (prev, kind) {
                (ItemKind::Subheading, ItemKind::Group) => style::SUBHEADING_BOTTOM_GAP,
                (ItemKind::Group, ItemKind::Description) => style::DESCRIPTION_TOP_GAP,
                _ => style::CARD_GAP,
            };
            children.push(Space::new().height(Length::Fixed(gap)).into());
        }

        children.push(element);
        prev_kind = Some(kind);
    }

    Column::with_children(children)
        .padding(Padding {
            top: 0.0,
            bottom: style::BOTTOM_GAP,
            left: style::PAGE_MARGIN,
            right: style::PAGE_MARGIN,
        })
        .width(Length::Fill)
}

/// A rounded surface, as wide as the page it sits in.
///
/// The surface and not the card: [`crate::pages::card::Card`] is what sits on it, and it is named
/// for what it holds rather than for the box.
pub(crate) fn card_surface<'a>(content: impl Into<UI<'a>>, theme: ThemeMode) -> UI<'a> {
    pomelo_widgets::card_surface(content, theme).into()
}

/// The hairline between two rows of a card: iced's own `rule`, inset to the row's text.
///
/// The insets are the row's own measurements — [`style::ROW_TEXT_INSET`] is where the label starts,
/// [`style::ROW_PADDING_H`] is where the chevron ends — so the line is the same layout the row is,
/// rather than a number someone has to keep in step with it. `FillMode` is why this needs no
/// wrapper: iced's `rule` insets itself.
pub(crate) fn separator<'a>(theme: ThemeMode) -> UI<'a> {
    rule::horizontal(style::ROW_SEPARATOR)
        .style(move |_theme| rule::Style {
            color: style::separator_for(theme),
            radius: 0.0.into(),
            fill_mode: rule::FillMode::AsymmetricPadding(
                style::ROW_TEXT_INSET as u16,
                style::ROW_PADDING_H as u16,
            ),
            snap: false,
        })
        .into()
}

/// The rows of a key/value table: one line each, with hairlines between them.
pub(crate) fn detail_rows<'a>(
    rows: Vec<(&'static str, String)>,
    theme: ThemeMode,
) -> Vec<UI<'a>> {
    let last = rows.len().saturating_sub(1);
    let mut children: Vec<UI<'a>> = Vec::new();

    for (index, (key, value)) in rows.into_iter().enumerate() {
        let line = Row::with_children(vec![
            text(key)
                .size(style::DETAIL_FONT)
                .color(style::detail_key_for(theme))
                .into(),
            Space::new().width(Length::Fill).into(),
            text(value)
                .size(style::DETAIL_FONT)
                .color(style::label_for(theme))
                .into(),
        ])
        .width(Length::Fill)
        .align_y(Alignment::Center);

        children.push(
            container(line)
                .width(Length::Fill)
                .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H])
                .into(),
        );

        if index < last {
            children.push(separator(theme));
        }
    }

    children
}

/// A key/value table in a card.
pub(crate) fn detail_card<'a>(rows: Vec<(&'static str, String)>, theme: ThemeMode) -> Group<'a> {
    Group::from_surface(card_surface(
        Column::with_children(detail_rows(rows, theme)).width(Length::Fill),
        theme,
    ))
}

/// A card whose row is a label and a switch.
///
/// A switch row's vertical padding is `ROW_PADDING_V`, the tile's: the two rows are the same row
/// with a different thing at its end, and giving them two numbers is how they came to differ.
pub(crate) fn switch_row<'a>(label: &'static str, control: UI<'a>, theme: ThemeMode) -> Group<'a> {
    let line = Row::with_children(vec![
        text(label)
            .size(style::LABEL_FONT)
            .color(style::label_for(theme))
            .into(),
        Space::new().width(Length::Fill).into(),
        control,
    ])
    .width(Length::Fill)
    .align_y(Alignment::Center);

    Group::from_surface(card_surface(
        container(line).padding([style::ROW_PADDING_V, style::SWITCH_PADDING_H]),
        theme,
    ))
}

/// A card with a labelled progress bar.
///
/// `percent` — not pixels: the two halves of the bar are flex shares, so the same number
/// describes the design's 440pt card and any other width.
pub(crate) fn usage_bar<'a>(
    title: &'static str,
    value: String,
    value_color: Color,
    percent: f32,
    bar_color: Color,
    theme: ThemeMode,
) -> Group<'a> {
    let share = (percent.clamp(0.0, 100.0) * 100.0).round() as u16;
    let rest = 10_000u16.saturating_sub(share);

    let track_content: UI<'a> = if share == 0 {
        Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(style::BAR_HEIGHT))
            .into()
    } else if share >= 10_000 {
        container(Space::new().width(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fixed(style::BAR_HEIGHT))
            .style(move |_theme| container::Style {
                background: Some(bar_color.into()),
                border: Border {
                    radius: style::BAR_RADIUS.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    } else {
        let filled = container(Space::new().width(Length::Fill))
            .width(Length::FillPortion(share))
            .height(Length::Fixed(style::BAR_HEIGHT))
            .style(move |_theme| container::Style {
                background: Some(bar_color.into()),
                border: Border {
                    radius: style::BAR_RADIUS.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            });

        let remaining = container(Space::new().width(Length::Fill))
            .width(Length::FillPortion(rest))
            .height(Length::Fixed(style::BAR_HEIGHT));

        Row::with_children(vec![filled.into(), remaining.into()])
            .width(Length::Fill)
            .height(Length::Fixed(style::BAR_HEIGHT))
            .into()
    };

    let track = container(track_content)
        .width(Length::Fill)
        .height(Length::Fixed(style::BAR_HEIGHT))
        .style(move |_theme| container::Style {
            background: Some(style::bar_track_for(theme).into()),
            border: Border {
                radius: style::BAR_RADIUS.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

    let heading = Row::with_children(vec![
        text(title)
            .size(style::DETAIL_FONT)
            .color(style::label_for(theme))
            .into(),
        Space::new().width(Length::Fill).into(),
        text(value)
            .size(style::DETAIL_FONT)
            .color(value_color)
            .into(),
    ])
    .width(Length::Fill);

    Group::from_surface(card_surface(
        container(
            Column::with_children(vec![
                heading.into(),
                Space::new().height(Length::Fixed(style::BAR_GAP)).into(),
                track.into(),
            ])
            .width(Length::Fill),
        )
        .padding(style::USAGE_PADDING),
        theme,
    ))
}

/// An iOS switch: a 52x28 track with a 24px knob, drawn from containers.
///
/// The whole track is the button. The knob is positioned by a fill on the side it is moving
/// away from, which is what puts it 2px from the edge it rests against without hard-coding the
/// track's own width into the knob's position.
pub(crate) fn toggle<'a>(on: bool, message: Message, theme: ThemeMode) -> UI<'a> {
    let knob = container(Space::new())
        .width(Length::Fixed(style::KNOB))
        .height(Length::Fixed(style::KNOB))
        .style(|_theme| container::Style {
            background: Some(Color::WHITE.into()),
            border: Border {
                radius: style::KNOB_RADIUS.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

    let mut parts: Vec<UI<'a>> = Vec::new();

    if on {
        parts.push(Space::new().width(Length::Fill).into());
        parts.push(knob.into());
        parts.push(
            Space::new()
                .width(Length::Fixed(style::TOGGLE_INSET))
                .into(),
        );
    } else {
        parts.push(
            Space::new()
                .width(Length::Fixed(style::TOGGLE_INSET))
                .into(),
        );
        parts.push(knob.into());
        parts.push(Space::new().width(Length::Fill).into());
    }

    button(
        Row::with_children(parts)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Alignment::Center),
    )
    .width(Length::Fixed(style::TOGGLE_W))
    .height(Length::Fixed(style::TOGGLE_H))
    .padding(0)
    .style(move |_theme, status| toggle_style(on, status, theme))
    .on_press(message)
    .into()
}

/// A row that offers an action — or, with `message` `None`, a row that names a state instead.
///
/// It is a row and not a card, so that it can be the last line of a card that already exists
/// ([`wifi::connection_card`]) as well as a card of its own.
pub(crate) fn action_row<'a>(
    label: &'static str,
    message: Option<Message>,
    theme: ThemeMode,
) -> UI<'a> {
    let color = match message {
        Some(_) => style::accent(),
        None => style::notice_for(theme),
    };

    let line = Row::with_children(vec![
        text(label).size(style::DETAIL_FONT).color(color).into(),
        Space::new().width(Length::Fill).into(),
        text(Icon::KEYBOARD_ARROW_RIGHT.glyph())
            .font(pomelo_material_symbols::font())
            .size(style::CHEVRON_FONT)
            .color(style::chevron_for(theme))
            .into(),
    ])
    .width(Length::Fill)
    .align_y(Alignment::Center);

    let contents = container(line)
        .width(Length::Fill)
        .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H]);

    match message {
        Some(message) => button(contents)
            .width(Length::Fill)
            .padding(0)
            .style(move |_theme, status| row_style(theme, status))
            .on_press(message)
            .into(),
        None => contents.into(),
    }
}

/// A round-ended button carrying a glyph.
pub(crate) fn capsule_button<'a>(
    icon: Icon,
    message: Option<Message>,
    look: fn(ThemeMode, button::Status) -> button::Style,
    theme: ThemeMode,
) -> UI<'a> {
    button(
        container(
            text(icon.glyph())
                .font(pomelo_material_symbols::font())
                .size(style::NAV_GLYPH),
        )
        .center_x(Length::Fixed(style::NAV_BUTTON_W))
        .center_y(Length::Fixed(style::NAV_BUTTON_H)),
    )
    .padding(0)
    .style(move |_theme, status| look(theme, status))
    .on_press_maybe(message)
    .into()
}

/// A card whose single row names a state instead of offering an action.
pub(crate) fn notice_card<'a>(label: &'static str, theme: ThemeMode) -> Group<'a> {
    Group::from_surface(card_surface(
        container(
            text(label)
                .size(style::DETAIL_FONT)
                .color(style::notice_for(theme)),
        )
        .width(Length::Fill)
        .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H]),
        theme,
    ))
}

// =============================================================================
// Styles
// =============================================================================

/// The wash a press leaves on a surface that has no colour of its own.
///
/// Light and dark take opposite films: white over a black card, black over a white one. Either way
/// the card is a little closer to the colour of the finger's shadow, which is all a press has to say.
pub(crate) fn press_wash(theme: ThemeMode) -> Color {
    match theme {
        ThemeMode::Dark => Color::from_rgba(1.0, 1.0, 1.0, 0.11),
        ThemeMode::Light => Color::from_rgba(0.0, 0.0, 0.0, 0.08),
    }
}

/// The shape of a capsule, given its fill and its ink.
///
/// Half the height is the radius, which is what makes the two ends semicircles whatever the width
/// is. The ink is set here rather than on the text because iced hands a button's `text_color` down
/// to whatever is drawn inside it — see `Button::draw` — so a capsule's colour is one decision and
/// not two that can disagree.
fn capsule(fill: Color, ink: Color) -> button::Style {
    button::Style {
        background: Some(fill.into()),
        text_color: ink,
        border: Border {
            radius: (style::NAV_BUTTON_H / 2.0).into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Whether a status is a finger on the button.
fn is_pressed(status: button::Status) -> bool {
    matches!(status, button::Status::Pressed | button::Status::Hovered)
}

/// Back, on the page: the card's own colour.
///
/// The card's and not a bar's, because there is no bar — the button is a shape of its own on the
/// page, and the fill is what makes it read as one.
pub(crate) fn nav_style(theme: ThemeMode, status: button::Status) -> button::Style {
    let fill = if is_pressed(status) {
        style::muted_for(theme)
    } else {
        style::card_for(theme)
    };

    capsule(fill, style::label_for(theme))
}


/// A main-list row: nothing at rest, a wash when the finger is on it.
pub(crate) fn row_style(theme: ThemeMode, status: button::Status) -> button::Style {
    let wash = if matches!(status, button::Status::Pressed) {
        Some(press_wash(theme).into())
    } else {
        None
    };

    button::Style {
        background: wash,
        text_color: style::label_for(theme),
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    }
}

/// A switch's track: green when on, grey when off.
pub(crate) fn toggle_style(
    on: bool,
    status: button::Status,
    theme: ThemeMode,
) -> button::Style {
    let fill = match (on, status) {
        (true, button::Status::Pressed) => style::green_pressed(),
        (true, _) => style::green(),
        (false, button::Status::Pressed) => style::track_off_pressed_for(theme),
        (false, _) => style::track_off_for(theme),
    };

    button::Style {
        background: Some(fill.into()),
        text_color: Color::WHITE,
        border: Border {
            radius: style::TOGGLE_RADIUS.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}



