//! Daily Schedule application: daily time display and interactive scrollable timeline canvas.

pub mod app;
pub mod digital_clock_layer;
pub mod graphic_clock_layer;
pub mod ical;
pub mod model;
pub mod ringing;
pub mod schedule_layer;
pub mod sound;
pub mod theme;
pub mod timeline;

pub use app::{App, DailySchedule, Message, SyncStatus};
pub use model::{ScheduleConfig, ScheduleEvent, TodayCache};
pub use sound::ScheduleSounds;
pub use timeline::Timeline;
