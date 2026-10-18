use alarm_clock::AlarmClock;

pub fn main() -> iced::Result {
    iced::application(AlarmClock::boot, AlarmClock::update, AlarmClock::view)
        .font(pomelo_material_symbols::FONT)
        .subscription(AlarmClock::subscription)
        .title("Alarm Clock")
        .window_size((480.0, 480.0))
        .theme(AlarmClock::theme)
        .run()
}
