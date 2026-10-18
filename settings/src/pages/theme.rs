//! The theme page: dark/light mode switch, font size, colour palette and display details.

use iced::widget::{container, text, Column, Row, Space};
use iced::{Alignment, Border, Length};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{ScreenOffTimeout, SleepTimeout, SystemPreferences};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::card::{back_bar, section_card, Card, Tile};
use crate::pages::common::{body, card_surface, detail_card, page, Group, Page, UI};
use crate::style;
use crate::{Message, SettingsSection};

/// Encapsulated state for the Display & Theme settings page.
#[derive(Debug, Clone, Default)]
pub struct ThemePageState {
    pub show_screen_off_timeout_dialog: bool,
    pub show_sleep_timeout_dialog: bool,
}

impl ThemePageState {
    pub fn prompt_screen_off_timeout(&mut self) {
        self.show_screen_off_timeout_dialog = true;
    }

    pub fn cancel_screen_off_timeout(&mut self) {
        self.show_screen_off_timeout_dialog = false;
    }

    pub fn prompt_sleep_timeout(&mut self) {
        self.show_sleep_timeout_dialog = true;
    }

    pub fn cancel_sleep_timeout(&mut self) {
        self.show_sleep_timeout_dialog = false;
    }

    pub fn has_active_modal(&self) -> bool {
        self.show_screen_off_timeout_dialog || self.show_sleep_timeout_dialog
    }

    pub fn dismiss_modal(&mut self) {
        self.show_screen_off_timeout_dialog = false;
        self.show_sleep_timeout_dialog = false;
    }
}

pub(crate) fn theme_page<'a>(
    preferences: SystemPreferences,
    _state: &ThemePageState,
) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let value_font = style::VALUE_FONT;
    let value_color = style::muted_for(theme);

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
            .secondary(
                text(preferences.theme.name(language))
                    .size(value_font)
                    .color(value_color),
            ),
        )
        .tile(
            Tile::action(
                Icon::FORMAT_SIZE,
                style::IconColor::for_section(SettingsSection::SystemInfo),
                language.text(Key::FontSize),
                Message::CycleFontTier,
            )
            .secondary(
                text(preferences.font_tier.name(language))
                    .size(value_font)
                    .color(value_color),
            ),
        )
        .tile(
            Tile::section(
                Icon::SCREEN_LOCK_PORTRAIT,
                SettingsSection::ScreenOffTimeout,
                language.text(Key::ScreenOff),
            )
            .secondary(
                text(preferences.screen_off_timeout.name(language))
                    .size(value_font)
                    .color(value_color),
            ),
        )
        .tile(
            Tile::section(
                Icon::BEDTIME,
                SettingsSection::SleepTimeout,
                language.text(Key::Sleep),
            )
            .secondary(
                text(preferences.sleep_timeout.name(language))
                    .size(value_font)
                    .color(value_color),
            ),
        )
        .tile(
            Tile::toggle(
                Icon::USB,
                style::IconColor::Green,
                language.text(Key::StayAwakeOnUsb),
                preferences.stay_awake_on_usb,
                Message::ToggleStayAwakeOnUsb,
                theme,
            ),
        );

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

    let palette = Group::from_surface(card_surface(
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
    ));

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
                )
                .into(),
                controls.into(),
                palette.into(),
                detail_card(details, theme).into(),
            ],
            theme,
        ),
    )
}

/// The Screen-off timeout selection page (Settings -> Theme -> Screen Off).
pub(crate) fn screen_off_timeout_page<'a>(preferences: SystemPreferences) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let mut options_card = Card::new(theme);
    for opt in ScreenOffTimeout::OPTIONS {
        let is_selected = opt == preferences.screen_off_timeout;
        options_card = options_card.tile(Tile::select_option(
            opt.name(language),
            is_selected,
            Message::SetScreenOffTimeout(opt),
        ));
    }

    page(
        back_bar(theme),
        body(vec![options_card.into()], theme),
    )
}

/// The Auto-sleep timeout selection page (Settings -> Theme -> Auto-Sleep).
pub(crate) fn sleep_timeout_page<'a>(preferences: SystemPreferences) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let mut options_card = Card::new(theme);
    for opt in SleepTimeout::OPTIONS {
        let is_selected = opt == preferences.sleep_timeout;
        options_card = options_card.tile(Tile::select_option(
            opt.name(language),
            is_selected,
            Message::SetSleepTimeout(opt),
        ));
    }

    page(
        back_bar(theme),
        body(vec![options_card.into()], theme),
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
