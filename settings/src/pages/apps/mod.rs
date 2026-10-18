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
use pomelo_widgets::{Item, ItemBadge, ItemGroup, OpenPage, Section, SystemPreferences};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::common::{back_bar, body, page, section_card, Page};
use crate::{Message, SettingsSection};

/// The Applications list page (Settings -> Apps).
pub(crate) fn apps_page<'a>(
    preferences: SystemPreferences,
    _schedule_configured: bool,
) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let item = Item::nav(
        language.text(Key::DailySchedule),
        None::<String>,
        OpenPage(Message::Open(SettingsSection::AppDailySchedule)),
    )
    .with_badge(ItemBadge::new(
        Icon::CALENDAR_MONTH,
        crate::style::IconColor::for_section(SettingsSection::AppDailySchedule).color(),
    ));

    let apps_section = Section::group(ItemGroup::new().push(item), theme).view();

    page(
        back_bar(theme),
        body(
            vec![
                section_card(
                    Icon::APPS,
                    SettingsSection::Apps,
                    language.text(Key::Applications),
                    theme,
                ),
                apps_section,
            ],
            theme,
        ),
    )
}
