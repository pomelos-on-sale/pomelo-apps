//! Modal password prompt sheet and custom password entry line for Wi-Fi.

use iced::border::Radius;
use iced::widget::{container, opaque, text, Column, Row, Space};
use iced::{Alignment, Border, Length, Padding};
use pomelo_material_symbols::Icon;
use pomelo_widgets::touch_keyboard;
use pomelo_widgets::{SystemPreferences, ThemeMode};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::card::Tile;
use crate::pages::common::{capsule_button, confirm_style, dismiss_style, text_button};
use crate::style;
use crate::{Message, UI};
use super::Wifi;

/// The password prompt, when one is open: the network, what has been typed, and the keyboard.
///
/// It is the whole screen — a dimmed backdrop that takes the finger, with the card on it — because
/// that is what makes the list behind it unreachable. `None` when no prompt is open, which is the
/// page as it stands.
/// The password prompt, when one is open: the sheet at the foot of the panel, and the dimming that
/// keeps the page behind it out of reach.
///
/// `None` when no prompt is open, which is the page as it stands.
///
/// One layer: the dimming and the sheet are one `container`, because the rectangle that covers the
/// page is the same rectangle the sheet stands in.
pub(crate) fn password_prompt<'a>(preferences: SystemPreferences, wifi: &Wifi) -> Option<UI<'a>> {
    let ap = wifi.prompt.and_then(|index| wifi.access_points.get(index))?;
    let language = preferences.language;
    let theme = preferences.theme;

    // The two answers, at the head of the sheet, one at each end. Glyphs and not words: the shape is
    // the capsule the page's back button is, a cross and a tick need no translating, and the pair of
    // them says which is which without a sentence.
    let answers = Row::with_children(vec![
        capsule_button(Icon::CLOSE, Some(Message::WifiCancel), dismiss_style, theme),
        Space::new().width(Length::Fill).into(),
        capsule_button(
            Icon::CHECK,
            if wifi.pending {
                None
            } else {
                Some(Message::WifiConnect)
            },
            confirm_style,
            theme,
        ),
    ])
    .width(Length::Fill);

    let mut children: Vec<UI<'a>> = vec![
        answers.into(),
        Space::new().height(Length::Fixed(style::PROMPT_GAP)).into(),
        text(ap.ssid.clone())
            .size(style::NAV_FONT)
            .color(style::label_for(theme))
            .into(),
        Space::new()
            .height(Length::Fixed(style::PROMPT_GAP))
            .into(),
        // One row and not two: a tile, the same row every list in this app is built from, with the
        // label on the left and the field on the right. `Tile::field` and not `Tile::action`,
        // because there is nothing behind this row to open and nothing for a press to do.
        Tile::field(language.text(Key::Password))
            .secondary(password_line(&wifi.password, wifi.revealed, theme))
            .view(theme),
    ];

    if wifi.failed {
        children.push(
            Space::new()
                .height(Length::Fixed(style::PROMPT_GAP))
                .into(),
        );
        children.push(
            text(language.text(Key::ConnectFailed))
                .size(style::VALUE_FONT)
                .color(style::error())
                .into(),
        );
    }

    // No gap above the button: `text_button` carries `ROW_PADDING_V` over its own label, and the
    // tile above carries as much under it, so a `Space` here would be a third helping of the same
    // 16 pixels — and the keys at the foot of the sheet are what pays for it.
    children.push(text_button(
        language.text(if wifi.revealed { Key::Hide } else { Key::Show }),
        Some(Message::WifiReveal),
        theme,
    ));

    // The question, with the panel's margins on both sides. It takes whatever the band under it does
    // not, and its bottom edge is where the keys begin — the keys have no margin of their own, so the
    // width and every pixel of slack above them belong to them.
    //
    // `Fill`, so the keys sit on the panel's floor and stay exactly `keyboard_height()` tall whatever
    // the question above them says.
    //
    // Nothing *below* the question, either: the text button at its foot carries `ROW_PADDING_V` of
    // its own for the finger to land on, and a second margin under that is 16 px of nothing between
    // it and the keys.
    let half = container(Column::with_children(children).width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top: style::PROMPT_PADDING,
            bottom: 0.0,
            left: style::PROMPT_PADDING,
            right: style::PROMPT_PADDING,
        });

    // The band's own height and not the sheet's leftovers. The formula is the keyboard crate's, so
    // this is the same keyboard the terminal draws at the same size on the same panel — see
    // `style::keyboard_height`. A `Fill` here would make the keys whatever the question left them,
    // which is a different keyboard on every password.
    let keys = container(touch_keyboard::band_with_theme(
        wifi.keyboard,
        Message::WifiKey,
        theme,
    ))
    .width(Length::Fill)
    .height(Length::Fixed(style::keyboard_height()));

    let sheet = container(
        Column::with_children(vec![half.into(), keys.into()])
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_theme| container::Style {
        background: Some(style::card_for(theme).into()),
        border: Border {
            color: style::card_border_for(theme),
            width: style::CARD_BORDER_WIDTH,
            // Rounded at the top only: the other three edges are the panel's own, so there is no
            // corner there to round.
            radius: Radius {
                top_left: style::PROMPT_RADIUS,
                top_right: style::PROMPT_RADIUS,
                ..Radius::default()
            },
        },
        ..container::Style::default()
    });

    // The dimming, and the whole of the modal. No margin on the sides or the floor: the sheet is as
    // wide as the panel and stands on it. The only gap is the one at the top, and the page showing
    // through it is what says this is a layer *over* the page rather than the page.
    //
    // `opaque` is what takes the finger. It captures any press inside its bounds, and its bounds are
    // the whole panel, so the list behind — a screen of buttons — never sees it.
    Some(opaque(
        container(sheet)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                top: style::PROMPT_TOP_GAP,
                bottom: 0.0,
                left: 0.0,
                right: 0.0,
            })
            .style(|_theme| container::Style {
                background: Some(style::backdrop().into()),
                ..container::Style::default()
            }),
    ))
}

