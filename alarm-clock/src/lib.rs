//! An elegant, clean iOS-inspired Alarm Clock application for Pomelo OS.
//!
//! Designed for the 480×480 AMOLED display with pitch black background,
//! vibrant Apple Clock orange accents, bold digital typography, intuitive
//! touch wheel rollers, ActionSheet modal add/edit flow, and grouped settings cards.

pub mod model;
pub mod sound;
pub mod style;
pub mod views;

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Datelike, Local, Timelike};
use iced::widget::container;
use iced::{Element, Length, Subscription, Task};

use pomelo_hal::storage::PersistentConfig;
use pomelo_hal::Board;
use pomelo_widgets::{AppPreferences, KeepAwake, Language, SystemPreferences};

use model::{AlarmConfig, AlarmItem, AlarmRepeat};
use sound::AlarmSounds;


/// Strongly-typed page navigation routes for the Alarm Clock application.
///
/// Implements [`pomelo_widgets::NavigationAction<Message>`], enforcing at compile time
/// that only valid page routes can be bound to [`pomelo_widgets::Item::nav`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlarmRoute {
    /// Navigate to the repeat schedule selection ActionSheet.
    Repeat,
    /// Navigate to the label preset selection ActionSheet.
    Label,
    /// Navigate to the sound melody selection ActionSheet.
    Sound,
    /// Navigate to the snooze duration selection ActionSheet.
    SnoozeDuration,
}

impl pomelo_widgets::NavigationAction<Message> for AlarmRoute {
    fn into_message(self) -> Message {
        Message::OpenRoute(self)
    }
}

/// Current active view mode.
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    /// Normal alarm list, optionally with delete controls enabled.
    List { is_editing_list: bool },
    /// Add or edit alarm details inside an ActionSheet modal.
    Edit {
        is_new: bool,
        draft: AlarmItem,
        sub_sheet: Option<AlarmRoute>,
    },
    /// Full-screen active ringing overlay.
    Ringing {
        alarm_id: u32,
        time_str: String,
        label: String,
        sound_id: u8,
        session_id: u32,
    },
}

/// Messages handled by the Alarm Clock application.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// System time has been synchronized or changed; recalculates next alarm.
    TimeSynced,
    /// Checks system schedule and snooze state to trigger an alarm if due.
    CheckSchedule,
    /// Ringing timeout reached (60 seconds elapsed).
    RingTimeout { session_id: u32 },
    /// Toggles the on/off state of an alarm by ID.
    ToggleAlarm(u32),
    /// Enters or exits list edit mode (showing delete buttons).
    ToggleListEdit,
    /// Opens the Add Alarm dialog.
    OpenNew,
    /// Opens the Edit Alarm dialog for an existing alarm.
    OpenEdit(u32),
    /// Saves the edited alarm and returns to the list view.
    SaveEdit,
    /// Deletes an alarm by ID.
    DeleteAlarm(u32),
    /// Adjusts the draft hour (+1 / -1).
    HourChange(i8),
    /// Adjusts the draft minute (+1 / -1).
    MinuteChange(i8),
    /// Drag delta update on the hour wheel roller.
    HourDrag(f32),
    /// End of drag gesture on the hour wheel roller.
    HourDragEnd,
    /// Drag delta update on the minute wheel roller.
    MinuteDrag(f32),
    /// End of drag gesture on the minute wheel roller.
    MinuteDragEnd,
    /// Opens an item selection ActionSheet for the given route.
    OpenRoute(AlarmRoute),
    /// Toggles the repeat selection for day index 0..6 (Sun..Sat).
    ToggleRepeatDay(usize),
    /// Selects a preset label.
    SelectLabel(String),
    /// Selects a ringtone melody.
    SelectSound(u8),
    /// Selects the snooze duration in minutes.
    SelectSnoozeMinutes(u8),
    /// Toggles snooze for the draft alarm.
    ToggleSnooze,
    /// Toggles preview playback of the selected ringtone.
    TogglePreviewSound,
    /// Dismisses active ringing alarm and stops sound.
    DismissRinging,
    /// Snoozes active ringing alarm for 5 minutes.
    SnoozeRinging,
    /// Navigates back one level in the view hierarchy, or snoozes active ringing.
    Back,
}

/// The Alarm Clock application state.
pub struct AlarmClock {
    pub config: AlarmConfig,
    pub mode: ViewMode,
    pub board: Arc<Board>,
    pub sounds: AlarmSounds,
    pub preferences: SystemPreferences,
    pub snooze_until: Option<DateTime<Local>>,
    last_triggered_key: Option<(u32, u32)>, // (day_of_year, minute_of_day)
    next_session_id: u32,
    preview_playing: bool,
    hour_drag_accum: f32,
    minute_drag_accum: f32,
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

