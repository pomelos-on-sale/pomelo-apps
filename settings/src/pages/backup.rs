//! The backup and restore page: backup AppData to SD card and restore from SD card.

use iced::widget::{button, container, stack, text, Column, Row, Space};
use iced::{Alignment, Length};

use pomelo_hal::types::SdCardStatus;
use pomelo_material_symbols::Icon;
use pomelo_widgets::{SystemPreferences, ThemeMode};

use crate::i18n::{Key, Language, LanguageExt as _};
use crate::pages::card::Header;
use crate::pages::common::{
    body, card_surface, notice_card, page, row_style, separator, UI,
};
use crate::style;
use crate::{Message, SettingsSection};

/// Modal dialog asking user to confirm or cancel AppData backup.
fn backup_confirm_modal<'a>(language: Language, theme: ThemeMode) -> UI<'a> {
    crate::pages::common::confirm_dialog(
        language.text(Key::BackupAppData),
        language.text(Key::BackupWarning),
        style::notice_for(theme),
        language.text(Key::Cancel),
        Message::CancelBackupAppData,
        language.text(Key::ConfirmBackup),
        Message::ConfirmBackupAppData,
        false,
        theme,
    )
}

/// Modal dialog asking user to confirm or cancel AppData restore.
fn restore_confirm_modal<'a>(language: Language, theme: ThemeMode) -> UI<'a> {
    crate::pages::common::confirm_dialog(
        language.text(Key::RestoreAppData),
        language.text(Key::RestoreWarning),
        style::red(),
        language.text(Key::Cancel),
        Message::CancelRestoreAppData,
        language.text(Key::ConfirmRestore),
        Message::ConfirmRestoreAppData,
        true,
        theme,
    )
}

/// Generic notice modal with a single OK button to dismiss.
fn notice_modal<'a>(
    title_str: &str,
    msg_str: &str,
    language: Language,
    theme: ThemeMode,
) -> UI<'a> {
    crate::pages::common::notice_dialog(
        title_str,
        msg_str,
        language.text(Key::Ok),
        Message::DismissBackupNotice,
        theme,
    )
}

fn custom_action_row<'a>(
    label: &'static str,
    status: Option<&'static str>,
    message: Option<Message>,
    theme: ThemeMode,
) -> UI<'a> {
    let text_color = if message.is_some() {
        style::label_for(theme)
    } else {
        style::notice_for(theme)
    };

    let mut right_items: Vec<UI<'a>> = Vec::new();
    if let Some(s) = status {
        right_items.push(
            text(s)
                .size(style::DETAIL_FONT)
                .color(style::notice_for(theme))
                .into(),
        );
    }
    if message.is_some() {
        right_items.push(
            text(">")
                .size(style::CHEVRON_FONT)
                .color(style::chevron_for(theme))
                .into(),
        );
    }

    let line = Row::with_children(vec![
        text(label).size(style::DETAIL_FONT).color(text_color).into(),
        Space::new().width(Length::Fill).into(),
        Row::with_children(right_items).spacing(8.0).align_y(iced::Alignment::Center).into(),
    ])
    .width(Length::Fill)
    .align_y(iced::Alignment::Center);

    let contents = container(line)
        .width(Length::Fill)
        .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H]);

    match message {
        Some(msg) => button(contents)
            .width(Length::Fill)
            .padding(0)
            .style(move |_theme, btn_status| row_style(theme, btn_status))
            .on_press(msg)
            .into(),
        None => contents.into(),
    }
}

/// Status card showing SD card connection state.
fn sd_status_card<'a>(label: &'static str, status: String, theme: ThemeMode) -> UI<'a> {
    let line = Row::with_children(vec![
        text(label)
            .size(style::DETAIL_FONT)
            .color(style::label_for(theme))
            .into(),
        Space::new().width(Length::Fill).into(),
        text(status)
            .size(style::DETAIL_FONT)
            .color(style::notice_for(theme))
            .into(),
    ])
    .width(Length::Fill)
    .align_y(Alignment::Center);

    card_surface(
        container(line)
            .width(Length::Fill)
            .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H]),
        theme,
    )
}

