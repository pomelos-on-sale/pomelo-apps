//! An elegant, clean iOS-inspired Alarm Clock application for Pomelo OS.
//!
//! Designed for the 480×480 AMOLED display with pitch black background,
//! vibrant Apple Clock orange accents, bold digital typography, intuitive
//! time stepper roller, and responsive iOS-style toggle switches.

pub mod model;
pub mod sound;
pub mod style;

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Datelike, Local, Timelike};
use iced::widget::{button, column, container, rule, scrollable, text, Column, Row, Space};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Subscription, Task};

use pomelo_hal::storage::PersistentConfig;
use pomelo_hal::Board;
use pomelo_material_symbols::Icon;
use pomelo_widgets::{icon, AppPreferences, Language, SystemPreferences, ThemeMode};

use model::{sound_name, AlarmConfig, AlarmItem, AlarmRepeat, PRESET_LABELS};
use sound::AlarmSounds;

fn tick_stream() -> impl iced::futures::Stream<Item = Message> {
    pomelo_widgets::interval_stream("alarm-clock-tick", Duration::from_millis(1000), || {
        Message::Tick(Local::now())
    })
}

/// Current active view mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewMode {
    /// Normal alarm list, optionally with delete controls enabled.
    List { is_editing_list: bool },
    /// Add or edit alarm details.
    Edit { is_new: bool, draft: AlarmItem },
    /// Full-screen active ringing overlay.
    Ringing {
        alarm_id: u32,
        time_str: String,
        label: String,
        sound_id: u8,
    },
}

/// Messages handled by the Alarm Clock application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Periodic 1-second system clock tick.
    Tick(DateTime<Local>),
    /// Toggles the on/off state of an alarm by ID.
    ToggleAlarm(u32),
    /// Enters or exits list edit mode (showing delete buttons).
    ToggleListEdit,
    /// Opens the Add Alarm dialog.
    OpenNew,
    /// Opens the Edit Alarm dialog for an existing alarm.
    OpenEdit(u32),
    /// Cancels editing and returns to the list view.
    CancelEdit,
    /// Saves the edited alarm and returns to the list view.
    SaveEdit,
    /// Deletes an alarm by ID.
    DeleteAlarm(u32),
    /// Adjusts the draft hour (+1 / -1).
    PickerHourChange(i8),
    /// Adjusts the draft minute (+1 / -1).
    PickerMinuteChange(i8),
    /// Cycles the repeat schedule (Once -> Everyday -> Weekdays -> Weekends).
    CycleRepeat,
    /// Selects a preset label for the draft alarm.
    SelectLabel(String),
    /// Cycles the ringtone melody (0 -> 1 -> 2).
    CycleSound,
    /// Toggles preview playback of the selected ringtone.
    TogglePreviewSound,
    /// Dismisses active ringing alarm and stops sound.
    DismissRinging,
    /// Snoozes active ringing alarm for 5 minutes.
    SnoozeRinging,
}

/// The Alarm Clock application state.
pub struct AlarmClock {
    pub config: AlarmConfig,
    pub mode: ViewMode,
    pub board: Arc<Board>,
    pub sounds: AlarmSounds,
    pub preferences: SystemPreferences,
    last_triggered_key: Option<(u32, u32)>, // (day_of_year, minute_of_day)
    snooze_until: Option<DateTime<Local>>,
    preview_playing: bool,
}

impl AlarmClock {
    /// Creates a new Alarm Clock application using the desktop simulated board.
    #[cfg(not(target_os = "espidf"))]
    pub fn new() -> Self {
        Self::with_board(Arc::new(Board::simulated()))
    }

    /// Creates a new Alarm Clock application with the given hardware board.
    pub fn with_board(board: Arc<Board>) -> Self {
        let config = AlarmConfig::load();
        let sounds = AlarmSounds::init_and_load();

        Self {
            config,
            mode: ViewMode::List {
                is_editing_list: false,
            },
            board,
            sounds,
            preferences: SystemPreferences::default(),
            last_triggered_key: None,
            snooze_until: None,
            preview_playing: false,
        }
    }

    /// Access the configuration.
    pub fn config(&self) -> &AlarmConfig {
        &self.config
    }

