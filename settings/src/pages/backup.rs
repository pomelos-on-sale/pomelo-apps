//! The backup and restore page: backup AppData to SD card and restore from SD card.

use iced::widget::{button, container, text, Column, Row, Space};
use iced::{Alignment, Length};

use pomelo_hal::types::SdCardStatus;
use pomelo_material_symbols::Icon;
use pomelo_widgets::{SystemPreferences, ThemeMode};

use crate::i18n::{Key, Language, LanguageExt as _};
use crate::pages::card::{back_bar, section_card};
use crate::pages::common::{
    body, card_surface, notice_card, page_with_sheet, row_style, separator, ActionSheet, Group,
    PageItem, UI,
};
use crate::style;
use crate::{Message, SettingsSection};

/// Modal Action Sheet asking user to confirm or cancel AppData backup.
fn backup_confirm_sheet(language: Language, theme: ThemeMode) -> ActionSheet<'static, Message> {
    ActionSheet::confirm(
        language.text(Key::BackupWarning),
        language.text(Key::ConfirmBackup),
        Message::ConfirmBackupAppData,
        language.text(Key::Cancel),
        Message::CancelBackupAppData,
        theme,
    )
}

/// Modal Action Sheet asking user to confirm or cancel AppData restore.
fn restore_confirm_sheet(language: Language, theme: ThemeMode) -> ActionSheet<'static, Message> {
    ActionSheet::destructive_confirm(
        language.text(Key::RestoreWarning),
        language.text(Key::Continue),
        Message::ConfirmRestoreAppData,
        language.text(Key::Cancel),
        Message::CancelRestoreAppData,
        theme,
    )
}

/// Generic notice Action Sheet with a single OK button to dismiss.
fn notice_sheet(
    title_str: &str,
    msg_str: &str,
    language: Language,
    theme: ThemeMode,
) -> ActionSheet<'static, Message> {
    ActionSheet::notice(
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
fn sd_status_card<'a>(label: &'static str, status: String, theme: ThemeMode) -> Group<'a> {
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

    Group::from_surface(card_surface(
        container(line)
            .width(Length::Fill)
            .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H]),
        theme,
    ))
}

/// Card offering AppData Backup and Restore options.
fn backup_restore_card<'a>(
    language: Language,
    theme: ThemeMode,
    sd_mounted: bool,
    has_backup: bool,
    is_busy: bool,
) -> Group<'a> {
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

    Group::from_surface(card_surface(
        Column::with_children(vec![
            header_box.into(),
            separator(theme),
            backup_row,
            separator(theme),
            restore_row,
        ])
        .width(Length::Fill),
        theme,
    ))
}

/// Encapsulated state for the Backup & Restore settings page.
#[derive(Debug, Clone, Default)]
pub struct BackupState {
    pub has_backup: bool,
    pub is_backing_up: bool,
    pub is_restoring: bool,
    pub show_backup_confirm: bool,
    pub show_restore_confirm: bool,
    pub backup_notice: Option<(String, String)>,
}

impl BackupState {
    pub fn refresh(&mut self, board: &pomelo_hal::Board) {
        self.has_backup = board.storage().has_backup();
    }

    pub fn prompt_backup(&mut self) {
        self.show_backup_confirm = true;
    }

    pub fn cancel_backup(&mut self) {
        self.show_backup_confirm = false;
    }

    pub fn confirm_backup(&mut self, board: &pomelo_hal::Board, lang: Language) {
        self.show_backup_confirm = false;
        self.is_backing_up = true;
        let res = board.storage().backup_app_data();
        self.is_backing_up = false;
        self.refresh(board);
        match res {
            Ok(stats) => {
                let msg = format!(
                    "{} ({} files, {:.1} KB)",
                    lang.text(Key::BackupSuccess),
                    stats.files_count,
                    stats.bytes_count as f64 / 1024.0
                );
                self.backup_notice = Some((lang.text(Key::BackupSuccess).to_string(), msg));
            }
            Err(e) => {
                let msg = format!("{}: {}", lang.text(Key::BackupFailed), e);
                self.backup_notice = Some((lang.text(Key::BackupFailed).to_string(), msg));
            }
        }
    }

    pub fn prompt_restore(&mut self) {
        self.show_restore_confirm = true;
    }

    pub fn cancel_restore(&mut self) {
        self.show_restore_confirm = false;
    }

    pub fn confirm_restore(&mut self, board: &pomelo_hal::Board, lang: Language) {
        self.show_restore_confirm = false;
        self.is_restoring = true;
        let res = board.storage().restore_app_data();
        self.is_restoring = false;
        self.refresh(board);
        match res {
            Ok(_) => {
                self.backup_notice = Some((
                    lang.text(Key::RestoreSuccess).to_string(),
                    lang.text(Key::DeviceRebooting).to_string(),
                ));
                let _ = board.reboot();
            }
            Err(e) => {
                let msg = format!("{}: {}", lang.text(Key::RestoreFailed), e);
                self.backup_notice = Some((lang.text(Key::RestoreFailed).to_string(), msg));
            }
        }
    }

    pub fn dismiss_notice(&mut self) {
        self.backup_notice = None;
    }

    pub fn has_active_modal(&self) -> bool {
        self.show_backup_confirm || self.show_restore_confirm || self.backup_notice.is_some()
    }

    pub fn dismiss_modal(&mut self) {
        self.show_backup_confirm = false;
        self.show_restore_confirm = false;
        self.backup_notice = None;
    }
}

/// The dedicated Backup & Restore page.
pub(crate) fn backup_page<'a>(
    preferences: SystemPreferences,
    sd_status: SdCardStatus,
    state: &BackupState,
) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let has_backup = state.has_backup;
    let is_backing_up = state.is_backing_up;
    let is_restoring = state.is_restoring;
    let show_backup_confirm = state.show_backup_confirm;
    let show_restore_confirm = state.show_restore_confirm;
    let backup_notice = state.backup_notice.as_ref();

    let is_busy = is_backing_up || is_restoring;
    let sd_mounted = sd_status == SdCardStatus::Mounted;

    let mut elements: Vec<Group<'a>> = Vec::new();

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

    let sheet = if show_backup_confirm {
        Some(backup_confirm_sheet(language, theme))
    } else if show_restore_confirm {
        Some(restore_confirm_sheet(language, theme))
    } else {
        backup_notice.map(|(title, msg)| notice_sheet(title, msg, language, theme))
    };

    let mut body_elements: Vec<PageItem<'a>> = vec![section_card(
        Icon::SETTINGS_BACKUP_RESTORE,
        SettingsSection::Backup,
        language.text(Key::DataBackup),
        theme,
    )
    .into()];
    body_elements.extend(elements.into_iter().map(PageItem::Group));

    page_with_sheet(
        back_bar(theme),
        body(body_elements, theme),
        sheet,
    )
}
