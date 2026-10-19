//! Daily Schedule application configuration and sub-pages.

use std::collections::BTreeMap;
use iced::widget::{button, container, text, text_input, Column, Row, Space};
use iced::{Alignment, Border, Color, Length, Padding};
use pomelo_hal::PersistentConfig;
use pomelo_material_symbols::Icon;
use pomelo_widgets::touch_keyboard;
use pomelo_widgets::{ActionItem, Language, SystemPreferences, ThemeMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::common::{
    back_bar, body, confirm_dialog, notice_card, page_with_modal, page_with_sub_page, section_card,
    Page, SubPage, UI,
};
use pomelo_widgets::{Item, ItemGroup, OpenPage, Section};
use crate::style;
use crate::{Message, SettingsSection};

/// A custom keyword-to-color preset rule for schedule event titles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorRule {
    /// Keyword to match against event title (e.g. "会议").
    pub keyword: String,
    /// Preset color index (0..7).
    pub color_index: usize,
}

/// Available preset colors for calendar event titles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetColor {
    Blue,
    Green,
    Orange,
    Purple,
    Red,
    Cyan,
    Yellow,
}

impl PresetColor {
    pub const ALL: [PresetColor; 7] = [
        PresetColor::Blue,
        PresetColor::Green,
        PresetColor::Orange,
        PresetColor::Purple,
        PresetColor::Red,
        PresetColor::Cyan,
        PresetColor::Yellow,
    ];

    pub fn index(self) -> usize {
        match self {
            Self::Blue => 0,
            Self::Green => 1,
            Self::Orange => 2,
            Self::Purple => 3,
            Self::Red => 4,
            Self::Cyan => 5,
            Self::Yellow => 6,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index % 7 {
            0 => Self::Blue,
            1 => Self::Green,
            2 => Self::Orange,
            3 => Self::Purple,
            4 => Self::Red,
            5 => Self::Cyan,
            6 => Self::Yellow,
            _ => unreachable!(),
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::Blue => Color::from_rgb8(56, 139, 253),
            Self::Green => Color::from_rgb8(46, 160, 67),
            Self::Orange => Color::from_rgb8(219, 109, 40),
            Self::Purple => Color::from_rgb8(163, 113, 247),
            Self::Red => Color::from_rgb8(248, 81, 73),
            Self::Cyan => Color::from_rgb8(57, 197, 207),
            Self::Yellow => Color::from_rgb8(227, 179, 65),
        }
    }

    pub fn name(self, lang: Language) -> &'static str {
        match lang {
            Language::Chinese => match self {
                Self::Blue => "蓝色",
                Self::Green => "绿色",
                Self::Orange => "橙色",
                Self::Purple => "紫色",
                Self::Red => "红色",
                Self::Cyan => "青色",
                Self::Yellow => "黄色",
            },
            Language::English => match self {
                Self::Blue => "Blue",
                Self::Green => "Green",
                Self::Orange => "Orange",
                Self::Purple => "Purple",
                Self::Red => "Red",
                Self::Cyan => "Cyan",
                Self::Yellow => "Yellow",
            },
        }
    }
}

impl std::fmt::Display for PresetColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Blue => "蓝色 (Blue)",
                Self::Green => "绿色 (Green)",
                Self::Orange => "橙色 (Orange)",
                Self::Purple => "紫色 (Purple)",
                Self::Red => "红色 (Red)",
                Self::Cyan => "青色 (Cyan)",
                Self::Yellow => "黄色 (Yellow)",
            }
        )
    }
}

/// State for the modal ActionSheet when adding or editing a color preset rule.
#[derive(Debug, Clone)]
pub struct ColorRuleModalState {
    /// If editing an existing rule, Some(index). If adding a new rule, None.
    pub editing_index: Option<usize>,
    /// Keyword being entered.
    pub keyword: String,
    /// Color currently selected.
    pub selected_color: PresetColor,
    /// Touch keyboard mode for typing.
    pub keyboard: touch_keyboard::KeyboardMode,
}

impl Default for ColorRuleModalState {
    fn default() -> Self {
        Self {
            editing_index: None,
            keyword: String::new(),
            selected_color: PresetColor::Blue,
            keyboard: touch_keyboard::KeyboardMode::Lower,
        }
    }
}

/// Persistent configuration model for the Daily Schedule application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DailyScheduleConfig {
    pub subscription_url: Option<String>,
    #[serde(default)]
    pub color_rules: Vec<ColorRule>,
    #[serde(default)]
    pub reminder_sound: bool,
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl PersistentConfig for DailyScheduleConfig {
    const APP_ID: &'static str = "DAILY_SCHEDULE";
}

