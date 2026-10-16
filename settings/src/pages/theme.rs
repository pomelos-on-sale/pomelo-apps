//! The theme page: dark/light mode switch, font size, colour palette and display details.

use iced::widget::{button, container, stack, text, Column, Row, Space};
use iced::{Alignment, Border, Color, Length};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{SleepTimeout, SystemPreferences};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::card::{Card, Header, Tile};
use crate::pages::common::{body, card_surface, detail_card, page, press_wash, separator, UI};
use crate::style;
use crate::{Message, SettingsSection};

pub(crate) fn theme_page<'a>(
    preferences: SystemPreferences,
    show_sleep_timeout_dialog: bool,
) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    // The two rows that change something are tiles — the same row the main list is built from — and
    // not a layout of their own: a value beside a label is exactly what a tile is.
    let controls = Card::new(theme)
        .tile(
            Tile::action(
                if preferences.theme.is_dark() {
                    Icon::DARK_MODE
                } else {
                    Icon::LIGHT_MODE
                },
                style::IconColor::for_section(SettingsSection::Theme),
                language.text(Key::DarkMode),
                Message::SetTheme(preferences.theme.other()),
            )
            .secondary(text(preferences.theme.name(language)))
            .trailing(None),
        )
        .tile(
            Tile::action(
                Icon::FORMAT_SIZE,
                style::IconColor::for_section(SettingsSection::SystemInfo),
                language.text(Key::FontSize),
                Message::CycleFontTier,
            )
            .secondary(text(preferences.font_tier.name(language)))
            .trailing(None),
        )
        .tile(
            Tile::action(
                Icon::SCREEN_LOCK_PORTRAIT,
                style::IconColor::Orange,
                language.text(Key::AutoSleep),
                Message::PromptSleepTimeout,
            )
            .secondary(text(preferences.sleep_timeout.name(language)))
            .trailing(Some(Icon::KEYBOARD_ARROW_RIGHT)),
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

    let palette = card_surface(
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
    );

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

    let pg = page(
        Header::section(
            Icon::PALETTE,
            SettingsSection::Theme,
            language.text(Key::ThemeTitle),
        )
        .view(theme),
        body(vec![controls, palette, detail_card(details, theme)]),
    );

    if show_sleep_timeout_dialog {
        let modal = sleep_timeout_dialog(preferences);
        stack![pg, modal].into()
    } else {
        pg
    }
}

/// Modal dialog displaying options for auto-sleep timeout.
fn sleep_timeout_dialog<'a>(preferences: SystemPreferences) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let header = Row::with_children(vec![
        container(
            text(Icon::SCREEN_LOCK_PORTRAIT.glyph())
                .font(pomelo_material_symbols::font())
                .size(style::FONT_SMALL)
                .color(style::IconColor::Orange.color()),
        )
        .into(),
        Space::new().width(Length::Fixed(8.0)).into(),
        text(language.text(Key::AutoSleep))
            .size(style::LABEL_FONT)
            .color(style::label_for(theme))
            .into(),
    ])
    .align_y(Alignment::Center);

    let mut options_col = Column::new().spacing(4.0).width(Length::Fill);

    for opt in SleepTimeout::OPTIONS {
        let is_selected = opt == preferences.sleep_timeout;
        let opt_name = opt.name(language);

        let mut row_children: Vec<UI<'a>> = vec![
            text(opt_name)
                .size(style::DETAIL_FONT)
                .color(if is_selected {
                    style::accent()
                } else {
                    style::label_for(theme)
                })
                .into(),
            Space::new().width(Length::Fill).into(),
        ];

        if is_selected {
            row_children.push(
                text(Icon::CHECK.glyph())
                    .font(pomelo_material_symbols::font())
                    .size(style::NAV_GLYPH)
                    .color(style::accent())
                    .into(),
            );
        }

        let row_content = Row::with_children(row_children)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Alignment::Center);

        let opt_btn = button(
            container(row_content)
                .width(Length::Fill)
                .padding([10.0, 14.0]),
        )
        .width(Length::Fill)
        .padding(0)
        .style(move |_theme, status| {
            let bg = if is_selected {
                Some(Color { a: 0.15, ..style::accent() }.into())
            } else if matches!(status, button::Status::Pressed) {
                Some(press_wash(theme).into())
            } else {
                None
            };
            button::Style {
                background: bg,
                border: Border {
                    radius: 8.0.into(),
                    ..Border::default()
                },
                shadow: iced::Shadow::default(),
                snap: false,
                ..button::Style::default()
            }
        })
        .on_press(Message::SetSleepTimeout(opt));

        options_col = options_col.push(opt_btn);
    }

    let cancel_btn = pomelo_widgets::dialog_cancel_button(
        language.text(Key::Cancel),
        Message::CancelSleepTimeout,
        theme,
    );

    let dialog_content = Column::with_children(vec![
        header.into(),
        Space::new().height(Length::Fixed(12.0)).into(),
        separator(theme),
        Space::new().height(Length::Fixed(8.0)).into(),
        options_col.into(),
        Space::new().height(Length::Fixed(8.0)).into(),
        separator(theme),
        Space::new().height(Length::Fixed(12.0)).into(),
        cancel_btn,
    ])
    .width(Length::Fill);

    let dialog_box = pomelo_widgets::dialog_card(dialog_content, theme);
    pomelo_widgets::modal_dialog(dialog_box)
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
