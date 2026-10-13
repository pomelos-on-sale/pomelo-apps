use std::sync::Arc;
use pomelo_hal::Board;
use wireless_clipboard::WirelessClipboard;

pub fn main() -> iced::Result {
    let board = Arc::new(Board::simulated());

    iced::application(
        move || WirelessClipboard::boot(Arc::clone(&board)),
        WirelessClipboard::update,
        WirelessClipboard::view,
    )
    .font(pomelo_material_symbols::FONT)
    .subscription(WirelessClipboard::subscription)
    .title("Wireless Clipboard")
    .window_size((480.0, 480.0))
    .theme(WirelessClipboard::theme)
    .run()
}
