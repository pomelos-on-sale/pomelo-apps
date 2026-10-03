//! The status bar: the clock on the left, the signal and the battery on the right.
//!
//! Three readings dynamically driven by [`Subscription`](iced::Subscription), and
//! two of them are drawn as **icons** out of `pomelo_material_symbols` rather than as shapes of this
//! file's own: a bar chart of four rectangles said "signal" only because the launcher drew one, while
//! [`Icon::WIFI_2_BAR`] is a picture everyone has already seen. The clock stays text, because a
//! number is what it is.
//!
//! # Four bars of signal, three pictures of it
//!
//! `pomelo_hal::signal_bars` answers on a `0..=4` scale — four thresholds ten dBm apart, which is
//! about as finely as an RSSI can honestly be divided — and Material Symbols stops at three bars. So
//! `n` bars draw the `n`-bar icon and the fourth draws the third's picture: the difference between
//! −50 and −60 dBm is not something a 20 px glyph can show, and a picture that overstated the signal
//! would be worse than one that stops at "strong". The settings app's list is where four steps are
//! drawn as four.
//!
//! # Eight pictures of a battery
//!
//! `battery_android_0` through `_6`, then `_full`: seven steps of about fourteen percent each, and a
//! full battery as its own picture. The icon is the whole reading — the same as the signal, and for
//! the same reason: a percentage printed beside a picture of the percentage would be one reading
//! drawn twice, and eight steps are what a 20 px glyph can say.
//!
//! # The row is inset, and the band is not
//!
//! The panel's corners are round ([`style::STATUS_INSET`]): the bar's *black* reaches both edges —
//! it is the top of the display — while what is drawn on it stops short of the corners.

use iced::widget::{container, row, text, Space};
use iced::{Alignment, Color, Element, Length, Padding};
use pomelo_material_symbols::{self as icons, Icon};

use crate::style;
use crate::Message;

/// The bar as an element: `clock` and background app icons on the left, `wifi` bars and `battery` percent on the right.
///
/// The whole bar is one row: the clock, background app icons, space, then the signal and the charge.
/// Nothing here is interactive — the readings are the platform's to push, and the bar has no message
/// of its own — so this is a plain `Element` and not one mapped from a message.
pub fn view<'a>(
    clock: &'a str,
    battery: u8,
    wifi: u8,
    background_icons: &[Icon],
) -> Element<'a, Message> {
    const STATUS_FG: Color = Color::from_rgb(0.0, 0.0, 0.0);

    let icon = |glyph: Icon| {
        text(glyph.glyph())
            .size(style::STATUS_ICON)
            .font(icons::font())
            .color(STATUS_FG)
    };

    let battery_icon_widget = |glyph: Icon| {
        text(glyph.glyph())
            .size(style::STATUS_BATTERY_ICON)
            .font(icons::font())
            .color(STATUS_FG)
    };

    let battery_group = row![
        text(format!("{battery}%"))
            .size(style::STATUS_PERCENT_FONT)
            .color(STATUS_FG),
        battery_icon_widget(battery_icon(battery)),
    ]
    .align_y(Alignment::Center)
    .spacing(style::STATUS_BATTERY_GAP);

    let mut bg_icons_row = row![].align_y(Alignment::Center).spacing(4.0);
    for &bg_icon in background_icons {
        bg_icons_row = bg_icons_row.push(
            text(bg_icon.glyph())
                .size(14.0)
                .font(icons::font())
                .color(Color::from_rgb(0.25, 0.25, 0.25)),
        );
    }

    container(
        row![
            text(clock).size(style::STATUS_FONT).color(STATUS_FG),
            bg_icons_row,
            Space::new().width(Length::Fill),
            icon(wifi_icon(wifi)),
            battery_group,
        ]
        .align_y(Alignment::Center)
        .spacing(style::STATUS_GAP),
    )
    // The row is centred in the band rather than sitting at the top of it: the bar is taller than
    // its text (it is the top of a round display), and the space belongs on both sides.
    .center_y(Length::Fixed(style::STATUS_HEIGHT))
    .width(Length::Fill)
    // The padding is what keeps the clock out of the left corner and the battery out of the right
    // one, and it is the container's own rather than a `Space` in the row: `Row::spacing` would have
    // added the gap between the row's contents a second time.
    .padding(Padding {
        left: style::STATUS_INSET,
        right: style::STATUS_INSET,
        ..Padding::ZERO
    })
    .style(|_theme| container::Style {
        background: Some(
            Color::from_rgb8(style::STATUS_BG.0, style::STATUS_BG.1, style::STATUS_BG.2).into(),
        ),
        ..container::Style::default()
    })
    .into()
}

