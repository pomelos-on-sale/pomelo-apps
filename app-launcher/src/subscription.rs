//! Launcher subscriptions and hardware event monitoring.

use std::sync::Arc;
use iced::Subscription;
use pomelo_hal::Board;

use super::{Launcher, Message, Screen};
use crate::apps::AppId;

use chrono::{Local, Timelike};

/// Returns the current (formatted clock string, minute index in day) using system local time.
pub(crate) fn current_time_info() -> (String, u32) {
    let now = Local::now();
    let hours = now.hour();
    let minutes = now.minute();
    (format!("{:02}:{:02}", hours, minutes), hours * 60 + minutes)
}

#[derive(Clone)]
pub(crate) struct StatusSubscription {
    pub(crate) board: Arc<Board>,
}

impl std::hash::Hash for StatusSubscription {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        "app_launcher_status_subscription".hash(state);
    }
}

impl PartialEq for StatusSubscription {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.board, &other.board)
    }
}

impl Eq for StatusSubscription {}

pub(crate) fn status_stream(sub: &StatusSubscription) -> impl iced::futures::Stream<Item = Message> {
    let board = Arc::clone(&sub.board);
    let (mut tx, rx) = iced::futures::channel::mpsc::channel(16);

    // Send initial status on subscription creation
    let (clock, _) = current_time_info();
    let battery = board.power().battery_percent().unwrap_or(0);
    let charging = board.power().is_charging().unwrap_or(false);
    let wifi = board.wifi().status().signal_bars();
    let _ = tx.try_send(Message::Status(clock, battery, charging, wifi));

    // Reactive hardware event listener — zero new threads spawned in app-launcher!
    let tx_event = tx;
    let board_clone = Arc::clone(&board);
    let listener_id = board.on_event(move |event| {
        let mut tx = tx_event.clone();
        match event {
            pomelo_hal::SystemEvent::BatteryChanged {
                percent, charging, ..
            } => {
                let (clock, _) = current_time_info();
                let wifi = board_clone.wifi().status().signal_bars();
                let _ = tx.try_send(Message::Status(clock, *percent, *charging, wifi));
            }
            pomelo_hal::SystemEvent::WifiStatusChanged(wifi_status) => {
                let (clock, _) = current_time_info();
                let battery = board_clone.power().battery_percent().unwrap_or(0);
                let charging = board_clone.power().is_charging().unwrap_or(false);
                let _ = tx.try_send(Message::Status(
                    clock,
                    battery,
                    charging,
                    wifi_status.signal_bars(),
                ));
            }
            pomelo_hal::SystemEvent::TimeSynced { .. } => {
                let (clock, _) = current_time_info();
                let battery = board_clone.power().battery_percent().unwrap_or(0);
                let charging = board_clone.power().is_charging().unwrap_or(false);
                let wifi = board_clone.wifi().status().signal_bars();
                let _ = tx.try_send(Message::Status(clock, battery, charging, wifi));
            }
            pomelo_hal::SystemEvent::InputAction(action) => {
                let msg = match action {
                    pomelo_hal::InputAction::Back => Message::Back,
                    pomelo_hal::InputAction::Exit => Message::Exit,
                };
                let _ = tx.try_send(msg);
            }
            pomelo_hal::SystemEvent::SdCardChanged(_) => {
                let (clock, _) = current_time_info();
                let battery = board_clone.power().battery_percent().unwrap_or(0);
                let charging = board_clone.power().is_charging().unwrap_or(false);
                let wifi = board_clone.wifi().status().signal_bars();
                let _ = tx.try_send(Message::Status(clock, battery, charging, wifi));
            }
            pomelo_hal::SystemEvent::HttpFinished { .. } => {}
        }
    });

    struct ListenerGuard {
        board: Arc<Board>,
        id: u64,
    }
    impl Drop for ListenerGuard {
        fn drop(&mut self) {
            self.board.remove_listener(self.id);
        }
    }

    use iced::futures::StreamExt;
    let guard = Arc::new(ListenerGuard { board, id: listener_id });
    rx.map(move |item| {
        let _ = &guard;
        item
    })
}

impl Launcher {
    /// The app's subscriptions: the screen's size, keyboard shortcuts, and hosted app subscriptions.
    ///
    /// Background apps like `music-player` and `settings` (Wi-Fi scanning) keep their subscriptions
    /// alive in the background, while on-screen apps get their subscriptions.
    pub fn subscription(&self) -> Subscription<Message> {
        let resized = iced::window::resize_events().map(|(_window, size)| Message::Resized(size));

        let keyboard = iced::keyboard::listen().filter_map(|event| {
            if let iced::keyboard::Event::KeyPressed {
                key,
                modified_key,
                physical_key,
                ..
            } = event
            {
                if key.as_ref() == iced::keyboard::Key::Character("q")
                    || key.as_ref() == iced::keyboard::Key::Character("Q")
                    || modified_key.as_ref() == iced::keyboard::Key::Character("q")
                    || modified_key.as_ref() == iced::keyboard::Key::Character("Q")
                    || matches!(
                        physical_key,
                        iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::KeyQ)
                    )
                {
                    return Some(Message::Back);
                }

                if key.as_ref() == iced::keyboard::Key::Character("e")
                    || key.as_ref() == iced::keyboard::Key::Character("E")
                    || modified_key.as_ref() == iced::keyboard::Key::Character("e")
                    || modified_key.as_ref() == iced::keyboard::Key::Character("E")
                    || matches!(
                        physical_key,
                        iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::KeyE)
                    )
                {
                    return Some(Message::Exit);
                }
            }
            None
        });

        let mut subs = vec![
            resized,
            keyboard,
            Subscription::run_with(
                StatusSubscription {
                    board: Arc::clone(&self.board),
                },
                status_stream,
            ),
        ];

        match self.screen {
            Screen::App(AppId::DemoHello) => {
                if let Some(hello) = &self.hello {
                    subs.push(hello.subscription().map(Message::Hello));
                }
            }
            Screen::App(AppId::Terminal) => {
                if let Some(terminal) = &self.terminal {
                    subs.push(terminal.subscription().map(Message::Terminal));
                }
            }
            Screen::App(AppId::DailySchedule) => {
                if let Some(daily_schedule) = &self.daily_schedule {
                    subs.push(daily_schedule.subscription().map(Message::DailySchedule));
                }
            }
            Screen::App(AppId::WirelessClipboard) => {
                if let Some(wireless_clipboard) = &self.wireless_clipboard {
                    subs.push(wireless_clipboard.subscription().map(Message::WirelessClipboard));
                }
            }
            _ => {}
        }

        if self.running_apps.contains(&AppId::Music) {
            if let Some(music) = &self.music {
                subs.push(music.subscription().map(Message::Music));
            }
        }
        if self.running_apps.contains(&AppId::Settings) {
            if let Some(settings) = &self.settings {
                subs.push(settings.subscription().map(Message::Settings));
            }
        }

        Subscription::batch(subs)
    }
}
