//! Applications management and per-app configuration pages.

use iced::widget::{stack, text};

use pomelo_material_symbols::Icon;
use pomelo_widgets::{Language, SystemPreferences, ThemeMode};
use pomelo_hal::PersistentConfig;
use serde::{Deserialize, Serialize};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::card::{Card, Header, Tile};
use crate::pages::common::{body, page, UI};
use crate::style;
use std::collections::BTreeMap;
use toml::Value;

use crate::{Message, SettingsSection};

/// Persistent configuration model for the Daily Schedule application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DailyScheduleConfig {
    pub subscription_url: Option<String>,
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl PersistentConfig for DailyScheduleConfig {
    const APP_ID: &'static str = "DAILY_SCHEDULE";
    const FILE_NAME: &'static str = "config.toml";
}

/// Sanitizes a calendar subscription link (handles webcal://, quotes, whitespace).
pub fn sanitize_calendar_url(raw: &str) -> Option<String> {
    let s = raw.trim();
    let s = s.trim_matches(|c| c == '"' || c == '\'' || c == '<' || c == '>');
    let s = s.trim();
    if s.starts_with("webcal://") {
        Some(format!("https://{}", &s[9..]))
    } else if s.starts_with("http://") || s.starts_with("https://") {
        Some(s.to_string())
    } else {
        None
    }
}

/// The Applications list page (Settings -> Apps).
pub(crate) fn apps_page<'a>(
    preferences: SystemPreferences,
    schedule_configured: bool,
) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let schedule_secondary = text(if schedule_configured {
        language.text(Key::Configured)
    } else {
        language.text(Key::NotConfigured)
    })
    .size(style::VALUE_FONT)
    .color(if schedule_configured {
        style::green()
    } else {
        style::muted_for(theme)
    });

    let apps_card = Card::new(theme).tile(
        Tile::section(
            Icon::CALENDAR_MONTH,
            SettingsSection::AppDailySchedule,
            language.text(Key::DailySchedule),
        )
        .secondary(schedule_secondary),
    );

    page(
        Header::section(
            Icon::APPS,
            SettingsSection::Apps,
            language.text(Key::Applications),
        )
        .view(theme),
        body(vec![apps_card.view()]),
    )
}

/// Modal dialog asking user to confirm or cancel clearing calendar subscription.
fn clear_schedule_confirm_modal<'a>(language: Language, theme: ThemeMode) -> UI<'a> {
    crate::pages::common::confirm_dialog(
        language.text(Key::ClearSubscription),
        language.text(Key::ConfirmClearPrompt),
        style::red(),
        language.text(Key::Cancel),
        Message::DailyScheduleCancelClear,
        language.text(Key::ConfirmClear),
        Message::DailyScheduleConfirmClear,
        true,
        theme,
    )
}

/// Generic notice modal for schedule settings with a single OK button to dismiss.
fn schedule_notice_modal<'a>(
    title_str: &str,
    msg_str: &str,
    language: Language,
    theme: ThemeMode,
) -> UI<'a> {
    crate::pages::common::notice_dialog(
        title_str,
        msg_str,
        language.text(Key::Ok),
        Message::DailyScheduleDismissNotice,
        theme,
    )
}

/// Encapsulated state for the Daily Schedule application settings page.
#[derive(Debug, Clone, Default)]
pub struct DailyScheduleSettingsState {
    pub config: DailyScheduleConfig,
    pub show_clear_confirm: bool,
    pub is_testing_sync: bool,
    pub sync_notice: Option<(String, String)>,
}

impl DailyScheduleSettingsState {
    pub fn refresh(&mut self, _board: &pomelo_hal::Board) {
        self.config = DailyScheduleConfig::load();
    }

    pub fn import_clipboard(&mut self, text: Option<String>, lang: Language) {
        let clipboard = text.unwrap_or_default();
        if let Some(url) = sanitize_calendar_url(&clipboard) {
            self.config.subscription_url = Some(url);
            let err_title = if lang == Language::English { "Import Failed" } else { "导入失败" };
            if let Err(e) = self.config.save() {
                self.sync_notice = Some((
                    err_title.to_string(),
                    format!("{}: {}", err_title, e),
                ));
            } else {
                self.sync_notice = Some((
                    lang.text(Key::LinkImported).to_string(),
                    lang.text(Key::LinkImported).to_string(),
                ));
            }
        } else {
            self.sync_notice = Some((
                lang.text(Key::ClipboardInvalid).to_string(),
                lang.text(Key::ClipboardInvalid).to_string(),
            ));
        }
    }

    pub fn prompt_clear(&mut self) {
        self.show_clear_confirm = true;
    }

    pub fn cancel_clear(&mut self) {
        self.show_clear_confirm = false;
    }

    pub fn confirm_clear(&mut self) {
        self.show_clear_confirm = false;
        self.config.subscription_url = None;
        let _ = self.config.save();
        let cache_path = pomelo_hal::storage::app_file_path("DAILY_SCHEDULE", "today_cache.toml");
        let _ = std::fs::remove_file(cache_path);
    }