/// The picture for `bars` of signal, on the HAL's own `0..=`[`style::WIFI_BARS`] scale.
///
/// The fourth bar and anything above it draw the fullest picture the set has, which is the three-bar
/// one; see this module's docs. A count above the scale is not rejected — the set is a picture of
/// "at least this strong", and a reading the platform mis-scaled should not take the bar down.
pub fn wifi_icon(bars: u8) -> Icon {
    match bars {
        0 => Icon::WIFI_OFF,
        1 => Icon::WIFI_1_BAR,
        2 => Icon::WIFI_2_BAR,
        _ => Icon::WIFI,
    }
}

/// The picture for `percent` of charge.
///
/// Eight pictures over a hundred percent, in one expression: `percent * 7 / 100` is 0 at empty, 6 at
/// 99 and 7 at full, so the boundaries fall out of the arithmetic rather than out of a table of
/// thresholds standing beside it — [`BATTERY`] is only the eight pictures, in the order they climb.
/// A value above 100 is clamped: this is a percentage, and a battery that reported 255 would
/// otherwise index past the end.
pub fn battery_icon(percent: u8) -> Icon {
    BATTERY[usize::from(percent.min(100)) * 7 / 100]
}

/// The eight pictures the charge climbs through, in order: seven of a battery filling up, then full.
const BATTERY: [Icon; 8] = [
    Icon::BATTERY_ANDROID_0,
    Icon::BATTERY_ANDROID_1,
    Icon::BATTERY_ANDROID_2,
    Icon::BATTERY_ANDROID_3,
    Icon::BATTERY_ANDROID_4,
    Icon::BATTERY_ANDROID_5,
    Icon::BATTERY_ANDROID_6,
    Icon::BATTERY_ANDROID_FULL,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Every bar the HAL counts is a bar in the picture, up to the three the set has.
    ///
    /// The bounds are the point: a reading of 0 is the crossed-out icon and not an empty bar chart,
    /// and 3 and 4 are the same picture rather than one of them borrowing a glyph that does not
    /// exist.
    #[test]
    fn the_signal_icon_shows_as_many_bars_as_the_hal_counted() {
        for (bars, icon) in [
            (0, Icon::WIFI_OFF),
            (1, Icon::WIFI_1_BAR),
            (2, Icon::WIFI_2_BAR),
            (3, Icon::WIFI),
            (4, Icon::WIFI),
            (9, Icon::WIFI),
        ] {
            assert_eq!(wifi_icon(bars), icon, "{bars} bars");
        }
    }

    /// The charge has eight pictures and the last one is full.
    ///
    /// Each step's two ends, so the boundaries are pinned and not just the shape of the curve:
    /// fourteen percent is still the empty picture, fifteen is the first bar of the climb, and
    /// ninety-nine is the last one before full.
    #[test]
    fn the_battery_icon_climbs_eight_steps_and_the_last_one_is_full() {
        for (percent, icon) in [
            (0, Icon::BATTERY_ANDROID_0),
            (14, Icon::BATTERY_ANDROID_0),
            (15, Icon::BATTERY_ANDROID_1),
            (28, Icon::BATTERY_ANDROID_1),
            (29, Icon::BATTERY_ANDROID_2),
            (43, Icon::BATTERY_ANDROID_3),
            (57, Icon::BATTERY_ANDROID_3),
            (58, Icon::BATTERY_ANDROID_4),
            (72, Icon::BATTERY_ANDROID_5),
            (86, Icon::BATTERY_ANDROID_6),
            (99, Icon::BATTERY_ANDROID_6),
            (100, Icon::BATTERY_ANDROID_FULL),
            (255, Icon::BATTERY_ANDROID_FULL),
        ] {
            assert_eq!(battery_icon(percent), icon, "{percent}%");
        }
    }

    /// A fuller battery is never drawn as a lower picture.
    ///
    /// The whole hundred percent, in order, which is what says the arithmetic in [`battery_icon`]
    /// does not fold back on itself somewhere between the boundaries above.
    ///
    /// The comparison is where the picture sits in [`BATTERY`], not the `Icon`s' own ordering: an
    /// `Icon` orders by its code point, and the code points of a battery filling up run *backwards*
    /// (`_0` is U+F30D and `_full` is U+F304), so `>` on the two would say the climb descends.
    #[test]
    fn a_fuller_battery_is_never_a_lower_picture() {
        let step = |percent| {
            let icon = battery_icon(percent);

            BATTERY
                .iter()
                .position(|candidate| *candidate == icon)
                .expect("every picture of a charge is one of the eight")
        };

        let mut last = step(0);

        for percent in 0..=100u8 {
            assert!(
                step(percent) >= last,
                "{percent}% draws lower than {}%",
                percent - 1
            );

            last = step(percent);
        }
    }
}
