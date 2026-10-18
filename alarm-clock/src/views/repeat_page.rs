//! Repeat schedule selection subpage (重复页面).
//!
//! Reuses [`Section`], [`ItemGroup`] and [`SubPage`] from `pomelo_widgets` to display the 7 days of the week
//! (Sunday through Saturday). Tapping any day toggles its selection state with
//! an iOS-style checkmark. Defaults to none selected ("永不" / Never).

use iced::Element;
use pomelo_widgets::{SystemPreferences};

use crate::model::AlarmItem;
use crate::views::selection_sheets::view_repeat_sheet;
use crate::Message;

/// Renders the Repeat schedule subpage.
pub fn view_repeat_page<'a>(
    draft: &'a AlarmItem,
    preferences: &SystemPreferences,
) -> Element<'a, Message> {
    view_repeat_sheet(draft, preferences)
}
