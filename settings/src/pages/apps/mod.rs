//! Applications management and per-app configuration pages.

pub mod daily_schedule;

pub use daily_schedule::{
    sanitize_calendar_url, ColorRule, ColorRuleModalState, DailyScheduleConfig,
    DailyScheduleSettingsState, PresetColor,
};
pub(crate) use daily_schedule::{
    daily_schedule_colors_page, daily_schedule_page, daily_schedule_url_page,
};

use pomelo_material_symbols::Icon;
use pomelo_widgets::SystemPreferences;

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::card::{back_bar, section_card, Card, Tile};
use crate::pages::common::{body, page, UI};
use crate::SettingsSection;

/// The Applications list page (Settings -> Apps).
pub(crate) fn apps_page<'a>(
    preferences: SystemPreferences,
    _schedule_configured: bool,
) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let apps_card = Card::new(theme).tile(Tile::section(
        Icon::CALENDAR_MONTH,
        SettingsSection::AppDailySchedule,
        language.text(Key::DailySchedule),
    ));

    page(
        back_bar(theme),
        body(
            vec![
                section_card(
                    Icon::APPS,
                    SettingsSection::Apps,
                    language.text(Key::Applications),
                    theme,
                )
                .into(),
                apps_card.into(),
            ],
            theme,
        ),
    )
}
