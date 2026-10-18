//! The backup and restore page: backup AppData to SD card and restore from SD card.

use pomelo_hal::types::SdCardStatus;
use pomelo_material_symbols::Icon;
use pomelo_widgets::{
    confirm_dialog, Item, ItemGroup, Section, SystemPreferences, ThemeMode,
};

use crate::i18n::{Key, Language, LanguageExt as _};
use crate::pages::common::{
    back_bar, body, notice_card, page_with_modal, section_card, Page, UI,
};
use crate::style;
use crate::{Message, SettingsSection};

/// Modal Dialog asking user to confirm or cancel AppData backup.
fn backup_confirm_dialog(language: Language, theme: ThemeMode) -> UI<'static> {
    confirm_dialog(
        language.text(Key::BackupSystemData),
        language.text(Key::BackupWarning),
        language.text(Key::ConfirmBackup),
        Message::ConfirmBackupAppData,
        language.text(Key::Cancel),
        Message::CancelBackupAppData,
        false,
        theme,
    )
}

/// Modal Dialog asking user to confirm or cancel AppData restore.
fn restore_confirm_dialog(language: Language, theme: ThemeMode) -> UI<'static> {
    confirm_dialog(
        language.text(Key::RestoreSystemData),
        language.text(Key::RestoreWarning),
        language.text(Key::Continue),
        Message::ConfirmRestoreAppData,
        language.text(Key::Cancel),
        Message::CancelRestoreAppData,
        true,
        theme,
    )
}

/// Status card showing SD card connection state.
fn sd_status_card<'a>(label: &'static str, status: String, theme: ThemeMode) -> UI<'a> {
    Section::group(
        ItemGroup::new().push(Item::key_value(label, status)),
        theme,
    )
    .view()
}

/// Card offering AppData Backup and Restore options.
fn backup_restore_card<'a>(
    language: Language,
    theme: ThemeMode,
    sd_mounted: bool,
    has_backup: bool,
    is_busy: bool,
) -> UI<'a> {
    let backup_item = if sd_mounted && !is_busy {
        Item::action(
            language.text(Key::BackupSystemData),
            style::blue(),
            Message::PromptBackupAppData,
        )
    } else {
        Item::key_value(
            language.text(Key::BackupSystemData),
            if !sd_mounted {
                language.text(Key::NeedsSdCard)
            } else {
                ""
            },
        )
    };

    let restore_item = if sd_mounted && has_backup && !is_busy {
        Item::action(
            language.text(Key::RestoreSystemData),
            style::blue(),
            Message::PromptRestoreAppData,
        )
    } else {
        let hint = if !sd_mounted {
            language.text(Key::NeedsSdCard)
        } else if !has_backup {
            language.text(Key::NoBackupFound)
        } else {
            ""
        };
        Item::key_value(
            language.text(Key::RestoreSystemData),
            hint,
        )
    };

    Section::group(
        ItemGroup::new().push(backup_item).push(restore_item),
        theme,
    )
    .header(language.text(Key::DataBackup))
    .view()
}

/// Encapsulated state for the Backup & Restore settings page.
#[derive(Debug, Clone, Default)]
pub struct BackupState {
    pub has_backup: bool,
    pub is_backing_up: bool,
    pub is_restoring: bool,
    pub show_backup_confirm: bool,
    pub show_restore_confirm: bool,
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

    pub fn confirm_backup(&mut self, board: &pomelo_hal::Board, lang: Language) -> Option<String> {
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
                Some(msg)
            }
            Err(e) => {
                let msg = format!("{}: {}", lang.text(Key::BackupFailed), e);
                Some(msg)
            }
        }
    }

    pub fn prompt_restore(&mut self) {
        self.show_restore_confirm = true;
    }

    pub fn cancel_restore(&mut self) {
        self.show_restore_confirm = false;
    }

    pub fn confirm_restore(&mut self, board: &pomelo_hal::Board, lang: Language) -> Option<String> {
        self.show_restore_confirm = false;
        self.is_restoring = true;
        let res = board.storage().restore_app_data();
        self.is_restoring = false;
        self.refresh(board);
        match res {
            Ok(_) => {
                let _ = board.reboot();
                Some(lang.text(Key::RestoreSuccess).to_string())
            }
            Err(e) => {
                let msg = format!("{}: {}", lang.text(Key::RestoreFailed), e);
                Some(msg)
            }
        }
    }

    pub fn dismiss_notice(&mut self) {}

    pub fn has_active_modal(&self) -> bool {
        self.show_backup_confirm || self.show_restore_confirm
    }

    pub fn dismiss_modal(&mut self) {
        self.show_backup_confirm = false;
        self.show_restore_confirm = false;
    }
}

/// The dedicated Backup & Restore page.
pub(crate) fn backup_page<'a>(
    preferences: SystemPreferences,
    sd_status: SdCardStatus,
    state: &BackupState,
) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let has_backup = state.has_backup;
    let is_backing_up = state.is_backing_up;
    let is_restoring = state.is_restoring;
    let show_backup_confirm = state.show_backup_confirm;
    let show_restore_confirm = state.show_restore_confirm;

    let is_busy = is_backing_up || is_restoring;
    let sd_mounted = sd_status == SdCardStatus::Mounted;

    let mut elements: Vec<UI<'a>> = Vec::new();

    elements.push(section_card(
        Icon::SETTINGS_BACKUP_RESTORE,
        SettingsSection::Backup,
        language.text(Key::DataBackup),
        theme,
    ));

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

    let modal = if show_backup_confirm {
        Some(backup_confirm_dialog(language, theme))
    } else if show_restore_confirm {
        Some(restore_confirm_dialog(language, theme))
    } else {
        None
    };

    page_with_modal(
        back_bar(theme),
        body(elements, theme),
        modal,
    )
}