/// Sanitizes a calendar subscription link (handles webcal://, quotes, whitespace).
pub fn sanitize_calendar_url(raw: &str) -> Option<String> {
    let s = raw.trim();
    let s = s.trim_matches(|c| c == '"' || c == '\'' || c == '<' || c == '>');
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("webcal://") {
        Some(format!("https://{rest}"))
    } else if s.starts_with("http://") || s.starts_with("https://") {
        Some(s.to_string())
    } else {
        None
    }
}

/// Modal Dialog asking user to confirm or cancel clearing calendar subscription.
fn clear_schedule_confirm_dialog(language: Language, theme: ThemeMode) -> UI<'static> {
    confirm_dialog(
        language.text(Key::ConfirmClear),
        language.text(Key::ConfirmClearPrompt),
        language.text(Key::ConfirmClear),
        Message::DailyScheduleConfirmClear,
        language.text(Key::Cancel),
        Message::DailyScheduleCancelClear,
        true,
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
    pub color_rule_modal: Option<ColorRuleModalState>,
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

    pub fn toggle_reminder_sound(&mut self) {
        self.config.reminder_sound = !self.config.reminder_sound;
        let _ = self.config.save();
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
        let _ = pomelo_hal::storage::kv_delete_raw("DAILY_SCHEDULE", "today_cache");
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

    pub fn prompt_add_rule(&mut self) {
        self.color_rule_modal = Some(ColorRuleModalState {
            editing_index: None,
            keyword: String::new(),
            selected_color: PresetColor::Blue,
            keyboard: touch_keyboard::KeyboardMode::Lower,
        });
    }

    pub fn prompt_edit_rule(&mut self, idx: usize) {
        if let Some(rule) = self.config.color_rules.get(idx) {
            self.color_rule_modal = Some(ColorRuleModalState {
                editing_index: Some(idx),
                keyword: rule.keyword.clone(),
                selected_color: PresetColor::from_index(rule.color_index),
                keyboard: touch_keyboard::KeyboardMode::Lower,
            });
        }
    }

    pub fn cancel_rule(&mut self) {
        self.color_rule_modal = None;
    }

    pub fn save_rule(&mut self) {
        if let Some(modal) = self.color_rule_modal.take() {
            let kw = modal.keyword.trim().to_string();
            if !kw.is_empty() {
                let rule = ColorRule {
                    keyword: kw,
                    color_index: modal.selected_color.index(),
                };
                if let Some(idx) = modal.editing_index {
                    if idx < self.config.color_rules.len() {
                        self.config.color_rules[idx] = rule;
                    }
                } else {
                    self.config.color_rules.push(rule);
                }
                let _ = self.config.save();
            }
        }
    }

    pub fn delete_rule(&mut self, idx: usize) {
        if idx < self.config.color_rules.len() {
            self.config.color_rules.remove(idx);
            let _ = self.config.save();
        }
        self.color_rule_modal = None;
    }

    pub fn rule_keyword_changed(&mut self, kw: String) {
        if let Some(modal) = &mut self.color_rule_modal {
            modal.keyword = kw;
        }
    }

    pub fn rule_color_selected(&mut self, color: PresetColor) {
        if let Some(modal) = &mut self.color_rule_modal {
            modal.selected_color = color;
        }
    }

    pub fn rule_key(&mut self, action: touch_keyboard::KeyAction) {
        if let Some(modal) = &mut self.color_rule_modal {
            match action {
                touch_keyboard::KeyAction::Char(c) => modal.keyword.push(c),
                touch_keyboard::KeyAction::Space => modal.keyword.push(' '),
                touch_keyboard::KeyAction::Backspace => {
                    modal.keyword.pop();
                }
                touch_keyboard::KeyAction::SwitchMode(mode) => modal.keyboard = mode,
                touch_keyboard::KeyAction::Enter => self.save_rule(),
                touch_keyboard::KeyAction::Paste => {}
            }
        }
    }

    pub fn paste_rule_keyword(&mut self, text: &str) {
        if let Some(modal) = &mut self.color_rule_modal {
            modal.keyword.push_str(text);
        }
    }

    pub fn has_active_modal(&self) -> bool {
        self.show_clear_confirm || self.color_rule_modal.is_some()
    }

    pub fn dismiss_modal(&mut self) {
        self.show_clear_confirm = false;
        self.sync_notice = None;
        self.color_rule_modal = None;
    }
}

/// The Daily Schedule app settings page (Settings -> Apps -> Daily Schedule).
pub(crate) fn daily_schedule_page<'a>(
    preferences: SystemPreferences,
    state: &DailyScheduleSettingsState,
) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let config = &state.config;
    let is_syncing = state.is_testing_sync;

    let reminder_card = Section::group(
        ItemGroup::new().push(Item::switch(
            language.text(Key::ScheduleReminderSound),
            config.reminder_sound,
            Message::DailyScheduleToggleReminderSound,
        )),
        theme,
    )
    .view();

    let link_card = Section::group(
        ItemGroup::new()
            .push(Item::nav(
                language.text(Key::SubscriptionUrl),
                None::<String>,
                OpenPage(Message::Open(SettingsSection::AppDailyScheduleUrl)),
            ))
            .push(Item::nav(
                language.text(Key::ColorRules),
                None::<String>,
                OpenPage(Message::Open(SettingsSection::AppDailyScheduleColors)),
            )),
        theme,
    )
    .view();

    let is_configured = config.subscription_url.is_some();
    let status_str = if is_configured {
        language.text(Key::Configured)
    } else {
        language.text(Key::NotConfigured)
    };

    let mut sync_group = ItemGroup::new().push(Item::key_value(language.text(Key::SyncStatus), status_str));

    if is_configured {
        let sync_label = if is_syncing {
            language.text(Key::TestingSync)
        } else {
            language.text(Key::TestSync)
        };
        sync_group = sync_group.push(Item::action(
            sync_label,
            style::blue(),
            Message::DailyScheduleTestSync,
        ));
    }
    let sync_card = Section::group(sync_group, theme).view();

    let modal: Option<UI<'a>> = None;

    page_with_modal(
        back_bar(theme),
        body(
            vec![
                section_card(
                    Icon::CALENDAR_MONTH,
                    SettingsSection::AppDailySchedule,
                    language.text(Key::DailySchedule),
                    theme,
                ),
                reminder_card,
                link_card,
                sync_card,
            ],
            theme,
        ),
        modal,
    )
}