        let app = Self {
            config,
            mode: ViewMode::List {
                is_editing_list: false,
            },
            board,
            sounds,
            preferences: SystemPreferences::default(),
            last_triggered_key: None,
            snooze_until: None,
            next_session_id: 0,
            preview_playing: false,
            hour_drag_accum: 0.0,
            minute_drag_accum: 0.0,
        };
        app.sync_wakeup_schedule();
        app
    }

    /// Synchronizes the nearest upcoming alarm or snooze trigger time with the system wakeup scheduler.
    pub fn sync_wakeup_schedule(&self) {
        let now = Local::now();
        let mut next_dt = self.config.next_trigger(now);

        if let Some(snooze) = self.snooze_until {
            if snooze > now {
                next_dt = match next_dt {
                    Some(trigger) => Some(trigger.min(snooze)),
                    None => Some(snooze),
                };
            }
        }

        if let Some(target) = next_dt {
            let unix_secs = target.timestamp();
            let subsec_nanos = target.timestamp_subsec_nanos();
            let target_st = if unix_secs >= 0 {
                std::time::SystemTime::UNIX_EPOCH
                    + Duration::from_secs(unix_secs as u64)
                    + Duration::from_nanos(subsec_nanos as u64)
            } else {
                std::time::SystemTime::UNIX_EPOCH
            };
            self.board
                .wakeup_scheduler()
                .schedule("alarm_clock", target_st, true);
            println!(
                "[alarm_clock] Synced next wakeup to {:?} (in {} s)",
                target,
                target.signed_duration_since(now).num_seconds()
            );
        } else {
            self.board.wakeup_scheduler().cancel("alarm_clock");
            println!("[alarm_clock] No active alarms, cancelled wakeup schedule");
        }
    }

    /// Access the configuration.
    pub fn config(&self) -> &AlarmConfig {
        &self.config
    }

    /// Handles back navigation from a physical hardware button or soft-back action.
    ///
    /// - Returns `true` if the back action was consumed internally (e.g. closing a sub-sheet,
    ///   cancelling an edit, exiting list-edit mode, or snoozing an active alarm).
    /// - Returns `false` if the application is already at the root list view, signalling
    ///   to the host (`AppLauncher`) that it may return to the grid.
    pub fn go_back(&mut self) -> bool {
        if let ViewMode::Edit {
            sub_sheet: Some(_), ..
        } = self.mode
        {
            self.stop_preview();
            if let ViewMode::Edit {
                ref mut sub_sheet, ..
            } = self.mode
            {
                *sub_sheet = None;
            }
            return true;
        }

        if let ViewMode::Edit {
            sub_sheet: None, ..
        } = self.mode
        {
            self.stop_preview();
            self.mode = ViewMode::List {
                is_editing_list: false,
            };
            return true;
        }

        if let ViewMode::List {
            is_editing_list: true,
        } = self.mode
        {
            self.mode = ViewMode::List {
                is_editing_list: false,
            };
            return true;
        }

        if let ViewMode::List {
            is_editing_list: false,
        } = self.mode
        {
            return false;
        }

        if let ViewMode::Ringing { .. } = self.mode {
            self.sounds.stop(&self.board);
            let alarm_id = self.ringing_alarm_id();
            let snooze_mins = if let Some(alarm) = self.config.get(alarm_id) {
                alarm.snooze_minutes as i64
            } else {
                5
            };
            self.snooze_until = Some(Local::now() + chrono::Duration::minutes(snooze_mins));
            self.sync_wakeup_schedule();
            self.mode = ViewMode::List {
                is_editing_list: false,
            };
            return true;
        }

        false
    }

    /// Updates the application state in response to a message.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        println!("[alarm_clock:update] Received message: {:?}", message);

        match message {
            Message::CheckSchedule => {
                let now = Local::now();
                println!(
                    "[alarm_clock:CheckSchedule] Checking schedule at {} (alarms_count={})",
                    now.format("%Y-%m-%d %H:%M:%S"),
                    self.config.alarms.len()
                );

                // Check if snoozed alarm has reached its time
                if let Some(snooze_time) = self.snooze_until {
                    println!("[alarm_clock:CheckSchedule] Snooze until: {:?}", snooze_time);
                    if now >= snooze_time && !matches!(self.mode, ViewMode::Ringing { .. }) {
                        println!("[alarm_clock:CheckSchedule] Snooze time reached, triggering!");
                        self.snooze_until = None;
                        self.sync_wakeup_schedule();
                        return self.trigger_alarm(0, "稍后提醒".to_string(), 0);
                    }
                }

                // Check configured alarms
                let current_key = (now.ordinal(), now.hour() * 60 + now.minute());
                println!(
                    "[alarm_clock:CheckSchedule] current_key={:?}, last_triggered={:?}",
                    current_key, self.last_triggered_key
                );
                if self.last_triggered_key != Some(current_key) {
                    for alarm in &mut self.config.alarms {
                        let should = alarm.should_ring(now);
                        println!(
                            "[alarm_clock:CheckSchedule] Alarm id={} {}:{:02} enabled={} repeat={:?} -> should_ring={}",
                            alarm.id, alarm.hour, alarm.minute, alarm.enabled, alarm.repeat, should
                        );
                        if should {
                            self.last_triggered_key = Some(current_key);
                            let alarm_id = alarm.id;
                            let label = alarm.label.clone();
                            let sound_id = alarm.sound_id;

                            // When "永不" (Never) is selected, alarm disables after firing once
                            if alarm.repeat.is_never() {
                                alarm.enabled = false;
                                let _ = self.config.save();
                            }

                            self.sync_wakeup_schedule();
                            println!("[alarm_clock:CheckSchedule] TRIGGERING ALARM id={} label='{}'", alarm_id, label);
                            return self.trigger_alarm(alarm_id, label, sound_id);
                        }
                    }
                }
            }

            Message::RingTimeout { session_id } => {
                if let ViewMode::Ringing {
                    session_id: current_session_id,
                    ..
                } = self.mode
                {
                    if current_session_id == session_id {
                        self.sounds.stop(&self.board);
                        self.mode = ViewMode::List {
                            is_editing_list: false,
                        };
                        self.sync_wakeup_schedule();
                    }
                }
            }

            Message::TimeSynced => {
                if let Some(snooze) = self.snooze_until {
                    if Local::now() >= snooze {
                        self.snooze_until = None;
                    }
                }
                self.sync_wakeup_schedule();
            }

            Message::ToggleAlarm(id) => {
                self.config.toggle(id);
                self.sync_wakeup_schedule();
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
                    AlarmRepeat::never(),
                );
                self.mode = ViewMode::Edit {
                    is_new: true,
                    draft,
                    sub_sheet: None,
                };
            }

            Message::OpenEdit(id) => {
                self.stop_preview();
                if let Some(item) = self.config.get(id) {
                    self.mode = ViewMode::Edit {
                        is_new: false,
                        draft: item.clone(),
                        sub_sheet: None,
                    };
                }
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
                    item.enabled = true;
                    self.config.upsert(item);
                }
                self.sync_wakeup_schedule();
                self.mode = ViewMode::List {
                    is_editing_list: false,
                };
            }

            Message::DeleteAlarm(id) => {
                self.stop_preview();
                self.config.remove(id);
                self.sync_wakeup_schedule();
                if matches!(self.mode, ViewMode::Edit { .. }) {
                    self.mode = ViewMode::List {
                        is_editing_list: false,
                    };
                }
            }

            Message::HourChange(delta) => {
                self.step_hour(delta);
            }

            Message::MinuteChange(delta) => {
                self.step_minute(delta);
            }

            Message::HourDrag(delta_y) => {
                self.hour_drag_accum += delta_y;
                if self.hour_drag_accum.abs() >= 24.0 {
                    let step = if self.hour_drag_accum < 0.0 { 1 } else { -1 };
                    self.hour_drag_accum = 0.0;
                    self.step_hour(step);
                }
            }

            Message::HourDragEnd => {
                self.hour_drag_accum = 0.0;
            }

            Message::MinuteDrag(delta_y) => {
                self.minute_drag_accum += delta_y;
                if self.minute_drag_accum.abs() >= 24.0 {
                    let step = if self.minute_drag_accum < 0.0 { 1 } else { -1 };
                    self.minute_drag_accum = 0.0;
                    self.step_minute(step);
                }
            }

            Message::MinuteDragEnd => {
                self.minute_drag_accum = 0.0;
            }

            Message::OpenRoute(route) => {
                if let ViewMode::Edit { sub_sheet, .. } = &mut self.mode {
                    *sub_sheet = Some(route);
                }
            }

            Message::ToggleRepeatDay(day_idx) => {
                if let ViewMode::Edit { draft, .. } = &mut self.mode {
                    draft.repeat.toggle_day(day_idx);
                }
            }

            Message::SelectLabel(lbl) => {
                if let ViewMode::Edit { draft, .. } = &mut self.mode {
                    draft.label = lbl;
                }
            }

            Message::SelectSound(id) => {
                if let ViewMode::Edit { draft, .. } = &mut self.mode {
                    draft.sound_id = id;
                }
            }

            Message::SelectSnoozeMinutes(m) => {
                if let ViewMode::Edit { draft, .. } = &mut self.mode {
                    draft.snooze_minutes = m;
                }
            }

            Message::ToggleSnooze => {
                if let ViewMode::Edit { draft, .. } = &mut self.mode {
                    draft.snooze_enabled = !draft.snooze_enabled;
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
                self.sync_wakeup_schedule();
            }

            Message::SnoozeRinging => {
                self.sounds.stop(&self.board);
                let snooze_mins = if let Some(alarm) = self.config.get(self.ringing_alarm_id()) {
                    alarm.snooze_minutes as i64
                } else {
                    5
                };
                self.snooze_until = Some(Local::now() + chrono::Duration::minutes(snooze_mins));
                self.sync_wakeup_schedule();
                self.mode = ViewMode::List {
                    is_editing_list: false,
                };
            }

            Message::Back => {
                self.go_back();
            }
        }

        Task::none()
    }

    fn step_hour(&mut self, delta: i8) {
        if let ViewMode::Edit { draft, .. } = &mut self.mode {
            let new_h = (draft.hour as i16 + delta as i16).rem_euclid(24) as u8;
            draft.hour = new_h;
        }
    }

    fn step_minute(&mut self, delta: i8) {
        if let ViewMode::Edit { draft, .. } = &mut self.mode {
            let new_m = (draft.minute as i16 + delta as i16).rem_euclid(60) as u8;
            draft.minute = new_m;
        }
    }

    fn ringing_alarm_id(&self) -> u32 {
        if let ViewMode::Ringing { alarm_id, .. } = self.mode {
            alarm_id
        } else {
            0
        }
    }

    fn trigger_alarm(&mut self, alarm_id: u32, label: String, sound_id: u8) -> Task<Message> {
        self.next_session_id = self.next_session_id.wrapping_add(1);
        let session_id = self.next_session_id;
        let now = Local::now();
        let time_str = format!("{:02}:{:02}", now.hour(), now.minute());
        self.mode = ViewMode::Ringing {
            alarm_id,
            time_str,
            label,
            sound_id,
            session_id,
        };
        self.sounds.play_loop(&self.board, sound_id);

        Task::perform(
            async move {
                let (tx, rx) = iced::futures::channel::oneshot::channel();
                let _ = std::thread::Builder::new()
                    .name("alarm_timeout".into())
                    .spawn(move || {
                        std::thread::sleep(Duration::from_secs(60));
                        let _ = tx.send(());
                    });
                let _ = rx.await;
                session_id
            },
            move |session_id| Message::RingTimeout { session_id },
        )
    }

    fn stop_preview(&mut self) {
        if self.preview_playing {
            self.preview_playing = false;
            self.sounds.stop(&self.board);
        }
    }

    /// Application subscription (purely event-driven, no background periodic polling).
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    /// Main view dispatcher.
    pub fn view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = match &self.mode {
            ViewMode::List { is_editing_list } => {
                views::view_list(&self.config, *is_editing_list, &self.preferences)
            }
            ViewMode::Edit {
                is_new,
                draft,
                sub_sheet,
            } => match sub_sheet {
                None => views::view_edit_sheet(*is_new, draft, &self.preferences),
                Some(AlarmRoute::Repeat) => {
                    views::view_repeat_sheet(draft, &self.preferences)
                }
                Some(AlarmRoute::Label) => {
                    views::view_label_sheet(draft, &self.preferences)
                }
                Some(AlarmRoute::Sound) => {
                    views::view_sound_sheet(draft, &self.preferences)
                }
                Some(AlarmRoute::SnoozeDuration) => {
                    views::view_snooze_duration_sheet(draft, &self.preferences)
                }
            },
            ViewMode::Ringing {
                time_str, label, ..
            } => views::view_ringing(time_str, label, &self.preferences),
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
}

impl AppPreferences for AlarmClock {
    fn preferences(&self) -> SystemPreferences {
        self.preferences
    }

    fn set_preferences(&mut self, preferences: SystemPreferences) {
        self.preferences = preferences;
    }
}

impl KeepAwake for AlarmClock {
    fn keep_screen_on(&self) -> bool {
        matches!(self.mode, ViewMode::Ringing { .. })
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
