//! Modal password prompt ActionSheet and custom password entry line for Wi-Fi.

use iced::widget::{button, container, text, text_input, Column, Row};
use iced::{Color, Length, Padding};
use pomelo_material_symbols::Icon;
use pomelo_widgets::touch_keyboard;
use pomelo_widgets::{ActionItem, ActionSheet, SystemPreferences, ThemeMode};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::card::Tile;
use crate::style;
use crate::{Message, UI};
use super::Wifi;

/// The password prompt, when one is open: the network, what has been typed, and the keyboard.
///
/// Implemented as a standard Pomelo OS ActionSheet anchored at the foot of the panel
/// occupying 100% screen height with top capsule controls and a dimmed backdrop.
pub(crate) fn password_prompt<'a>(preferences: SystemPreferences, wifi: &Wifi) -> Option<ActionSheet<'a, Message>> {
    let ap = wifi.prompt.and_then(|index| wifi.access_points.get(index))?;
    let language = preferences.language;
    let theme = preferences.theme;

    // Eye toggle button placed directly at the trailing end inside the password card
    let reveal_icon = if wifi.revealed {
        Icon::VISIBILITY_OFF
    } else {
        Icon::VISIBILITY
    };

    let reveal_btn = button(
        pomelo_widgets::icon(reveal_icon)
            .size(24)
            .color(style::label_for(theme)),
    )
    .on_press(Message::WifiReveal)
    .padding(Padding {
        top: 2.0,
        bottom: 2.0,
        left: 6.0,
        right: 4.0,
    })
    .style(|_theme, status| button::Style {
        background: if matches!(status, button::Status::Pressed) {
            Some(Color::from_rgba(1.0, 1.0, 1.0, 0.12).into())
        } else {
            None
        },
        border: iced::Border {
            radius: 8.0.into(),
            ..iced::Border::default()
        },
        ..button::Style::default()
    });

    let password_entry = Row::with_children(vec![
        password_input(&wifi.password, wifi.revealed, theme),
        reveal_btn.into(),
    ])
    .align_y(iced::Alignment::Center)
    .width(Length::Fill);

    let mut children: Vec<UI<'a>> = vec![
        Tile::field(language.text(Key::Password))
            .secondary(password_entry)
            .view(theme),
    ];

    if wifi.failed {
        children.push(
            text(language.text(Key::ConnectFailed))
                .size(style::VALUE_FONT)
                .color(style::error())
                .into(),
        );
    }

    let half = container(
        Column::with_children(children)
            .width(Length::Fill)
            .spacing(style::PROMPT_GAP),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(Padding {
        top: 6.0,
        bottom: 0.0,
        left: style::PROMPT_PADDING,
        right: style::PROMPT_PADDING,
    });

    let keys = container(touch_keyboard::band_with_theme(
        wifi.keyboard,
        Message::WifiKey,
        theme,
    ))
    .width(Length::Fill)
    .height(Length::Fixed(style::keyboard_height()));

    let body = Column::with_children(vec![half.into(), keys.into()])
        .width(Length::Fill)
        .height(Length::Fill);

    let connect_action = ActionItem::new("Connect", Message::WifiConnect)
        .icon(Icon::CHECK)
        .enabled(!wifi.pending);

    let cancel_action = ActionItem::secondary("Cancel", Message::WifiCancel)
        .icon(Icon::CLOSE);

    Some(
        ActionSheet::new(theme)
            .title_icon(Icon::WIFI)
            .title(ap.ssid.clone())
            .action_item(connect_action)
            .cancel_item(cancel_action)
            .custom_body(body),
    )
}

/// The widget ID for the Wi-Fi password text input, used for focus management.
pub(crate) const PASSWORD_INPUT_ID: &str = "wifi_password_input";

/// The password entry field using iced's built-in `text_input`.
pub(crate) fn password_input<'a>(password: &str, revealed: bool, theme: ThemeMode) -> UI<'a> {
    text_input("", password)
        .id(PASSWORD_INPUT_ID)
        .secure(!revealed)
        .size(style::DETAIL_FONT)
        .padding(0.0)
        .width(Length::Fill)
        .style(move |_theme, _status| text_input::Style {
            background: Color::TRANSPARENT.into(),
            border: iced::Border::default(),
            icon: Color::TRANSPARENT,
            placeholder: style::separator_for(theme),
            value: style::label_for(theme),
            selection: style::accent(),
        })
        .on_input(Message::WifiPasswordChanged)
        .on_submit(Message::WifiConnect)
        .into()
}
