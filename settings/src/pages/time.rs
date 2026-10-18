//! The date and time page: 24-hour switch and clock details.

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::common::{back_bar, body, detail_card, page, section_card, Page};
use crate::{Message, SettingsSection};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{Item, ItemGroup, Section, SystemPreferences};

pub(crate) fn time_page<'a>(preferences: SystemPreferences, is_24h: bool) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let details = vec![
        (
            language.text(Key::SystemTime),
            if is_24h {
                "10:24".to_string()
            } else {
                "10:24 AM".to_string()
            },
        ),
        (
            language.text(Key::TimeZone),
            "CST (UTC+8, Beijing)".to_string(),
        ),
        (language.text(Key::NtpSync), "on (pool.ntp.org)".to_string()),
        (language.text(Key::Rtc), "ESP32-S3 internal RTC".to_string()),
        (
            language.text(Key::SyncStatus),
            "calibrated (offset < 5ms)".to_string(),
        ),
    ];

    let switch_section = Section::group(
        ItemGroup::new().push(Item::switch(
            language.text(Key::TwentyFourHour),
            is_24h,
            Message::Toggle24Hour,
        )),
        theme,
    )
    .view();

    page(
        back_bar(theme),
        body(
            vec![
                section_card(
                    Icon::SCHEDULE,
                    SettingsSection::Time,
                    language.text(Key::Time),
                    theme,
                ),
                switch_section,
                detail_card(details, theme),
            ],
            theme,
        ),
    )
}
