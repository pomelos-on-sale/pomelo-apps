//! The settings, as a program you can run.
//!
//! ```text
//! cargo run                      # from this directory: iced's real `iced_winit`, and a window
//! ```
//!
//! iced builds the state with `Settings::new`, then owns the loop. The board is the HAL's desktop
//! simulator — the firmware injects the ESP32-S3 one into the same `Settings::new` — so the Wi-Fi
//! page has a radio to talk to here: five access points, a scan that takes three ticks, and a
//! connection that comes up as soon as the password is accepted.

use std::sync::Arc;
use std::time::Duration;

use iced::Subscription;
use pomelo_hal::Board;
use settings::{Message, Settings};

fn main() -> iced::Result {
    let board = Arc::new(Board::simulated());

    // The simulator only advances when somebody calls `tick`, and a desktop run has no firmware loop
    // to do it: this thread is the platform here, the way `rust_main`'s loop is on the board. The
    // device's own backends are event-driven and their `tick` is a no-op, so this is a desktop-only
    // concern — which is why it lives in `main` and not in the app.
    {
        let board = Arc::clone(&board);

        std::thread::spawn(move || loop {
            board.tick();
            std::thread::sleep(Duration::from_millis(10));
        });
    }

    iced::application(
        move || Settings::new(Arc::clone(&board)),
        Settings::update,
        Settings::view,
    )
    .theme(Settings::theme)
    .subscription(|settings| {
        Subscription::batch([
            settings.subscription(),
            iced::keyboard::listen().filter_map(|event| {
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
                }
                None
            }),
        ])
    })
    .run()
}

