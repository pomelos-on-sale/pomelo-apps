//! The main settings list: four sections of items.

use pomelo_material_symbols::Icon;
use pomelo_widgets::{Item, ItemBadge, ItemGroup, OpenPage, Section, SystemPreferences};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::common::{body, page, title, Page};
use crate::style;
use crate::{Message, SettingsSection};

fn section_item(
    icon: Icon,
    section: SettingsSection,
    label: &'static str,
    value: Option<String>,
) -> Item<Message> {
    let color = style::IconColor::for_section(section).color();
    Item::nav(label, value, OpenPage(Message::Open(section)))
        .with_badge(ItemBadge::new(icon, color))
}

/// The main list, in `language`.
pub(crate) fn main_page<'a>(preferences: SystemPreferences, connected: Option<&str>) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let wifi = connected
        .unwrap_or_else(|| language.text(Key::NotConnected))
        .to_string();

    let connectivity = Section::group(
        ItemGroup::new().push(section_item(
            Icon::WIFI,
            SettingsSection::Wifi,
            language.text(Key::Wifi),
            Some(wifi),
        )),
        theme,
    )
    .view();

    let device = Section::group(
        ItemGroup::new()
            .push(section_item(
                Icon::MEMORY,
                SettingsSection::Memory,
                language.text(Key::Memory),
                None,
            ))
            .push(section_item(
                Icon::STORAGE,
                SettingsSection::Storage,
                language.text(Key::Storage),
                None,
            ))
            .push(section_item(
                Icon::BATTERY_FULL,
                SettingsSection::Battery,
                language.text(Key::BatteryRow),
                None,
            )),
        theme,
    )
    .view();

    let system = Section::group(
        ItemGroup::new()
            .push(section_item(
                Icon::SETTINGS_BACKUP_RESTORE,
                SettingsSection::Backup,
                language.text(Key::DataBackup),
                None,
            ))
            .push(section_item(
                Icon::INFO,
                SettingsSection::SystemInfo,
                language.text(Key::System),
                None,
            ))
            .push(section_item(
                Icon::PALETTE,
                SettingsSection::Theme,
                language.text(Key::Theme),
                None,
            ))
            .push(section_item(
                Icon::SCHEDULE,
                SettingsSection::Time,
                language.text(Key::Time),
                None,
            ))
            .push(section_item(
                Icon::TRANSLATE,
                SettingsSection::Language,
                language.text(Key::Language),
                Some(preferences.language.name().to_string()),
            )),
        theme,
    )
    .view();

    let apps = Section::group(
        ItemGroup::new().push(section_item(
            Icon::APPS,
            SettingsSection::Apps,
            language.text(Key::Applications),
            None,
        )),
        theme,
    )
    .view();

    page(
        title(language.text(Key::Settings), theme),
        body(vec![connectivity, device, apps, system], theme),
    )
}