/// The Daily Schedule subscription link details page (Settings -> Apps -> Daily Schedule -> Subscription Link).
pub(crate) fn daily_schedule_url_page<'a>(
    preferences: SystemPreferences,
    state: &DailyScheduleSettingsState,
) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let config = &state.config;
    let show_clear_confirm = state.show_clear_confirm;

    let url_display = match &config.subscription_url {
        Some(url) => url.clone(),
        None => language.text(Key::NotConfigured).to_string(),
    };

    let group = ItemGroup::new()
        .push(Item::key_value(language.text(Key::SubscriptionUrl), url_display))
        .push(Item::action(
            language.text(Key::ConfirmClear),
            style::blue(),
            Message::DailySchedulePromptClear,
        ))
        .push(Item::action(
            language.text(Key::ImportFromClipboard),
            style::blue(),
            Message::DailyScheduleImportClipboard,
        ));
    let group_card = Section::group(group, theme).view();

    let modal = show_clear_confirm.then(|| clear_schedule_confirm_dialog(language, theme));

    page_with_modal(
        back_bar(theme),
        body(vec![group_card], theme),
        modal,
    )
}

/// Modal SubPage for adding or editing a color preset rule.
fn color_rule_prompt_sheet<'a>(
    language: Language,
    theme: ThemeMode,
    modal: &ColorRuleModalState,
) -> SubPage<'a, Message> {
    let is_edit = modal.editing_index.is_some();
    let title = if is_edit {
        language.text(Key::EditColorRule)
    } else {
        language.text(Key::AddColorRule)
    };

    let kw_input = text_input(language.text(Key::KeywordPlaceholder), &modal.keyword)
        .size(style::DETAIL_FONT)
        .padding(0.0)
        .width(Length::Fill)
        .style(move |_theme, _status| text_input::Style {
            background: Color::TRANSPARENT.into(),
            border: Border::default(),
            icon: Color::TRANSPARENT,
            placeholder: style::separator_for(theme),
            value: style::label_for(theme),
            selection: style::accent(),
        })
        .on_input(Message::DailyScheduleRuleKeywordChanged);
    let field_box = |label: &'static str, secondary: UI<'a>| -> UI<'a> {
        container(
            Row::new()
                .width(Length::Fill)
                .align_y(Alignment::Center)
                .push(text(label).size(style::LABEL_FONT).color(style::label_for(theme)))
                .push(Space::new().width(Length::Fixed(style::VALUE_GAP * 2.0)))
                .push(secondary),
        )
        .width(Length::Fill)
        .padding([style::ROW_PADDING_V, style::ROW_PADDING_H])
        .style(move |_theme| container::Style {
            border: Border {
                color: style::separator_for(theme),
                width: style::CARD_BORDER_WIDTH,
                radius: style::FIELD_RADIUS.into(),
            },
            ..container::Style::default()
        })
        .into()
    };

    let kw_tile = field_box(language.text(Key::Keyword), kw_input.into());

    let color_picker = iced::widget::pick_list(
        &PresetColor::ALL[..],
        Some(modal.selected_color),
        Message::DailyScheduleRuleColorSelected,
    )
    .padding(Padding {
        top: 4.0,
        bottom: 4.0,
        left: 10.0,
        right: 10.0,
    })
    .text_size(style::DETAIL_FONT);

    let color_indicator = text("●")
        .size(style::DETAIL_FONT)
        .color(modal.selected_color.color());

    let color_entry = Row::with_children(vec![
        color_indicator.into(),
        color_picker.into(),
    ])
    .spacing(8)
    .align_y(Alignment::Center);

    let color_tile = field_box(language.text(Key::Color), color_entry.into());

    let mut upper_children: Vec<UI<'a>> = vec![kw_tile, color_tile];

    if let Some(idx) = modal.editing_index {
        let delete_btn = button(
            Row::new()
                .spacing(8)
                .align_y(Alignment::Center)
                .push(pomelo_widgets::icon(Icon::DELETE).size(style::FONT_SMALL).color(style::error()))
                .push(
                    text(language.text(Key::DeleteRule))
                        .size(style::DETAIL_FONT)
                        .color(style::error()),
                ),
        )
        .padding(Padding {
            top: 8.0,
            bottom: 8.0,
            left: 16.0,
            right: 16.0,
        })
        .style(|_theme, status| button::Style {
            background: if matches!(status, button::Status::Pressed) {
                Some(Color::from_rgba(1.0, 0.2, 0.2, 0.22).into())
            } else {
                Some(Color::from_rgba(1.0, 0.2, 0.2, 0.12).into())
            },
            border: Border {
                radius: 10.0.into(),
                color: Color::from_rgba(1.0, 0.2, 0.2, 0.4),
                width: 1.0,
            },
            ..button::Style::default()
        })
        .on_press(Message::DailyScheduleDeleteRule(idx));

        upper_children.push(
            container(delete_btn)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .into(),
        );
    }

    let half = container(
        Column::with_children(upper_children)
            .width(Length::Fill)
            .spacing(style::PROMPT_GAP),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(Padding {
        top: 6.0,
        bottom: 0.0,
        left: style::PROMPT_PADDING,
        right: style::PROMPT_PADDING,
    });

    let keys = container(touch_keyboard::band_with_theme(
        modal.keyboard,
        Message::DailyScheduleRuleKey,
        theme,
    ))
    .width(Length::Fill)
    .height(Length::Fixed(style::keyboard_height()));

    let body = Column::with_children(vec![half.into(), keys.into()])
        .width(Length::Fill)
        .height(Length::Fill);

    let save_action = ActionItem::new("Save", Message::DailyScheduleSaveRule)
        .icon(Icon::CHECK)
        .enabled(!modal.keyword.trim().is_empty());

    let cancel_action = ActionItem::secondary("Cancel", Message::DailyScheduleCancelRule)
        .icon(Icon::CLOSE);

    SubPage::modal(title, theme)
        .title_icon(Icon::PALETTE)
        .leading(cancel_action)
        .trailing(save_action)
        .body(body)
}

