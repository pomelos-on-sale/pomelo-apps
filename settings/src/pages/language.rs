//! The language selection page: Chinese and English options with checkmark indicator.

use pomelo_material_symbols::Icon;
use pomelo_widgets::{Item, ItemGroup, Language, Section, SystemPreferences};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::common::{back_bar, body, page, section_card, Page};
use crate::{Message, SettingsSection};

pub(crate) fn language_page<'a>(preferences: SystemPreferences) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let mut group = ItemGroup::new();
    for opt in Language::ALL {
        let is_selected = opt == preferences.language;
        group = group.push(Item::option(
            opt.name(),
            is_selected,
            Message::SetLanguage(opt),
        ));
    }
    let options_section = Section::group(group, theme).view();

    page(
        back_bar(theme),
        body(
            vec![
                section_card(
                    Icon::TRANSLATE,
                    SettingsSection::Language,
                    language.text(Key::Language),
                    theme,
                ),
                options_section,
            ],
            theme,
        ),
    )
}
