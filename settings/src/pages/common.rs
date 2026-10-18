//! Shared layout primitives, page stack scaffolding, and usage bar component.

use std::borrow::Cow;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, container, scrollable, text, Column, Stack};
use iced::{Border, Element, Length, Padding, Shadow};

use crate::style;
use crate::Message;
use pomelo_widgets::{Section, ThemeMode};

pub(crate) use pomelo_widgets::{confirm_dialog, usage_bar, ActionSheet, UsageCard};
pub(crate) use super::nav::{back_bar, section_card};

/// The element type every builder in this crate speaks.
pub(crate) type UI<'a> = Element<'a, Message>;

/// The id the page body's `scrollable` carries.
pub(crate) const BODY: &str = "settings-body";

// =============================================================================
// Shared page scaffolding
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

/// A section subheading above a section or card.
pub(crate) fn subheading<'a>(title: impl Into<Cow<'static, str>>, theme: ThemeMode) -> UI<'a> {
    container(
        text(title.into())
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
    .into()
}

/// A page body layout container providing standard card spacing and page margins.
pub(crate) fn body<'a>(
    children: Vec<UI<'a>>,
    _theme: ThemeMode,
) -> Column<'a, Message, iced::Theme, iced::Renderer> {
    Column::with_children(children)
        .spacing(style::CARD_GAP)
        .padding(Padding {
            top: 0.0,
            bottom: style::BOTTOM_GAP,
            left: style::PAGE_MARGIN,
            right: style::PAGE_MARGIN,
        })
        .width(Length::Fill)
}

/// Convenience helper to create a key-value detail card using unified [`Section`].
pub(crate) fn detail_card<'a>(
    rows: impl IntoIterator<Item = (impl Into<Cow<'static, str>>, impl Into<Cow<'static, str>>)>,
    theme: ThemeMode,
) -> UI<'a> {
    Section::details(rows, theme).view()
}

/// A card whose single row names a state (e.g. Wi-Fi off or no SD card) instead of offering an action.
pub(crate) fn notice_card<'a>(label: impl Into<Cow<'static, str>>, theme: ThemeMode) -> UI<'a> {
    Section::notice(label, theme).view()
}

/// Row wash style when pressed.
pub(crate) fn row_style(theme: ThemeMode, status: button::Status) -> button::Style {
    let wash = if matches!(status, button::Status::Pressed) {
        Some(theme.card_pressed().into())
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
