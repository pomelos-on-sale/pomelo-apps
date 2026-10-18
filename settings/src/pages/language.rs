//! The language selection page: Chinese and English options with checkmark indicator.

use pomelo_material_symbols::Icon;
use pomelo_widgets::{Language, SystemPreferences};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::card::{back_bar, section_card, Card, Tile};
use crate::pages::common::{body, page, Page};
use crate::{Message, SettingsSection};

pub(crate) fn language_page<'a>(preferences: SystemPreferences) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let mut options_card = Card::new(theme);
    for opt in Language::ALL {
        let is_selected = opt == preferences.language;
        options_card = options_card.tile(Tile::select_option(
            opt.name(),
            is_selected,
            Message::SetLanguage(opt),
        ));
    }

    page(
        back_bar(theme),
        body(
            vec![
                section_card(
                    Icon::TRANSLATE,
                    SettingsSection::Language,
                    language.text(Key::Language),
                    theme,
                )
                .into(),
                options_card.into(),
            ],
            theme,
        ),
    )
}