/// What has been typed: dots and a caret, or the password itself once it is revealed.
pub(crate) fn password_line<'a>(password: &str, revealed: bool, theme: ThemeMode) -> UI<'a> {
    if revealed {
        return Row::with_children(vec![
            text(password.to_string())
                .size(style::DETAIL_FONT)
                .color(style::label_for(theme))
                .into(),
            Space::new().width(Length::Fixed(style::CARET_GAP)).into(),
            caret(),
        ])
        .align_y(Alignment::Center)
        .height(Length::Fixed(style::LINE_H))
        .into();
    }

    let dot_color = style::label_for(theme);
    let dots: Vec<UI<'a>> = password
        .chars()
        .map(move |_| {
            container(Space::new())
                .width(Length::Fixed(style::PASSWORD_DOT))
                .height(Length::Fixed(style::PASSWORD_DOT))
                .style(move |_theme| container::Style {
                    background: Some(dot_color.into()),
                    border: Border {
                        radius: (style::PASSWORD_DOT / 2.0).into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                })
                .into()
        })
        .collect();

    // The dots in a row of their own, so the caret can stand closer to the last one than two dots
    // stand to each other.
    let dots = Row::with_children(dots)
        .spacing(style::PASSWORD_DOT_GAP)
        .align_y(Alignment::Center);

    Row::with_children(vec![
        dots.into(),
        Space::new().width(Length::Fixed(style::CARET_GAP)).into(),
        caret(),
    ])
    .align_y(Alignment::Center)
    .height(Length::Fixed(style::LINE_H))
    .into()
}

/// The caret that says where the next character goes.
///
/// Painted, not typed: the font has no `•` and no block, so what marks the cursor is a rectangle —
/// the same choice the terminal makes for its input line.
///
/// It hangs from the foot of the line rather than sitting in its middle. A line box carries an
/// ascender and a descender and the letters only use the part between them, so a caret centred in the
/// box floats above the letters it is meant to stand among. The `CARET_DROP` under it is that
/// descender, given back.
pub(crate) fn caret<'a>() -> UI<'a> {
    let bar = container(Space::new())
        .width(Length::Fixed(style::CARET_W))
        .height(Length::Fixed(style::CARET_H))
        .style(|_theme| container::Style {
            background: Some(style::accent().into()),
            ..container::Style::default()
        });

    // A `Column` and not a container with a bottom-aligned child: the row the caret goes into is
    // `LINE_H` tall, the bar is at the column's foot, and the column's own bottom margin is what
    // holds the bar up off the line's floor.
    Column::with_children(vec![
        Space::new().height(Length::Fill).into(),
        bar.into(),
        Space::new().height(Length::Fixed(style::CARET_DROP)).into(),
    ])
    .height(Length::Fill)
    .into()
}
