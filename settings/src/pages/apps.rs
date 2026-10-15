//! Applications management and per-app configuration pages.

use iced::widget::{button, container, opaque, stack, text, Column, Row, Space};
use iced::{Border, Color, Length};

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

fn modal_cancel_btn<'a>(
    label: &'static str,
    on_press: Message,
    theme: ThemeMode,
) -> button::Button<'a, Message, iced::Theme, iced::Renderer> {
    button(
        container(
            text(label)
                .size(style::DETAIL_FONT)
                .color(style::label_for(theme)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill),
    )
    .width(Length::FillPortion(1))
    .height(Length::Fixed(44.0))
    .padding(0)
    .style(move |_theme, status| {
        let fill = if matches!(status, button::Status::Pressed | button::Status::Hovered) {
            style::track_off_pressed_for(theme)
        } else {
            style::track_off_for(theme)
        };
        button::Style {
            background: Some(fill.into()),
            text_color: style::label_for(theme),
            border: Border {
                radius: 12.0.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
            snap: false,
        }
    })
    .on_press(on_press)
}

fn modal_confirm_btn<'a>(
    label: &'static str,
    bg_color: Color,
    on_press: Message,
) -> button::Button<'a, Message, iced::Theme, iced::Renderer> {
    button(
        container(
            text(label)
                .size(style::DETAIL_FONT)
                .color(Color::WHITE),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill),
    )
    .width(Length::FillPortion(1))
    .height(Length::Fixed(44.0))
    .padding(0)
    .style(move |_theme, status| {
        let fill = if matches!(status, button::Status::Pressed | button::Status::Hovered) {
            Color::from_rgba(bg_color.r * 0.85, bg_color.g * 0.85, bg_color.b * 0.85, bg_color.a)
        } else {
            bg_color
        };
        button::Style {
            background: Some(fill.into()),
            text_color: Color::WHITE,
            border: Border {
                radius: 12.0.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
            snap: false,
        }
    })
    .on_press(on_press)
}

/// Modal dialog asking user to confirm or cancel clearing calendar subscription.
fn clear_schedule_confirm_modal<'a>(language: Language, theme: ThemeMode) -> UI<'a> {
    let title = text(language.text(Key::ClearSubscription))
        .size(style::LABEL_FONT)
        .color(style::label_for(theme));
    let warning = text(language.text(Key::ConfirmClearPrompt))
        .size(style::DETAIL_FONT)
        .color(style::red());
    let cancel_btn = modal_cancel_btn(language.text(Key::Cancel), Message::DailyScheduleCancelClear, theme);
    let confirm_btn = modal_confirm_btn(
        language.text(Key::ConfirmClear),
        style::red(),
        Message::DailyScheduleConfirmClear,
    );

    build_dialog_box(
        title,
        warning,
        Row::with_children(vec![cancel_btn.into(), confirm_btn.into()])
            .spacing(12.0)
            .width(Length::Fill),
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
    let title = text(title_str.to_string())
        .size(style::LABEL_FONT)
        .color(style::label_for(theme));
    let msg = text(msg_str.to_string())
        .size(style::DETAIL_FONT)
        .color(style::label_for(theme));
    let ok_btn = modal_confirm_btn(
        language.text(Key::Ok),
        style::accent(),
        Message::DailyScheduleDismissNotice,
    );

    build_dialog_box(
        title,
        msg,
        Row::with_children(vec![ok_btn.into()]).width(Length::Fill),
        theme,
    )
}

fn build_dialog_box<'a>(
    title: text::Text<'a>,
    message: text::Text<'a>,
    actions: Row<'a, Message>,
    theme: ThemeMode,
) -> UI<'a> {
    let dialog_content = Column::with_children(vec![
        title.into(),
        Space::new().height(Length::Fixed(12.0)).into(),
        message.into(),
        Space::new().height(Length::Fixed(20.0)).into(),
        actions.into(),
    ])
    .width(Length::Fill);

    let dialog_box = container(dialog_content)
        .width(Length::Fixed(360.0))
        .padding(20.0)
        .style(move |_theme| container::Style {
            background: Some(style::card_for(theme).into()),
            border: Border {
                color: style::card_border_for(theme),
                width: style::CARD_BORDER_WIDTH,
                radius: style::CARD_RADIUS.into(),
            },
            ..container::Style::default()
        });

    opaque(
        container(dialog_box)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(style::backdrop().into()),
                ..container::Style::default()
            }),
    )
    .into()
}

/// The Daily Schedule app settings page (Settings -> Apps -> Daily Schedule).
pub(crate) fn daily_schedule_page<'a>(
    preferences: SystemPreferences,
    config: &DailyScheduleConfig,
    is_syncing: bool,
    show_clear_confirm: bool,
    notice: Option<&(String, String)>,
) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

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
