//! Screen-off timeout selection sub-page (熄屏时间设置页面).

use pomelo_widgets::{Item, ItemGroup, ScreenOffTimeout, Section, SystemPreferences};

use crate::pages::common::{back_bar, body, page, Page};
use crate::Message;

/// The Screen-off timeout selection page (Settings -> Theme -> Screen Off).
pub(crate) fn screen_off_timeout_page<'a>(preferences: SystemPreferences) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let mut group = ItemGroup::new();
    for opt in ScreenOffTimeout::OPTIONS {
        let is_selected = opt == preferences.screen_off_timeout;
        group = group.push(Item::option(
            opt.name(language),
            is_selected,
            Message::SetScreenOffTimeout(opt),
        ));
    }

    page(
        back_bar(theme),
        body(vec![Section::group(group, theme).view()], theme),
    )
}
