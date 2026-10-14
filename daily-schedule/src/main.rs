use daily_schedule::DailySchedule;

pub fn main() -> iced::Result {
    iced::application(DailySchedule::boot, DailySchedule::update, DailySchedule::view)
        .font(pomelo_material_symbols::FONT)
        .subscription(DailySchedule::subscription)
        .title("Daily Schedule")
        .window_size((480.0, 480.0))
        .theme(DailySchedule::theme)
        .run()
}
