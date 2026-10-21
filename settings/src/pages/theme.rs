//! The theme page: dark/light mode switch, font size, colour palette and display details.

use iced::widget::{container, slider, text, Column, Row, Space};
use iced::{Alignment, Border, Color, Element, Length, Padding};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{
    Item, ItemBadge, ItemGroup, OpenPage, Section,
    SystemPreferences,
};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::common::{back_bar, body, detail_card, page, section_card, Page, UI};
use crate::style;
use crate::{Message, SettingsSection};

pub(crate) fn theme_page<'a>(preferences: SystemPreferences) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    // 0. 屏幕亮度调节 slider 卡片
    let brightness_slider = slider(10..=100, preferences.brightness, Message::SetBrightness)
        .step(1)
        .width(Length::Fill)
        .style(move |_t, _status| {
            slider::Style {
                rail: slider::Rail {
                    backgrounds: (
                        theme.accent().into(),
                        Color::from_rgba(1.0, 1.0, 1.0, 0.12).into(),
                    ),
                    width: 10.0,
                    border: Border {
                        radius: 5.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
                handle: slider::Handle {
                    shape: slider::HandleShape::Circle { radius: 14.0 },
                    background: Color::WHITE.into(),
                    border_width: 2.0,
                    border_color: theme.accent(),
                },
            }
        });

    let slider_container = container(brightness_slider)
        .padding(Padding {
            top: 16.0,
            bottom: 16.0,
            left: 16.0,
            right: 16.0,
        })
        .width(Length::Fill);

    let brightness_card: Element<'a, Message> = Section::new(slider_container, theme)
        .header(language.text(Key::Brightness))
        .view();

    let theme_badge = ItemBadge::new(
        if preferences.theme.is_dark() {
            Icon::DARK_MODE
        } else {
            Icon::LIGHT_MODE
        },
        style::IconColor::for_section(SettingsSection::Theme).color(),
    );
    let font_badge = ItemBadge::new(
        Icon::FORMAT_SIZE,
        style::IconColor::for_section(SettingsSection::SystemInfo).color(),
    );
    let screen_off_badge = ItemBadge::new(
        Icon::SCREEN_LOCK_PORTRAIT,
        style::IconColor::for_section(SettingsSection::ScreenOffTimeout).color(),
    );
    let usb_badge = ItemBadge::new(Icon::USB, style::IconColor::Green.color());

    let controls = Section::group(
        ItemGroup::new()
            .push(
                Item::button(
                    language.text(Key::DarkMode),
                    Message::SetTheme(preferences.theme.other()),
                )
                .with_badge(theme_badge)
                .with_value(preferences.theme.name(language)),
            )
            .push(
                Item::button(language.text(Key::FontSize), Message::CycleFontTier)
                    .with_badge(font_badge)
                    .with_value(preferences.font_tier.name(language)),
            )
            .push(
                Item::nav(
                    language.text(Key::ScreenOff),
                    Some(preferences.screen_off_timeout.name(language)),
                    OpenPage(Message::Open(SettingsSection::ScreenOffTimeout)),
                )
                .with_badge(screen_off_badge),
            )
            .push(
                Item::switch(
                    language.text(Key::StayAwakeOnUsb),
                    preferences.stay_awake_on_usb,
                    Message::ToggleStayAwakeOnUsb,
                )
                .with_badge(usb_badge),
            ),
        theme,
    )
    .view();

    let colors = style::palette();

    let top = Row::with_children(
        colors[..4]
            .iter()
            .map(|color| chip(*color, theme))
            .collect::<Vec<_>>(),
    )
    .spacing(style::CHIP_GAP);
    let bottom = Row::with_children(
        colors[4..]
            .iter()
            .map(|color| chip(*color, theme))
            .collect::<Vec<_>>(),
    )
    .spacing(style::CHIP_GAP);

    let palette = Section::new(
        container(
            Column::with_children(vec![
                text(language.text(Key::Presets))
                    .size(style::DETAIL_FONT)
                    .color(style::label_for(theme))
                    .into(),
                Space::new().height(Length::Fixed(style::CHIP_GAP)).into(),
                top.into(),
                Space::new().height(Length::Fixed(style::CHIP_GAP)).into(),
                bottom.into(),
            ])
            .width(Length::Fill)
            .align_x(Alignment::Center),
        )
        .padding(style::PALETTE_PADDING),
        theme,
    )
    .view();

    let details = vec![
        (
            language.text(Key::Style),
            preferences.theme.name(language).to_string(),
        ),
        (language.text(Key::Wallpaper), "macOS Sierra".to_string()),
        (
            language.text(Key::Emissive),
            "AMOLED (no backlight bleed)".to_string(),
        ),
        (
            language.text(Key::RefreshRate),
            "60 Hz Direct QSPI DMA".to_string(),
        ),
        (
            language.text(Key::AntiAliasing),
            "subpixel coverage (analytical AA)".to_string(),
        ),
    ];

    page(
        back_bar(theme),
        body(
            vec![
                section_card(
                    Icon::PALETTE,
                    SettingsSection::Theme,
                    language.text(Key::ThemeTitle),
                    theme,
                ),
                brightness_card,
                controls,
                palette,
                detail_card(details, theme),
            ],
            theme,
        ),
    )
}


/// One preset colour chip, 60x40.
fn chip<'a>(color: iced::Color, theme: pomelo_widgets::ThemeMode) -> UI<'a> {
    container(Space::new())
        .width(Length::Fixed(style::CHIP_W))
        .height(Length::Fixed(style::CHIP_H))
        .style(move |_theme| container::Style {
            background: Some(color.into()),
            border: Border {
                color: style::chip_border_for(theme),
                width: 1.0,
                radius: style::CHIP_RADIUS.into(),
            },
            ..container::Style::default()
        })
        .into()
}
