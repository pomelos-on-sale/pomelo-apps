//! Modular view components for the Alarm Clock application.

pub mod edit_sheet;
pub mod list;
pub mod ringing;
pub mod selection_sheets;
pub mod wheel_picker;

pub use edit_sheet::view_edit_sheet;
pub use list::{render_alarm_card, view_list};
pub use ringing::view_ringing;
pub use selection_sheets::{
    view_label_sheet, view_repeat_sheet, view_snooze_duration_sheet, view_sound_sheet,
};
pub use wheel_picker::render_wheel_pickers;
