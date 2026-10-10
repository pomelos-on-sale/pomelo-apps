//! The storage page: a usage bar and LittleFS details.

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::card::Header;
use crate::pages::common::{body, detail_card, page, usage_bar, UI};
use crate::style;
use crate::SettingsSection;
use pomelo_material_symbols::Icon;
use pomelo_widgets::SystemPreferences;

pub(crate) fn storage_page<'a>(preferences: SystemPreferences) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let bar = usage_bar(
        language.text(Key::LittlefsPartition),
        format!("99.9% {}", language.text(Key::Free)),
        style::green(),
        4.5,
        style::storage_bar(),
        theme,
    );

    let details = vec![
        (language.text(Key::MountPoint), "/internal".to_string()),
        (
            language.text(Key::Filesystem),
            "LittleFS (power-fail safe)".to_string(),
        ),
        (
            language.text(Key::Total),
            "11,534,336 bytes (11.0 MB)".to_string(),
        ),
        (language.text(Key::Used), "8,192 bytes (0.07%)".to_string()),
        (
            language.text(Key::Free),
            "11,526,144 bytes (99.93%)".to_string(),
        ),
        (
            language.text(Key::Files),
            "1 file (welcome.txt)".to_string(),
        ),
        (
            language.text(Key::WearLevelling),
            "active (good)".to_string(),
        ),
    ];

    page(
        Header::section(
            Icon::STORAGE,
            SettingsSection::Storage,
            language.text(Key::Storage),
        )
        .view(theme),
        body(vec![bar, detail_card(details, theme)]),
    )
}
