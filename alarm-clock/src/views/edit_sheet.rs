//! Add / Edit Alarm ActionSheet modal view (添加/编辑闹钟页面).
//!
//! Conforms to Pomelo OS design language:
//! - Hosts inside standard [`ActionSheet`] with centered title "添加闹钟" (or "编辑闹钟")
//! - Cancel ("✕") and Save ("✓") capsule navigation buttons
//! - Two touch-scrollable digital number wheel rollers for hours and minutes
//! - Grouped card with navigation row for "重复", "标签", "铃声", "稍后提醒", "稍后提醒时长"

use iced::widget::{column, container, Space};
use iced::{Alignment, Element, Length};
use pomelo_widgets::{Item, ItemGroup, Language, Section, SubPage, SystemPreferences};

use crate::model::{sound_name, AlarmItem};
use crate::views::wheel_picker::render_wheel_pickers;
use crate::{AlarmRoute, Message};

/// Renders the Add or Edit Alarm ActionSheet view.
pub fn view_edit_sheet<'a>(
    is_new: bool,
    draft: &'a AlarmItem,
    preferences: &SystemPreferences,
) -> Element<'a, Message> {
    let theme = preferences.theme;
    let lang = preferences.language;

    let title_str = match (is_new, lang) {
        (true, Language::Chinese) => "添加闹钟",
        (true, Language::English) => "Add Alarm",
        (false, Language::Chinese) => "编辑闹钟",
        (false, Language::English) => "Edit Alarm",
    };

    // 1. Digital number wheel rollers
    let pickers = render_wheel_pickers(draft.hour, draft.minute);

    // 2. Settings Group (ItemGroup inside visual Section)
    let repeat_label = match lang {
        Language::Chinese => "重复",
        Language::English => "Repeat",
    };
    let repeat_summary = draft.repeat.display_summary(lang);

    let tag_label = match lang {
        Language::Chinese => "标签",
        Language::English => "Label",
    };
    let tag_value: std::borrow::Cow<'static, str> = if draft.label.trim().is_empty() {
        match lang {
            Language::Chinese => "闹钟".into(),
            Language::English => "Alarm".into(),
        }
    } else {
        draft.label.clone().into()
    };

    let sound_label = match lang {
        Language::Chinese => "铃声",
        Language::English => "Sound",
    };
    let sound_val = sound_name(draft.sound_id, lang);

    let snooze_label = match lang {
        Language::Chinese => "稍后提醒",
        Language::English => "Snooze",
    };

    let snooze_dur_label = match lang {
        Language::Chinese => "稍后提醒时长",
        Language::English => "Snooze Duration",
    };
    let snooze_dur_val = match lang {
        Language::Chinese => format!("{} 分钟", draft.snooze_minutes),
        Language::English => format!("{} min", draft.snooze_minutes),
    };

    let mut options_group = ItemGroup::new()
        .push(Item::nav(repeat_label, Some(repeat_summary), AlarmRoute::Repeat))
        .push(Item::nav(tag_label, Some(tag_value), AlarmRoute::Label))
        .push(Item::nav(sound_label, Some(sound_val), AlarmRoute::Sound))
        .push(Item::switch(snooze_label, draft.snooze_enabled, Message::ToggleSnooze));

    if draft.snooze_enabled {
        options_group = options_group.push(Item::nav(
            snooze_dur_label,
            Some(snooze_dur_val),
            AlarmRoute::SnoozeDuration,
        ));
    }

    let mut content_col = column![
        pickers,
        Space::new().height(Length::Fixed(6.0)),
        Section::group(options_group, theme).view(),
    ]
    .width(Length::Fixed(440.0))
    .align_x(Alignment::Center)
    .spacing(8.0);

    content_col = content_col.push(Space::new().height(Length::Fixed(40.0)));

    let body = container(content_col)
        .width(Length::Fill)
        .center_x(Length::Fill);

    SubPage::modal(title_str, theme)
        .cancel(Message::CancelEdit)
        .confirm(Message::SaveEdit)
        .body(body)
        .view()
}
