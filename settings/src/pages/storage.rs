//! The storage page: usage bars and partition/card details for Flash NVS and MicroSD card.

use iced::Color;

use crate::i18n::{Key, Language, LanguageExt as _};
use crate::pages::common::{
    back_bar, body, confirm_dialog, notice_card, page_with_modal, section_card, Page, UI,
    UsageCard,
};
use crate::style;
use crate::{Message, SettingsSection};
use pomelo_hal::{SdCardInfo, SdCardStatus, StorageStats};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{Item, ItemGroup, Section, SystemPreferences, ThemeMode};

fn format_size(bytes: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;

    if bytes >= 1024 * 1024 * 1024 {
        let gb = bytes as f64 / GB;
        format!("{:.1} GB", gb)
    } else {
        let mb = bytes as f64 / MB;
        if mb < 0.005 {
            "0 MB".to_string()
        } else if mb < 0.1 {
            format!("{:.2} MB", mb)
        } else {
            format!("{:.1} MB", mb)
        }
    }
}

/// A combined storage card with usage bar at top and key/value details at the end.
fn storage_capacity_card<'a>(
    title: &'static str,
    used_text: String,
    percent: f32,
    bar_color: Color,
    details: Vec<(&'static str, String)>,
    theme: ThemeMode,
) -> UI<'a> {
    UsageCard::new(title, used_text, percent, bar_color, theme)
        .details(details)
        .view()
}

/// A card for unmounted / not-inserted SD card showing Title on the left and Status on the right.
fn sd_status_card<'a>(
    title: &'static str,
    status_text: String,
    theme: ThemeMode,
) -> UI<'a> {
    Section::group(
        ItemGroup::new().push(Item::key_value(title, status_text)),
        theme,
    )
    .view()
}

/// The format SD option row without any '>' chevron.
fn format_option_card<'a>(label: &'static str, theme: ThemeMode) -> UI<'a> {
    Section::group(
        ItemGroup::new().push(Item::action(label, style::red(), Message::PromptFormatSd)),
        theme,
    )
    .view()
}

/// Confirmation modal dialog for SD Card formatting.
fn format_confirm_dialog(language: Language, theme: ThemeMode) -> UI<'static> {
    confirm_dialog(
        language.text(Key::FormatSdCard),
        language.text(Key::FormatWarning),
        language.text(Key::ConfirmFormat),
        Message::ConfirmFormatSd,
        language.text(Key::Cancel),
        Message::CancelFormatSd,
        true,
        theme,
    )
}

/// Encapsulated state for the Storage settings page.
#[derive(Debug, Clone, Default)]
pub struct StorageState {
    pub sd_status: SdCardStatus,
    pub sd_stats: Option<StorageStats>,
    pub sd_info: Option<SdCardInfo>,
    pub is_formatting: bool,
    pub show_format_confirm: bool,
}

impl StorageState {
    pub fn refresh(&mut self, board: &pomelo_hal::Board) {
        let storage = board.storage();
        self.sd_status = storage.sd_status();
        self.sd_stats = storage.sd_stats().ok();
        self.sd_info = storage.sd_card_info();
    }

    pub fn prompt_format(&mut self) {
        self.show_format_confirm = true;
    }

    pub fn cancel_format(&mut self) {
        self.show_format_confirm = false;
    }

    pub fn confirm_format(&mut self, board: &pomelo_hal::Board, lang: Language) -> Option<String> {
        self.show_format_confirm = false;
        self.is_formatting = true;
        let res = board.storage().format_sd();
        self.is_formatting = false;
        self.refresh(board);
        match res {
            Ok(_) => Some(lang.text(Key::FormatSuccess).to_string()),
            Err(_) => Some(lang.text(Key::FormatFailed).to_string()),
        }
    }

    pub fn has_active_modal(&self) -> bool {
        self.show_format_confirm
    }

    pub fn dismiss_modal(&mut self) {
        self.show_format_confirm = false;
    }
}

pub(crate) fn storage_page<'a>(
    preferences: SystemPreferences,
    state: &StorageState,
) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let sd_status = state.sd_status;
    let sd_stats = state.sd_stats;
    let is_formatting = state.is_formatting;
    let show_format_confirm = state.show_format_confirm;

    let internal_used = 12_288u64;
    let internal_total = 65_536u64;
    let internal_val = format!(
        "{} {} / {}",
        language.text(Key::UsedPrefix),
        format_size(internal_used),
        format_size(internal_total)
    );

    let internal_details = vec![
        (language.text(Key::MountPoint), "Flash NVS".to_string()),
        (language.text(Key::Filesystem), "NVS KV".to_string()),
    ];

    let internal_card = storage_capacity_card(
        language.text(Key::InternalStorage),
        internal_val,
        18.7,
        style::storage_bar(),
        internal_details,
        theme,
    );

    let mut elements: Vec<UI<'a>> = vec![
        section_card(
            Icon::STORAGE,
            SettingsSection::Storage,
            language.text(Key::Storage),
            theme,
        ),
        internal_card,
    ];

    match sd_status {
        SdCardStatus::Mounted => {
            let (used_str, total_str, used_pct) = if let Some(stats) = sd_stats {
                let pct = stats.used_percent();
                (
                    format_size(stats.used_bytes()),
                    format_size(stats.total_bytes),
                    pct,
                )
            } else {
                ("0 MB".to_string(), "Unknown".to_string(), 0.0)
            };

            let sd_bar_val = format!(
                "{} {} / {}",
                language.text(Key::UsedPrefix),
                used_str,
                total_str
            );

            let sd_details = vec![
                (
                    language.text(Key::Status),
                    language.text(Key::Inserted).to_string(),
                ),
                (language.text(Key::MountPoint), "/sdcard".to_string()),
                (language.text(Key::Filesystem), "FAT32".to_string()),
            ];

            let sd_card = storage_capacity_card(
                language.text(Key::SdCard),
                sd_bar_val,
                used_pct,
                style::storage_bar(),
                sd_details,
                theme,
            );

            elements.push(sd_card);

            if !is_formatting {
                elements.push(format_option_card(language.text(Key::FormatSdCard), theme));
            }
        }
        SdCardStatus::Unformatted => {
            elements.push(sd_status_card(
                language.text(Key::SdCard),
                language.text(Key::Unformatted).to_string(),
                theme,
            ));

            if !is_formatting {
                elements.push(format_option_card(language.text(Key::FormatSdCard), theme));
            }
        }
        SdCardStatus::NotInserted => {
            elements.push(sd_status_card(
                language.text(Key::SdCard),
                language.text(Key::NotInserted).to_string(),
                theme,
            ));
        }
        SdCardStatus::Error => {
            elements.push(sd_status_card(
                language.text(Key::SdCard),
                language.text(Key::ConnectFailed).to_string(),
                theme,
            ));

            if !is_formatting {
                elements.push(format_option_card(language.text(Key::FormatSdCard), theme));
            }
        }
    }

    if is_formatting {
        elements.push(notice_card(language.text(Key::Formatting), theme));
    }

    let modal = show_format_confirm.then(|| format_confirm_dialog(language, theme));

    page_with_modal(
        back_bar(theme),
        body(elements, theme),
        modal,
    )
}
