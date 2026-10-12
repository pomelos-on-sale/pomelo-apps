//! The storage page: usage bars and partition/card details for LittleFS and MicroSD card.

use iced::widget::{button, container, opaque, stack, text, Column, Row, Space};
use iced::{Border, Color, Length};

use crate::i18n::{Key, Language, LanguageExt as _};
use crate::pages::card::Header;
use crate::pages::common::{body, card_surface, detail_rows, notice_card, page, row_style, separator, UI};
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
) -> UI<'a> {
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

    card_surface(
        Column::with_children(children).width(Length::Fill),
        theme,
    )
}

/// A card for unmounted / not-inserted SD card showing Title on the left and Status on the right.
fn sd_status_card<'a>(
    title: &'static str,
    status_text: String,
    theme: ThemeMode,
) -> UI<'a> {
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

    card_surface(
        container(line)
            .width(Length::Fill)
            .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H]),
        theme,
    )
}

/// The format SD option row without any '>' chevron.
fn format_option_card<'a>(label: &'static str, theme: ThemeMode) -> UI<'a> {
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

    card_surface(btn, theme)
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
            style::muted_for(theme)
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

fn build_dialog_box<'a>(
    title: text::Text<'a, iced::Theme, iced::Renderer>,
    desc: text::Text<'a, iced::Theme, iced::Renderer>,
    buttons: Row<'a, Message, iced::Theme, iced::Renderer>,
    theme: ThemeMode,
) -> UI<'a> {
    let dialog_content = Column::with_children(vec![
        title.into(),
        Space::new().height(Length::Fixed(12.0)).into(),
        desc.into(),
        Space::new().height(Length::Fixed(24.0)).into(),
        buttons.into(),
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

/// Modal dialog asking user to confirm or cancel SD card formatting.
fn format_confirm_modal<'a>(language: Language, theme: ThemeMode) -> UI<'a> {
    let title = text(language.text(Key::FormatSdCard))
        .size(style::LABEL_FONT)
        .color(style::label_for(theme));
    let warning = text(language.text(Key::FormatWarning))
        .size(style::DETAIL_FONT)
        .color(style::notice_for(theme));
    let cancel_btn = modal_cancel_btn(language.text(Key::Cancel), Message::CancelFormatSd, theme);
    let confirm_btn = modal_confirm_btn(
        language.text(Key::ConfirmFormat),
        style::red(),
        Message::ConfirmFormatSd,
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

pub(crate) fn storage_page<'a>(
    preferences: SystemPreferences,
    sd_status: SdCardStatus,
    sd_stats: Option<StorageStats>,
    _sd_info: Option<SdCardInfo>,
    is_formatting: bool,
    show_format_confirm: bool,
) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

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

    let pg = page(
        Header::section(
            Icon::STORAGE,
            SettingsSection::Storage,
            language.text(Key::Storage),
        )
        .view(theme),
        body(elements),
    );

    if show_format_confirm {
        let modal = format_confirm_modal(language, theme);
        stack![pg, modal].into()
    } else {
        pg
    }
}


