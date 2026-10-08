//! Wi-Fi network scanning list, network rows, signal bars, and connection card.

use iced::widget::{button, container, text, Column, Row, Space};
use iced::{Alignment, Border, Length};
use pomelo_hal::{ApInfo, WifiState, WifiStatus};
use pomelo_widgets::{Language, ThemeMode};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::common::{
    action_row, card_surface, detail_rows, notice_card, row_style, separator,
};
use crate::style;
use crate::{Message, UI};
use super::Wifi;

/// The networks the last scan found, best signal first — or a line saying there are none.
pub(crate) fn network_list<'a>(language: Language, wifi: &Wifi, theme: ThemeMode) -> UI<'a> {
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

    card_surface(Column::with_children(rows).width(Length::Fill), theme)
}

/// One network: its name, whether it is locked, whether we are on it, and its signal.
///
/// The lock is a *word* and not a padlock, for the same reason the keyboard's shift key says
/// `shift`: the font is a Chinese and Latin subset with no padlock in it. That is no longer the
/// whole story — `pomelo-material-symbols` is an icon font and `Icon::LOCK` is one call site away
/// — but swapping it is a change to this list's layout, not to this sentence.
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
            text(language.text(Key::Secured))
                .size(style::VALUE_FONT)
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

    right.push(signal_bars(ap.signal_bars(), theme));

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

/// Four bars, the lit ones as tall as the signal — the same scale the launcher's status bar draws.
fn signal_bars<'a>(bars: u8, theme: ThemeMode) -> UI<'a> {
    let count = style::SIGNAL_BARS;
    let heights = style::SIGNAL_BAR_H - style::SIGNAL_BAR_MIN_H;

    let bars = (0..count).map(move |index| {
        let height =
            style::SIGNAL_BAR_MIN_H + heights * f32::from(index + 1) / f32::from(count);
        let lit = index < bars;

        container(Space::new())
            .width(Length::Fixed(style::SIGNAL_BAR_W))
            .height(Length::Fixed(height))
            .style(move |_theme| container::Style {
                background: Some(
                    if lit {
                        style::signal_on_for(theme)
                    } else {
                        style::signal_off_for(theme)
                    }
                    .into(),
                ),
                border: Border {
                    radius: 1.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    });

    Row::with_children(bars)
        .spacing(style::SIGNAL_BAR_GAP)
        .align_y(Alignment::End)
        .into()
}

/// What is known about the connection: the network, the numbers that came with it, and the way out.
pub(crate) fn connection_card<'a>(
    language: Language,
    status: &WifiStatus,
    theme: ThemeMode,
) -> UI<'a> {
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

    card_surface(Column::with_children(children).width(Length::Fill), theme)
}
