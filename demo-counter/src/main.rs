//! The counter, as a program you can run.
//!
//! ```text
//! cargo run                      # from this directory: iced's real `iced_winit`, and a window
//! ```
//!
//! The whole of it: iced builds the state with `Default`, then owns the loop.

use demo_counter::DemoCounter;

fn main() -> iced::Result {
    iced::application(DemoCounter::new, DemoCounter::update, DemoCounter::view)
        .theme(DemoCounter::theme)
        .run()
}
