//! The storage page: usage bars and partition/card details for LittleFS and MicroSD card.

use iced::widget::{button, container, text, Column, Row, Space};
use iced::{Border, Color, Length};

use crate::i18n::{Key, Language, LanguageExt as _};
use crate::pages::card::{back_bar, section_card};
use crate::pages::common::{
    body, card_surface, detail_rows, notice_card, page_with_sheet, row_style, separator,
    ActionSheet, Group, PageItem, UI,
};
use crate::style;
use crate::{Message, SettingsSection};
use pomelo_hal::{SdCardInfo, SdCardStatus, StorageStats};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{SystemPreferences, ThemeMode};

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
) -> Group<'a> {
    let share = (percent.clamp(0.0, 100.0) * 100.0).round() as u16;
    let rest = 10_000u16.saturating_sub(share);

    let track_content: UI<'a> = if share == 0 {
        Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(style::BAR_HEIGHT))
            .into()
    } else if share >= 10_000 {
        container(Space::new().width(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fixed(style::BAR_HEIGHT))
            .style(move |_theme| container::Style {
                background: Some(bar_color.into()),
                border: Border {
                    radius: style::BAR_RADIUS.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    } else {
        let filled = container(Space::new().width(Length::Fill))
            .width(Length::FillPortion(share))
            .height(Length::Fixed(style::BAR_HEIGHT))
            .style(move |_theme| container::Style {
                background: Some(bar_color.into()),
                border: Border {
                    radius: style::BAR_RADIUS.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            });

        let remaining = container(Space::new().width(Length::Fill))
            .width(Length::FillPortion(rest))
            .height(Length::Fixed(style::BAR_HEIGHT));

        Row::with_children(vec![filled.into(), remaining.into()])
            .width(Length::Fill)
            .height(Length::Fixed(style::BAR_HEIGHT))
            .into()
    };

    let track = container(track_content)
        .width(Length::Fill)
        .height(Length::Fixed(style::BAR_HEIGHT))
        .style(move |_theme| container::Style {
            background: Some(style::bar_track_for(theme).into()),
            border: Border {
                radius: style::BAR_RADIUS.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

    let heading = Row::with_children(vec![
        text(title)
            .size(style::DETAIL_FONT)
            .color(style::label_for(theme))
            .into(),
        Space::new().width(Length::Fill).into(),
        text(used_text)
            .size(style::DETAIL_FONT)
            .color(style::detail_key_for(theme))
            .into(),
    ])
    .width(Length::Fill);

    let top_section = container(
        Column::with_children(vec![
            heading.into(),
            Space::new().height(Length::Fixed(style::BAR_GAP)).into(),
            track.into(),
        ])
        .width(Length::Fill),
    )
    .padding(style::USAGE_PADDING);

    let mut children: Vec<UI<'a>> = vec![top_section.into()];

    if !details.is_empty() {
        children.push(separator(theme));
        children.extend(detail_rows(details, theme));
    }

    Group::from_surface(card_surface(
        Column::with_children(children).width(Length::Fill),
        theme,
    ))
}

/// A card for unmounted / not-inserted SD card showing Title on the left and Status on the right.
fn sd_status_card<'a>(
    title: &'static str,
    status_text: String,
    theme: ThemeMode,
) -> Group<'a> {
    let line = Row::with_children(vec![
        text(title)
            .size(style::DETAIL_FONT)
            .color(style::label_for(theme))
            .into(),
        Space::new().width(Length::Fill).into(),
        text(status_text)
            .size(style::DETAIL_FONT)
            .color(style::notice_for(theme))
            .into(),
    ])
    .width(Length::Fill)
    .align_y(iced::Alignment::Center);

    Group::from_surface(card_surface(
        container(line)
            .width(Length::Fill)
            .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H]),
        theme,
    ))
}

/// The format SD option row without any '>' chevron.
fn format_option_card<'a>(label: &'static str, theme: ThemeMode) -> Group<'a> {
    let line = Row::with_children(vec![
        text(label)
            .size(style::DETAIL_FONT)
            .color(style::red())
            .into(),
        Space::new().width(Length::Fill).into(),
    ])
    .width(Length::Fill)
    .align_y(iced::Alignment::Center);

    let contents = container(line)
        .width(Length::Fill)
        .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H]);

    let btn = button(contents)
        .width(Length::Fill)
        .padding(0)
        .style(move |_theme, status| row_style(theme, status))
        .on_press(Message::PromptFormatSd);

    Group::from_surface(card_surface(btn, theme))
}

/// Modal Action Sheet asking user to confirm or cancel SD card formatting.
fn format_confirm_sheet(language: Language, theme: ThemeMode) -> ActionSheet<'static, Message> {
    ActionSheet::destructive_confirm(
        language.text(Key::FormatWarning),
        language.text(Key::ConfirmFormat),
        Message::ConfirmFormatSd,
        language.text(Key::Cancel),
        Message::CancelFormatSd,
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

    pub fn confirm_format(&mut self, board: &pomelo_hal::Board) {
        self.show_format_confirm = false;
        self.is_formatting = true;
        let _ = board.storage().format_sd();
        self.is_formatting = false;
        self.refresh(board);
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
) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let sd_status = state.sd_status;
    let sd_stats = state.sd_stats;
    let is_formatting = state.is_formatting;
    let show_format_confirm = state.show_format_confirm;

    let internal_used = 8_192u64;
    let internal_total = 11_534_336u64;
    let internal_val = format!(
        "{} {} / {}",
        language.text(Key::UsedPrefix),
        format_size(internal_used),
        format_size(internal_total)
    );

    let internal_details = vec![
        (language.text(Key::MountPoint), "/internal".to_string()),
        (language.text(Key::Filesystem), "LittleFS".to_string()),
    ];

    let internal_card = storage_capacity_card(
        language.text(Key::InternalStorage),
        internal_val,
        4.5,
        style::storage_bar(),
        internal_details,
        theme,
    );

    let mut elements = vec![internal_card];

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

    let sheet = show_format_confirm.then(|| format_confirm_sheet(language, theme));

    let mut body_elements: Vec<PageItem<'a>> = vec![section_card(
        Icon::STORAGE,
        SettingsSection::Storage,
        language.text(Key::Storage),
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