/// The Daily Schedule color presets page (Settings -> Apps -> Daily Schedule -> Color Presets).
pub(crate) fn daily_schedule_colors_page<'a>(
    preferences: SystemPreferences,
    state: &DailyScheduleSettingsState,
) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let config = &state.config;

    let add_card = Section::group(
        ItemGroup::new().push(Item::action(
            language.text(Key::AddColorRule),
            style::blue(),
            Message::DailySchedulePromptAddRule,
        )),
        theme,
    )
    .view();

    let rules_content = if config.color_rules.is_empty() {
        notice_card(language.text(Key::NoColorRules), theme)
    } else {
        let mut group = ItemGroup::new();
        for (index, rule) in config.color_rules.iter().enumerate() {
            let preset = PresetColor::from_index(rule.color_index);
            let val = format!("● {}", preset.name(language));
            group = group.push(Item::nav(
                rule.keyword.clone(),
                Some(val),
                OpenPage(Message::DailySchedulePromptEditRule(index)),
            ));
        }
        Section::group(group, theme).view()
    };

    let sheet = state.color_rule_modal.as_ref().map(|modal| {
        color_rule_prompt_sheet(language, theme, modal)
    });

    page_with_sub_page(
        back_bar(theme),
        body(
            vec![
                section_card(
                    Icon::PALETTE,
                    SettingsSection::AppDailyScheduleColors,
                    language.text(Key::ColorRules),
                    theme,
                ),
                add_card,
                rules_content,
            ],
            theme,
        ),
        sheet,
    )
}
