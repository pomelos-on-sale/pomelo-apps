//! Selection ActionSheets (项选择页面) for Alarm Clock.
//!
//! Provides dedicated modal selection ActionSheets for:
//! - "重复" (Repeat schedule)
//! - "标签" (Label presets)
//! - "铃声" (Ringtone melody)
//! - "稍后提醒时长" (Snooze duration)
//!
//! All selection items use [`Item::option`] inside an [`ItemGroup`] with standard checkmark feedback (`Icon::CHECK`).

use iced::widget::{column, container, Space};
use iced::{Alignment, Element, Length};
use pomelo_widgets::{ActionSheet, ActionStyle, Item, ItemGroup, Language, Section, SystemPreferences};

use crate::model::{sound_name, AlarmItem, ALARM_SOUND_NAMES, DAY_NAMES, PRESET_LABELS, SNOOZE_PRESETS};
use crate::Message;

/// Renders the Repeat schedule selection ActionSheet.
pub fn view_repeat_sheet<'a>(
    draft: &'a AlarmItem,
    preferences: &SystemPreferences,
) -> Element<'a, Message> {
    let theme = preferences.theme;
    let lang = preferences.language;

    let title_str = match lang {
        Language::Chinese => "重复",
        Language::English => "Repeat",
    };
    let cancel_str = match lang {
        Language::Chinese => "取消",
        Language::English => "Cancel",
    };
    let confirm_str = match lang {
        Language::Chinese => "确定",
        Language::English => "Done",
    };

    let mut group = ItemGroup::new();
    for idx in 0..7 {
        let label = match lang {
            Language::Chinese => DAY_NAMES[idx].0,
            Language::English => DAY_NAMES[idx].1,
        };
        let is_selected = draft.repeat.is_day_selected(idx);
        group = group.push(Item::option(label, is_selected, Message::ToggleRepeatDay(idx)));
    }

    let body = container(
        column![
            Space::new().height(Length::Fixed(8.0)),
            Section::group(group, theme).view(),
            Space::new().height(Length::Fixed(40.0)),
        ]
        .width(Length::Fixed(440.0))
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .center_x(Length::Fill);

    ActionSheet::new(theme)
        .title(title_str)
        .cancel(cancel_str, Message::CloseSubSheet)
        .action(confirm_str, Message::CloseSubSheet, ActionStyle::Primary)
        .custom_body(body)
        .view()
}

/// Renders the Label selection ActionSheet.
pub fn view_label_sheet<'a>(
    draft: &'a AlarmItem,
    preferences: &SystemPreferences,
) -> Element<'a, Message> {
    let theme = preferences.theme;
    let lang = preferences.language;

    let title_str = match lang {
        Language::Chinese => "标签",
        Language::English => "Label",
    };
    let cancel_str = match lang {
        Language::Chinese => "取消",
        Language::English => "Cancel",
    };
    let confirm_str = match lang {
        Language::Chinese => "确定",
        Language::English => "Done",
    };

    let mut group = ItemGroup::new();
    for &lbl in &PRESET_LABELS {
        let is_selected = draft.label == lbl;
        group = group.push(Item::option(lbl, is_selected, Message::SelectLabel(lbl.to_string())));
    }

    let body = container(
        column![
            Space::new().height(Length::Fixed(8.0)),
            Section::group(group, theme).view(),
            Space::new().height(Length::Fixed(40.0)),
        ]
        .width(Length::Fixed(440.0))
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .center_x(Length::Fill);

    ActionSheet::new(theme)
        .title(title_str)
        .cancel(cancel_str, Message::CloseSubSheet)
        .action(confirm_str, Message::CloseSubSheet, ActionStyle::Primary)
        .custom_body(body)
        .view()
}

/// Renders the Ringtone Sound selection ActionSheet.
pub fn view_sound_sheet<'a>(
    draft: &'a AlarmItem,
    preferences: &SystemPreferences,
) -> Element<'a, Message> {
    let theme = preferences.theme;
    let lang = preferences.language;

    let title_str = match lang {
        Language::Chinese => "铃声",
        Language::English => "Sound",
    };
    let cancel_str = match lang {
        Language::Chinese => "取消",
        Language::English => "Cancel",
    };
    let confirm_str = match lang {
        Language::Chinese => "确定",
        Language::English => "Done",
    };

    let mut group = ItemGroup::new();
    for idx in 0..ALARM_SOUND_NAMES.len() {
        let name = sound_name(idx as u8, lang);
        let is_selected = draft.sound_id == idx as u8;
        group = group.push(Item::option(name, is_selected, Message::SelectSound(idx as u8)));
    }

    let body = container(
        column![
            Space::new().height(Length::Fixed(8.0)),
            Section::group(group, theme).view(),
            Space::new().height(Length::Fixed(40.0)),
        ]
        .width(Length::Fixed(440.0))
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .center_x(Length::Fill);

    ActionSheet::new(theme)
        .title(title_str)
        .cancel(cancel_str, Message::CloseSubSheet)
        .action(confirm_str, Message::CloseSubSheet, ActionStyle::Primary)
        .custom_body(body)
        .view()
}

/// Renders the Snooze Duration selection ActionSheet.
pub fn view_snooze_duration_sheet<'a>(
    draft: &'a AlarmItem,
    preferences: &SystemPreferences,
) -> Element<'a, Message> {
    let theme = preferences.theme;
    let lang = preferences.language;

    let title_str = match lang {
        Language::Chinese => "稍后提醒时长",
        Language::English => "Snooze Duration",
    };
    let cancel_str = match lang {
        Language::Chinese => "取消",
        Language::English => "Cancel",
    };
    let confirm_str = match lang {
        Language::Chinese => "确定",
        Language::English => "Done",
    };

    let mut group = ItemGroup::new();
    for &min in &SNOOZE_PRESETS {
        let label = match lang {
            Language::Chinese => format!("{} 分钟", min),
            Language::English => format!("{} min", min),
        };
        let is_selected = draft.snooze_minutes == min;
        group = group.push(Item::option(label, is_selected, Message::SelectSnoozeMinutes(min)));
    }

    let body = container(
        column![
            Space::new().height(Length::Fixed(8.0)),
            Section::group(group, theme).view(),
            Space::new().height(Length::Fixed(40.0)),
        ]
        .width(Length::Fixed(440.0))
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .center_x(Length::Fill);

    ActionSheet::new(theme)
        .title(title_str)
        .cancel(cancel_str, Message::CloseSubSheet)
        .action(confirm_str, Message::CloseSubSheet, ActionStyle::Primary)
        .custom_body(body)
        .view()
}
