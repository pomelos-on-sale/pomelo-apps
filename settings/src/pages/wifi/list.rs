//! Wi-Fi network scanning list, network rows, signal bars, and connection card.

use iced::widget::{button, container, text, Column, Row, Space};
use iced::{Alignment, Length};
use pomelo_hal::{ApInfo, WifiState, WifiStatus};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{Language, ThemeMode};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::common::{
    action_row, card_surface, detail_rows, notice_card, row_style, separator, Group,
};
use crate::style;
use crate::{Message, UI};
use super::Wifi;

/// The networks the last scan found, best signal first — or a line saying there are none.
pub(crate) fn network_list<'a>(language: Language, wifi: &Wifi, theme: ThemeMode) -> Group<'a> {
    if wifi.access_points.is_empty() {
        return notice_card(language.text(Key::NoNetworks), theme);
    }

    let connected = if wifi.status.state == WifiState::Connected {
        Some(wifi.status.ssid.as_str())
    } else {
        None
    };

    let last = wifi.access_points.len() - 1;
    let mut rows: Vec<UI<'a>> = Vec::new();

    for (index, ap) in wifi.access_points.iter().enumerate() {
        rows.push(network_row(
            language,
            ap,
            connected == Some(ap.ssid.as_str()),
            index,
            theme,
        ));

        if index < last {
            rows.push(separator(theme));
        }
    }

    Group::from_surface(card_surface(Column::with_children(rows).width(Length::Fill), theme))
}

/// One network: its name, lock icon if secured, connected status, and its wifi signal.
fn network_row<'a>(
    language: Language,
    ap: &ApInfo,
    connected: bool,
    index: usize,
    theme: ThemeMode,
) -> UI<'a> {
    let mut right: Vec<UI<'a>> = Vec::new();

    if ap.secure {
        right.push(
            text(Icon::LOCK.glyph())
                .font(pomelo_material_symbols::font())
                .size(style::SIGNAL_ICON_SIZE)
                .color(style::notice_for(theme))
                .into(),
        );
        right.push(Space::new().width(Length::Fixed(style::SIGNAL_GAP)).into());
    }

    if connected {
        right.push(
            text(language.text(Key::Connected))
                .size(style::VALUE_FONT)
                .color(style::accent())
                .into(),
        );
        right.push(Space::new().width(Length::Fixed(style::SIGNAL_GAP)).into());
    }

    right.push(wifi_signal_icon(ap.signal_bars(), theme));

    let contents = Row::with_children(vec![
        text(ap.ssid.clone())
            .size(style::LABEL_FONT)
            .color(style::label_for(theme))
            .into(),
        Space::new().width(Length::Fill).into(),
        Row::with_children(right)
            .align_y(Alignment::Center)
            .into(),
    ])
    .align_y(Alignment::Center)
    .width(Length::Fill);

    button(
        container(contents)
            .width(Length::Fill)
            .padding([style::ROW_PADDING_V, style::ROW_PADDING_H]),
    )
    .width(Length::Fill)
    .padding(0)
    .style(move |_theme, status| row_style(theme, status))
    .on_press(Message::WifiSelect(index))
    .into()
}

/// A Wi-Fi signal arc icon corresponding to the signal strength level.
fn wifi_signal_icon<'a>(bars: u8, theme: ThemeMode) -> UI<'a> {
    let icon = match bars {
        0 => Icon::WIFI_OFF,
        1 => Icon::WIFI_1_BAR,
        2 => Icon::WIFI_2_BAR,
        _ => Icon::WIFI,
    };
    let color = if bars == 0 {
        style::signal_off_for(theme)
    } else {
        style::signal_on_for(theme)
    };
    text(icon.glyph())
        .font(pomelo_material_symbols::font())
        .size(style::SIGNAL_ICON_SIZE)
        .color(color)
        .into()
}

/// What is known about the connection: the network, the numbers that came with it, and the way out.
pub(crate) fn connection_card<'a>(
    language: Language,
    status: &WifiStatus,
    theme: ThemeMode,
) -> Group<'a> {
    let mut children = detail_rows(
        vec![
            (language.text(Key::Network), status.ssid.clone()),
            (language.text(Key::Signal), format!("{} dBm", status.rssi)),
            (language.text(Key::IpAddress), status.ip.clone()),
            (language.text(Key::Gateway), status.gateway.clone()),
            (language.text(Key::SubnetMask), status.netmask.clone()),
        ],
        theme,
    );

    children.push(separator(theme));
    children.push(action_row(
        language.text(Key::Disconnect),
        Some(Message::WifiDisconnect),
        theme,
    ));

    Group::from_surface(card_surface(Column::with_children(children).width(Length::Fill), theme))
}