/// Card offering AppData Backup and Restore options.
fn backup_restore_card<'a>(
    language: Language,
    theme: ThemeMode,
    sd_mounted: bool,
    has_backup: bool,
    is_busy: bool,
) -> UI<'a> {
    let backup_msg = if sd_mounted && !is_busy {
        Some(Message::PromptBackupAppData)
    } else {
        None
    };

    let restore_msg = if sd_mounted && has_backup && !is_busy {
        Some(Message::PromptRestoreAppData)
    } else {
        None
    };

    let backup_row = custom_action_row(
        language.text(Key::BackupAppData),
        if !sd_mounted {
            Some(language.text(Key::NeedsSdCard))
        } else {
            None
        },
        backup_msg,
        theme,
    );

    let restore_row = custom_action_row(
        language.text(Key::RestoreAppData),
        if !sd_mounted {
            Some(language.text(Key::NeedsSdCard))
        } else if !has_backup {
            Some(language.text(Key::NoBackupFound))
        } else {
            None
        },
        restore_msg,
        theme,
    );

    let header_line = Row::with_children(vec![
        text(language.text(Key::DataBackup))
            .size(style::DETAIL_FONT)
            .color(style::label_for(theme))
            .into(),
        Space::new().width(Length::Fill).into(),
    ])
    .width(Length::Fill);

    let header_box = container(header_line)
        .width(Length::Fill)
        .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H]);

    card_surface(
        Column::with_children(vec![
            header_box.into(),
            separator(theme),
            backup_row,
            separator(theme),
            restore_row,
        ])
        .width(Length::Fill),
        theme,
    )
}

/// The dedicated Backup & Restore page.
pub(crate) fn backup_page<'a>(
    preferences: SystemPreferences,
    sd_status: SdCardStatus,
    has_backup: bool,
    is_backing_up: bool,
    is_restoring: bool,
    show_backup_confirm: bool,
    show_restore_confirm: bool,
    backup_notice: Option<&(String, String)>,
) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let is_busy = is_backing_up || is_restoring;
    let sd_mounted = sd_status == SdCardStatus::Mounted;

    let mut elements: Vec<UI<'a>> = Vec::new();

    // 1. SD card status card
    let status_str = match sd_status {
        SdCardStatus::Mounted => language.text(Key::Inserted),
        SdCardStatus::NotInserted => language.text(Key::NotInserted),
        SdCardStatus::Unformatted => language.text(Key::Unformatted),
        SdCardStatus::Error => language.text(Key::ConnectFailed),
    };
    elements.push(sd_status_card(
        language.text(Key::SdCard),
        status_str.to_string(),
        theme,
    ));

    // 2. Action card (Backup / Restore)
    elements.push(backup_restore_card(
        language,
        theme,
        sd_mounted,
        has_backup,
        is_busy,
    ));

    // 3. Busy notices
    if is_backing_up {
        elements.push(notice_card(language.text(Key::BackingUp), theme));
    }
    if is_restoring {
        elements.push(notice_card(language.text(Key::Restoring), theme));
    }

    let pg = page(
        Header::section(
            Icon::SETTINGS_BACKUP_RESTORE,
            SettingsSection::Backup,
            language.text(Key::DataBackup),
        )
        .view(theme),
        body(elements),
    );

    if show_backup_confirm {
        let modal = backup_confirm_modal(language, theme);
        stack![pg, modal].into()
    } else if show_restore_confirm {
        let modal = restore_confirm_modal(language, theme);
        stack![pg, modal].into()
    } else if let Some((title, msg)) = backup_notice {
        let modal = notice_modal(title, msg, language, theme);
        stack![pg, modal].into()
    } else {
        pg
    }
}
