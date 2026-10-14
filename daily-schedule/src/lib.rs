//! Daily Schedule application: daily time display and interactive scrollable timeline canvas.

pub mod app;
pub mod ical;
pub mod model;
pub mod theme;
pub mod time_display;
pub mod timeline;

pub use app::{App, DailySchedule, Message, SyncStatus};
pub use model::{ScheduleConfig, ScheduleEvent, TodayCache};
pub use timeline::Timeline;
