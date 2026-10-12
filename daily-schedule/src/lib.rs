//! Daily Schedule application: daily time display and interactive scrollable timeline canvas.

pub mod app;
pub mod theme;
pub mod time_display;
pub mod timeline;

pub use app::{App, DailySchedule, Message};
pub use timeline::Timeline;