    /// Updates the application state in response to a message.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => {
                // Check if snoozed alarm has reached its time
                if let Some(snooze_time) = self.snooze_until {
                    if now >= snooze_time && !matches!(self.mode, ViewMode::Ringing { .. }) {
                        self.snooze_until = None;
                        self.trigger_alarm(0, "稍后提醒".to_string(), 0);
                        return Task::none();
                    }
                }

                // Check configured alarms
                let current_key = (now.ordinal(), now.hour() * 60 + now.minute());
                if self.last_triggered_key != Some(current_key) {
                    for alarm in &mut self.config.alarms {
                        if alarm.should_ring(now) {
                            self.last_triggered_key = Some(current_key);
                            let alarm_id = alarm.id;
                            let label = alarm.label.clone();
                            let sound_id = alarm.sound_id;

                            // One-time alarms turn off after firing
                            if alarm.repeat == AlarmRepeat::Once {
                                alarm.enabled = false;
                                let _ = self.config.save();
                            }

                            self.trigger_alarm(alarm_id, label, sound_id);
                            break;
                        }
                    }
                }
            }

            Message::ToggleAlarm(id) => {
                self.config.toggle(id);
            }

            Message::ToggleListEdit => {
                if let ViewMode::List { is_editing_list } = &mut self.mode {
                    *is_editing_list = !*is_editing_list;
                }
            }

            Message::OpenNew => {
                self.stop_preview();
                let now = Local::now();
                let default_hour = ((now.hour() + 1) % 24) as u8;
                let draft = AlarmItem::new(
                    0,
                    default_hour,
                    0,
                    "闹钟",
                    AlarmRepeat::Everyday,
                );
                self.mode = ViewMode::Edit {
                    is_new: true,
                    draft,
                };
            }

            Message::OpenEdit(id) => {
                self.stop_preview();
                if let Some(item) = self.config.get(id) {
                    self.mode = ViewMode::Edit {
                        is_new: false,
                        draft: item.clone(),
                    };
                }
            }

            Message::CancelEdit => {
                self.stop_preview();
                self.mode = ViewMode::List {
                    is_editing_list: false,
                };
            }

            Message::SaveEdit => {
                self.stop_preview();
                if let ViewMode::Edit { draft, .. } = &self.mode {
                    let mut item = draft.clone();
                    if item.label.trim().is_empty() {
                        item.label = match self.preferences.language {
                            Language::Chinese => "闹钟".to_string(),
                            Language::English => "Alarm".to_string(),
                        };
                    }
                    self.config.upsert(item);
                }
                self.mode = ViewMode::List {
                    is_editing_list: false,
                };
            }

            Message::DeleteAlarm(id) => {
                self.stop_preview();
                self.config.remove(id);
                if matches!(self.mode, ViewMode::Edit { .. }) {
                    self.mode = ViewMode::List {
                        is_editing_list: false,
                    };
                }
            }

            Message::PickerHourChange(delta) => {
                if let ViewMode::Edit { draft, .. } = &mut self.mode {
                    let new_h = (draft.hour as i16 + delta as i16).rem_euclid(24) as u8;
                    draft.hour = new_h;
                }
            }

            Message::PickerMinuteChange(delta) => {
                if let ViewMode::Edit { draft, .. } = &mut self.mode {
                    let new_m = (draft.minute as i16 + delta as i16).rem_euclid(60) as u8;
                    draft.minute = new_m;
                }
            }

            Message::CycleRepeat => {
                if let ViewMode::Edit { draft, .. } = &mut self.mode {
                    draft.repeat = draft.repeat.next();
                }
            }

            Message::SelectLabel(lbl) => {
                if let ViewMode::Edit { draft, .. } = &mut self.mode {
                    draft.label = lbl;
                }
            }

            Message::CycleSound => {
                if let ViewMode::Edit { draft, .. } = &mut self.mode {
                    draft.sound_id = (draft.sound_id + 1) % 3;
                    if self.preview_playing {
                        self.sounds.play(&self.board, draft.sound_id);
                    }
                }
            }

            Message::TogglePreviewSound => {
                if let ViewMode::Edit { draft, .. } = &self.mode {
                    if self.preview_playing {
                        self.stop_preview();
                    } else {
                        self.preview_playing = true;
                        self.sounds.play(&self.board, draft.sound_id);
                    }
                }
            }

            Message::DismissRinging => {
                self.sounds.stop(&self.board);
                self.mode = ViewMode::List {
                    is_editing_list: false,
                };
            }

            Message::SnoozeRinging => {
                self.sounds.stop(&self.board);
                let snooze_mins = self.config.snooze_minutes as i64;
                self.snooze_until = Some(Local::now() + chrono::Duration::minutes(snooze_mins));
                self.mode = ViewMode::List {
                    is_editing_list: false,
                };
            }
        }

        Task::none()
    }

    fn trigger_alarm(&mut self, alarm_id: u32, label: String, sound_id: u8) {
        let now = Local::now();
        let time_str = format!("{:02}:{:02}", now.hour(), now.minute());
        self.mode = ViewMode::Ringing {
            alarm_id,
            time_str,
            label,
            sound_id,
        };
        self.sounds.play(&self.board, sound_id);
    }

    fn stop_preview(&mut self) {
        if self.preview_playing {
            self.preview_playing = false;
            self.sounds.stop(&self.board);
        }
    }

    /// Application subscription for the periodic clock tick.
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::run(tick_stream)
    }

    /// Main view dispatcher.
    pub fn view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = match &self.mode {
            ViewMode::List { is_editing_list } => self.view_list(*is_editing_list),
            ViewMode::Edit { is_new, draft } => self.view_edit(*is_new, draft),
            ViewMode::Ringing {
                time_str, label, ..
            } => self.view_ringing(time_str, label),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(style::BG_BLACK.into()),
                ..container::Style::default()
            })
            .into()
    }

    /// Renders the main alarm list view inspired by iOS Clock.
    fn view_list(&self, is_editing_list: bool) -> Element<'_, Message> {
        let lang = self.preferences.language;

        // Top Bar
        let edit_label = if is_editing_list {
            match lang {
                Language::Chinese => "完成",
                Language::English => "Done",
            }
        } else {
            match lang {
                Language::Chinese => "编辑",
                Language::English => "Edit",
            }
        };

        let edit_btn = button(
            text(edit_label)
                .size(style::HEADER_BTN_FONT_SIZE)
                .color(style::ORANGE),
        )
        .padding([6.0, 10.0])
        .style(|_, status| style::orange_text_btn_style(status))
        .on_press(Message::ToggleListEdit);

        let title_text = match lang {
            Language::Chinese => "闹钟",
            Language::English => "Alarms",
        };

        let title = text(title_text)
            .size(style::TITLE_FONT_SIZE)
            .color(style::TEXT_PRIMARY);

        let add_btn = button(
            icon(Icon::ADD)
                .size(24.0)
                .color(style::ORANGE),
        )
        .padding([6.0, 10.0])
        .style(|_, status| style::orange_text_btn_style(status))
        .on_press(Message::OpenNew);

        let top_bar = Row::new()
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .push(edit_btn)
            .push(Space::new().width(Length::Fill))
            .push(title)
            .push(Space::new().width(Length::Fill))
            .push(add_btn);

        let top_container = container(top_bar)
            .width(Length::Fill)
            .padding([10.0, 20.0]);

        // Alarms list
        let mut list_col = Column::new()
            .width(Length::Fill)
            .spacing(12.0)
            .align_x(Alignment::Center);

        if self.config.alarms.is_empty() {
            let empty_text = match lang {
                Language::Chinese => "暂无闹钟\n点击右上角 + 添加",
                Language::English => "No Alarms\nTap + to add one",
            };
            let empty_view = container(
                column![
                    icon(Icon::ALARM).size(48.0).color(style::TEXT_MUTED),
                    Space::new().height(Length::Fixed(12.0)),
                    text(empty_text)
                        .size(style::LABEL_FONT_SIZE)
                        .color(style::TEXT_MUTED)
                        .align_x(Alignment::Center),
                ]
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fixed(240.0))
            .center_x(Length::Fill)
            .center_y(Length::Fill);

            list_col = list_col.push(empty_view);
        } else {
            for item in &self.config.alarms {
                list_col = list_col.push(self.alarm_card(item, is_editing_list));
            }
        }

        // Bottom spacer to clear the round AMOLED bezel
        list_col = list_col.push(Space::new().height(Length::Fixed(40.0)));

        let scrollable_content = scrollable(list_col)
            .width(Length::Fill)
            .height(Length::Fill);

        column![top_container, scrollable_content]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Renders an individual alarm card row.
    fn alarm_card(&self, item: &AlarmItem, is_editing_list: bool) -> Element<'_, Message> {
        let lang = self.preferences.language;
        let time_color = if item.enabled {
            style::TEXT_PRIMARY
        } else {
            style::TEXT_MUTED
        };

        let time_widget = text(item.time_string())
            .size(style::TIME_FONT_SIZE)
            .color(time_color);

        let subtitle_widget = text(item.subtitle_string(lang))
            .size(style::SUBTITLE_FONT_SIZE)
            .color(style::TEXT_MUTED);

        let info_col = column![time_widget, subtitle_widget].spacing(2.0);

        let action_widget: Element<'_, Message> = if is_editing_list {
            let del_btn = button(
                icon(Icon::DELETE)
                    .size(24.0)
                    .color(style::DANGER_RED),
            )
            .padding([8.0, 12.0])
            .style(|_, status| {
                let bg = match status {
                    button::Status::Pressed => style::CARD_PRESSED,
                    _ => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: style::DANGER_RED,
                    border: Border::default(),
                    shadow: Shadow::default(),
                    snap: false,
                }
            })
            .on_press(Message::DeleteAlarm(item.id));
            del_btn.into()
        } else {
            self.ios_switch(item.enabled, Message::ToggleAlarm(item.id))
        };

        // Main row
        let row_content = Row::new()
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .push(info_col)
            .push(Space::new().width(Length::Fill))
            .push(action_widget);

        let card_inner = container(row_content)
            .width(Length::Fill)
            .padding([style::CARD_PAD_V, style::CARD_PAD_H]);

        button(card_inner)
            .width(Length::Fixed(440.0))
            .padding(0)
            .style(|_, status| style::alarm_row_btn_style(status))
            .on_press(Message::OpenEdit(item.id))
            .into()
    }

    /// Renders an authentic iOS-style toggle switch.
    fn ios_switch(&self, on: bool, on_toggle: Message) -> Element<'_, Message> {
        let knob = container(Space::new())
            .width(Length::Fixed(style::KNOB_SIZE))
            .height(Length::Fixed(style::KNOB_SIZE))
            .style(|_| container::Style {
                background: Some(Color::WHITE.into()),
                border: Border {
                    radius: style::KNOB_RADIUS.into(),
                    ..Border::default()
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
                    offset: iced::Vector::new(0.0, 2.0),
                    blur_radius: 4.0,
                },
                ..container::Style::default()
            });

        let mut row_items: Vec<Element<'_, Message>> = Vec::new();
        if on {
            row_items.push(Space::new().width(Length::Fill).into());
            row_items.push(knob.into());
            row_items.push(Space::new().width(Length::Fixed(style::TOGGLE_INSET)).into());
        } else {
            row_items.push(Space::new().width(Length::Fixed(style::TOGGLE_INSET)).into());
            row_items.push(knob.into());
            row_items.push(Space::new().width(Length::Fill).into());
        }

        button(
            Row::with_children(row_items)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center),
        )
        .width(Length::Fixed(style::TOGGLE_W))
        .height(Length::Fixed(style::TOGGLE_H))
        .padding(0)
        .style(move |_, status| style::toggle_btn_style(on, status))
        .on_press(on_toggle)
        .into()
    }

    /// Renders the Add / Edit Alarm screen.
    fn view_edit(&self, is_new: bool, draft: &AlarmItem) -> Element<'_, Message> {
        let lang = self.preferences.language;

        // Top Bar
        let cancel_str = match lang {
            Language::Chinese => "取消",
            Language::English => "Cancel",
        };
        let save_str = match lang {
            Language::Chinese => "存储",
            Language::English => "Save",
        };
        let title_str = match (is_new, lang) {
            (true, Language::Chinese) => "添加闹钟",
            (true, Language::English) => "Add Alarm",
            (false, Language::Chinese) => "编辑闹钟",
            (false, Language::English) => "Edit Alarm",
        };

        let cancel_btn = button(
            text(cancel_str)
                .size(style::HEADER_BTN_FONT_SIZE)
                .color(style::ORANGE),
        )
        .padding([6.0, 10.0])
        .style(|_, status| style::orange_text_btn_style(status))
        .on_press(Message::CancelEdit);

        let title = text(title_str)
            .size(style::TITLE_FONT_SIZE)
            .color(style::TEXT_PRIMARY);

        let save_btn = button(
            text(save_str)
                .size(style::HEADER_BTN_FONT_SIZE)
                .color(style::ORANGE),
        )
        .padding([6.0, 10.0])
        .style(|_, status| style::orange_text_btn_style(status))
        .on_press(Message::SaveEdit);

        let top_bar = Row::new()
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .push(cancel_btn)
            .push(Space::new().width(Length::Fill))
            .push(title)
            .push(Space::new().width(Length::Fill))
            .push(save_btn);

        let top_container = container(top_bar)
            .width(Length::Fill)
            .padding([10.0, 20.0]);

        // Time Picker Stepper Box
        let time_picker = self.view_time_picker(draft);

        // Options Card (Repeat, Label presets, Ringtone)
        let options_card = self.view_options_card(draft);

        let mut edit_col = Column::new()
            .width(Length::Fill)
            .spacing(16.0)
            .align_x(Alignment::Center)
            .push(time_picker)
            .push(options_card);

        // If editing existing alarm, add Delete Alarm button
        if !is_new {
            let del_text = match lang {
                Language::Chinese => "删除闹钟",
                Language::English => "Delete Alarm",
            };
            let delete_card_btn = button(
                container(
                    text(del_text)
                        .size(style::LABEL_FONT_SIZE)
                        .color(style::DANGER_RED),
                )
                .width(Length::Fill)
                .align_x(Alignment::Center),
            )
            .width(Length::Fixed(440.0))
            .padding([12.0, 16.0])
            .style(|_, status| style::delete_btn_style(status))
            .on_press(Message::DeleteAlarm(draft.id));

            edit_col = edit_col.push(delete_card_btn);
        }

        // Bottom space
        edit_col = edit_col.push(Space::new().height(Length::Fixed(40.0)));

        let scrollable_content = scrollable(edit_col)
            .width(Length::Fill)
            .height(Length::Fill);

        column![top_container, scrollable_content]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Renders the iOS-style digital time picker with steppers.
    fn view_time_picker(&self, draft: &AlarmItem) -> Element<'_, Message> {
        let hour_up = button(icon(Icon::KEYBOARD_ARROW_UP).size(26.0))
            .padding([4.0, 16.0])
            .style(|_, status| style::stepper_btn_style(status))
            .on_press(Message::PickerHourChange(1));

        let hour_text = text(format!("{:02}", draft.hour))
            .size(style::PICKER_FONT_SIZE)
            .color(style::TEXT_PRIMARY);

        let hour_down = button(icon(Icon::KEYBOARD_ARROW_DOWN).size(26.0))
            .padding([4.0, 16.0])
            .style(|_, status| style::stepper_btn_style(status))
            .on_press(Message::PickerHourChange(-1));

        let hour_col = column![hour_up, hour_text, hour_down]
            .spacing(6.0)
            .align_x(Alignment::Center);

        let colon = text(":")
            .size(style::PICKER_FONT_SIZE)
            .color(style::TEXT_MUTED);

        let minute_up = button(icon(Icon::KEYBOARD_ARROW_UP).size(26.0))
            .padding([4.0, 16.0])
            .style(|_, status| style::stepper_btn_style(status))
            .on_press(Message::PickerMinuteChange(1));

        let minute_text = text(format!("{:02}", draft.minute))
            .size(style::PICKER_FONT_SIZE)
            .color(style::TEXT_PRIMARY);

        let minute_down = button(icon(Icon::KEYBOARD_ARROW_DOWN).size(26.0))
            .padding([4.0, 16.0])
            .style(|_, status| style::stepper_btn_style(status))
            .on_press(Message::PickerMinuteChange(-1));

        let minute_col = column![minute_up, minute_text, minute_down]
            .spacing(6.0)
            .align_x(Alignment::Center);

        let picker_row = Row::new()
            .align_y(Alignment::Center)
            .spacing(24.0)
            .push(hour_col)
            .push(colon)
            .push(minute_col);

        container(picker_row)
            .width(Length::Fixed(440.0))
            .padding([14.0, 20.0])
            .center_x(Length::Fill)
            .style(|_| style::alarm_card_style(ThemeMode::Dark))
            .into()
    }

    /// Renders the settings card for repeat, label presets, and ringtone.
    fn view_options_card(&self, draft: &AlarmItem) -> Element<'_, Message> {
        let lang = self.preferences.language;

        // Repeat row
        let repeat_title = match lang {
            Language::Chinese => "重复",
            Language::English => "Repeat",
        };
        let repeat_val = draft.repeat.display_name(lang);
        let repeat_btn = button(
            Row::new()
                .align_y(Alignment::Center)
                .spacing(6.0)
                .push(text(repeat_val).size(style::LABEL_FONT_SIZE).color(style::ORANGE))
                .push(icon(Icon::CHEVRON_RIGHT).size(18.0).color(style::TEXT_MUTED)),
        )
        .padding(0)
        .style(|_, status| style::orange_text_btn_style(status))
        .on_press(Message::CycleRepeat);

        let repeat_row = Row::new()
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .push(text(repeat_title).size(style::LABEL_FONT_SIZE).color(style::TEXT_PRIMARY))
            .push(Space::new().width(Length::Fill))
            .push(repeat_btn);

        // Divider
        let divider1 = rule::horizontal(1).style(|_| rule::Style {
            color: style::SEPARATOR_COLOR,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        });

        // Label presets row
        let label_title = match lang {
            Language::Chinese => "标签",
            Language::English => "Label",
        };

        let mut label_chips = Row::new().spacing(8.0).align_y(Alignment::Center);
        for preset in PRESET_LABELS {
            let is_selected = draft.label == preset;
            let chip_btn = button(
                text(preset)
                    .size(style::SUBTITLE_FONT_SIZE)
                    .color(if is_selected { Color::BLACK } else { style::TEXT_MUTED }),
            )
            .padding([6.0, 10.0])
            .style(move |_, status| style::pill_btn_style(is_selected, status))
            .on_press(Message::SelectLabel(preset.to_string()));

            label_chips = label_chips.push(chip_btn);
        }

        let label_section = column![
            Row::new()
                .width(Length::Fill)
                .align_y(Alignment::Center)
                .push(text(label_title).size(style::LABEL_FONT_SIZE).color(style::TEXT_PRIMARY))
                .push(Space::new().width(Length::Fill))
                .push(text(draft.label.clone()).size(style::LABEL_FONT_SIZE).color(style::ORANGE)),
            Space::new().height(Length::Fixed(6.0)),
            label_chips,
        ];

        // Divider
        let divider2 = rule::horizontal(1).style(|_| rule::Style {
            color: style::SEPARATOR_COLOR,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        });

        // Ringtone sound row
        let sound_title = match lang {
            Language::Chinese => "铃声",
            Language::English => "Sound",
        };
        let sound_val = sound_name(draft.sound_id, lang);
        let preview_icon = if self.preview_playing {
            Icon::STOP
        } else {
            Icon::PLAY_ARROW
        };

        let preview_btn = button(icon(preview_icon).size(20.0).color(style::ORANGE))
            .padding([4.0, 8.0])
            .style(|_, status| style::orange_text_btn_style(status))
            .on_press(Message::TogglePreviewSound);

        let sound_cycle_btn = button(
            Row::new()
                .align_y(Alignment::Center)
                .spacing(6.0)
                .push(text(sound_val).size(style::LABEL_FONT_SIZE).color(style::ORANGE))
                .push(icon(Icon::CHEVRON_RIGHT).size(18.0).color(style::TEXT_MUTED)),
        )
        .padding(0)
        .style(|_, status| style::orange_text_btn_style(status))
        .on_press(Message::CycleSound);

        let sound_row = Row::new()
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .push(text(sound_title).size(style::LABEL_FONT_SIZE).color(style::TEXT_PRIMARY))
            .push(Space::new().width(Length::Fill))
            .push(preview_btn)
            .push(sound_cycle_btn);

        let content = column![repeat_row, divider1, label_section, divider2, sound_row]
            .spacing(12.0)
            .width(Length::Fill);

        container(content)
            .width(Length::Fixed(440.0))
            .padding([style::CARD_PAD_V, style::CARD_PAD_H])
            .style(|_| style::alarm_card_style(ThemeMode::Dark))
            .into()
    }

    /// Renders full-screen ringing alert overlay.
    fn view_ringing(&self, time_str: &str, label: &str) -> Element<'_, Message> {
        let lang = self.preferences.language;
        let alarm_icon = icon(Icon::ALARM)
            .size(56.0)
            .color(style::ORANGE);

        let label_text = text(label.to_string())
            .size(style::TITLE_FONT_SIZE)
            .color(style::TEXT_PRIMARY);

        let time_text = text(time_str.to_string())
            .size(style::RINGING_FONT_SIZE)
            .color(style::TEXT_PRIMARY);

        let snooze_str = match lang {
            Language::Chinese => "稍后提醒 (5分钟)",
            Language::English => "Snooze (5 min)",
        };

        let dismiss_str = match lang {
            Language::Chinese => "关闭闹钟",
            Language::English => "Dismiss",
        };

        let snooze_btn = button(
            container(
                text(snooze_str)
                    .size(style::LABEL_FONT_SIZE)
                    .color(style::TEXT_PRIMARY),
            )
            .width(Length::Fill)
            .align_x(Alignment::Center),
        )
        .width(Length::Fixed(360.0))
        .padding([14.0, 20.0])
        .style(|_, status| {
            let bg = match status {
                button::Status::Pressed => style::CARD_PRESSED,
                _ => style::CARD_BG,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: style::TEXT_PRIMARY,
                border: Border {
                    color: style::BORDER_COLOR,
                    width: 1.0,
                    radius: 14.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::SnoozeRinging);

        let dismiss_btn = button(
            container(
                text(dismiss_str)
                    .size(style::HEADER_BTN_FONT_SIZE)
                    .color(Color::BLACK),
            )
            .width(Length::Fill)
            .align_x(Alignment::Center),
        )
        .width(Length::Fixed(360.0))
        .padding([14.0, 20.0])
        .style(|_, status| {
            let bg = match status {
                button::Status::Pressed => style::ORANGE_PRESSED,
                _ => style::ORANGE,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Color::BLACK,
                border: Border {
                    radius: 14.0.into(),
                    ..Border::default()
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::DismissRinging);

        let col = column![
            Space::new().height(Length::Fixed(40.0)),
            alarm_icon,
            Space::new().height(Length::Fixed(12.0)),
            label_text,
            Space::new().height(Length::Fixed(8.0)),
            time_text,
            Space::new().height(Length::Fill),
            snooze_btn,
            Space::new().height(Length::Fixed(12.0)),
            dismiss_btn,
            Space::new().height(Length::Fixed(50.0)),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center);

        container(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| container::Style {
                background: Some(style::BG_BLACK.into()),
                ..container::Style::default()
            })
            .into()
    }
}

impl AppPreferences for AlarmClock {
    fn preferences(&self) -> SystemPreferences {
        self.preferences
    }

    fn set_preferences(&mut self, preferences: SystemPreferences) {
        self.preferences = preferences;
    }
}

#[cfg(not(target_os = "espidf"))]
impl Default for AlarmClock {
    fn default() -> Self {
        Self::new()
    }
}

impl AlarmClock {
    /// Returns the application iced theme.
    pub fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    /// Application boot entrypoint for standalone desktop runner.
    #[cfg(not(target_os = "espidf"))]
    pub fn boot() -> (Self, Task<Message>) {
        (Self::new(), Task::none())
    }
}
