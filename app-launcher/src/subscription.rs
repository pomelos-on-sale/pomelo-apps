//! Launcher subscriptions and hardware event monitoring.

use std::sync::Arc;
use iced::Subscription;
use pomelo_hal::Board;

use super::{Launcher, Message, Screen, HELLO, MUSIC, SETTINGS, TERMINAL};

/// Returns the current (formatted clock string, minute index in day).
pub(crate) fn current_time_info() -> (String, u32) {
    if let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        let total_secs = duration.as_secs();
        // UTC+8 offset (China Standard Time / Beijing Time: 8 hours = 28,800 seconds)
        let local_secs = total_secs + 28800;
        let day_secs = local_secs % 86400;
        let hours = (day_secs / 3600) as u32;
        let minutes = ((day_secs % 3600) / 60) as u32;
        (format!("{:02}:{:02}", hours, minutes), hours * 60 + minutes)
    } else {
        ("00:00".to_string(), 0)
    }
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
    board.on_event(move |event| {
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
        }
    });

    rx
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

                if key.as_ref() == iced::keyboard::Key::Character("w")
                    || key.as_ref() == iced::keyboard::Key::Character("W")
                    || modified_key.as_ref() == iced::keyboard::Key::Character("w")
                    || modified_key.as_ref() == iced::keyboard::Key::Character("W")
                    || matches!(
                        physical_key,
                        iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::KeyW)
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
            Screen::App(HELLO) => {
                if let Some(hello) = &self.hello {
                    subs.push(hello.subscription().map(Message::Hello));
                }
            }
            Screen::App(TERMINAL) => {
                if let Some(terminal) = &self.terminal {
                    subs.push(terminal.subscription().map(Message::Terminal));
                }
            }
            _ => {}
        }

        if self.running_apps.contains(&MUSIC) {
            if let Some(music) = &self.music {
                subs.push(music.subscription().map(Message::Music));
            }
        }
        if self.running_apps.contains(&SETTINGS) {
            if let Some(settings) = &self.settings {
                subs.push(settings.subscription().map(Message::Settings));
            }
        }

        Subscription::batch(subs)
    }
}