    pub fn test_sync(&mut self, board: &pomelo_hal::Board, lang: Language) {
        if let Some(url) = &self.config.subscription_url {
            let mut clean_url = url.trim().to_string();
            if clean_url.starts_with("webcal://") {
                clean_url = format!("https://{}", &clean_url[9..]);
            }
            let res = board.http().start(&clean_url);
            match res {
                Ok(()) => {
                    self.is_testing_sync = true;
                }
                Err(e) => {
                    self.is_testing_sync = false;
                    self.sync_notice = Some((
                        lang.text(Key::SyncFailed).to_string(),
                        format!("{}: {:?}", lang.text(Key::SyncFailed), e),
                    ));
                }
            }
        }
    }

    pub fn handle_sync_frame(&mut self, board: &pomelo_hal::Board, lang: Language) {
        let mut http = board.http();
        match http.state() {
            pomelo_hal::HttpState::Running { .. } => {}
            pomelo_hal::HttpState::Done => {
                self.is_testing_sync = false;
                let status_code = http.status_code().unwrap_or(200);
                if (200..300).contains(&status_code) {
                    let payload = http.take_result().unwrap_or_default();
                    let title = lang.text(Key::SyncSuccess).to_string();
                    let msg = if lang == Language::English {
                        format!("Sync verified (HTTP {}, {} bytes)", status_code, payload.len())
                    } else {
                        format!("同步验证成功（HTTP {}, 共 {} 字节）", status_code, payload.len())
                    };
                    self.sync_notice = Some((title, msg));
                } else {
                    let _ = http.take_result();
                    let title = lang.text(Key::SyncFailed).to_string();
                    let msg = if lang == Language::English {
                        format!("HTTP Error: Status {}", status_code)
                    } else {
                        format!("HTTP 请求失败: 状态码 {}", status_code)
                    };
                    self.sync_notice = Some((title, msg));
                }
            }
            pomelo_hal::HttpState::Error => {
                self.is_testing_sync = false;
                let code_str = http.status_code().map(|c| format!(" (HTTP {})", c)).unwrap_or_default();
                let _ = http.take_result();
                self.sync_notice = Some((
                    lang.text(Key::SyncFailed).to_string(),
                    format!("{}{}", lang.text(Key::SyncFailed), code_str),
                ));
            }
            pomelo_hal::HttpState::Idle => {
                self.is_testing_sync = false;
            }
        }
    }

    pub fn dismiss_notice(&mut self) {
        self.sync_notice = None;
    }

    pub fn has_active_modal(&self) -> bool {
        self.show_clear_confirm || self.sync_notice.is_some()
    }

    pub fn dismiss_modal(&mut self) {
        self.show_clear_confirm = false;
        self.sync_notice = None;
    }
}

/// The Daily Schedule app settings page (Settings -> Apps -> Daily Schedule).
pub(crate) fn daily_schedule_page<'a>(
    preferences: SystemPreferences,
    state: &DailyScheduleSettingsState,
) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let config = &state.config;
    let is_syncing = state.is_testing_sync;
    let show_clear_confirm = state.show_clear_confirm;
    let notice = state.sync_notice.as_ref();

    let url_display = match &config.subscription_url {
        Some(url) => {
            if url.len() > 30 {
                format!("{}...", &url[..27])
            } else {
                url.clone()
            }
        }
        None => language.text(Key::NotConfigured).to_string(),
    };

    let url_color = if config.subscription_url.is_some() {
        style::label_for(theme)
    } else {
        style::muted_for(theme)
    };

    let mut source_card = Card::new(theme)
        .tile(
            Tile::field(language.text(Key::SubscriptionUrl))
                .secondary(text(url_display).size(style::VALUE_FONT).color(url_color)),
        )
        .tile(Tile::action(
            Icon::CONTENT_PASTE,
            style::IconColor::Blue,
            language.text(Key::ImportFromClipboard),
            Message::DailyScheduleImportClipboard,
        ));

    if config.subscription_url.is_some() {
        source_card = source_card.tile(Tile::action(
            Icon::DELETE,
            style::IconColor::Red,
            language.text(Key::ClearSubscription),
            Message::DailySchedulePromptClear,
        ));
    }

    let is_configured = config.subscription_url.is_some();
    let status_secondary = text(if is_configured {
        language.text(Key::Configured)
    } else {
        language.text(Key::NotConfigured)
    })
    .size(style::VALUE_FONT)
    .color(if is_configured {
        style::green()
    } else {
        style::muted_for(theme)
    });

    let mut sync_card = Card::new(theme).tile(
        Tile::field(language.text(Key::SyncStatus))
            .secondary(status_secondary),
    );

    if is_configured {
        let sync_label = if is_syncing {
            language.text(Key::TestingSync)
        } else {
            language.text(Key::TestSync)
        };
        sync_card = sync_card.tile(Tile::action(
            Icon::SYNC,
            style::IconColor::Green,
            sync_label,
            Message::DailyScheduleTestSync,
        ));
    }

    let content = page(
        Header::section(
            Icon::CALENDAR_MONTH,
            SettingsSection::AppDailySchedule,
            language.text(Key::DailySchedule),
        )
        .view(theme),
        body(vec![source_card.view(), sync_card.view()]),
    );

    if show_clear_confirm {
        let modal = clear_schedule_confirm_modal(language, theme);
        stack![content, modal].into()
    } else if let Some((title, msg)) = notice {
        let modal = schedule_notice_modal(title, msg, language, theme);
        stack![content, modal].into()
    } else {
        content
    }
}
