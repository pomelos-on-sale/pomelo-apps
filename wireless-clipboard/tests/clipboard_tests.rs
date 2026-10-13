use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use pomelo_hal::Board;
use wireless_clipboard::{Message, WirelessClipboard};

#[test]
fn wireless_clipboard_initialization_and_clear() {
    let board = Arc::new(Board::simulated());
    let mut app = WirelessClipboard::new(board);

    pomelo_widgets::clipboard::write("initial test");
    let _ = app.update(Message::Tick);
    assert_eq!(pomelo_widgets::clipboard::read(), "initial test");

    let _ = app.update(Message::Clear);
    assert_eq!(pomelo_widgets::clipboard::read(), "");
}

#[test]
fn wireless_clipboard_http_server_sync() {
    let board = Arc::new(Board::simulated());
    let mut app = WirelessClipboard::new(board);
    let port = app.port();

    // Allow HTTP server thread to bind port
    thread::sleep(Duration::from_millis(100));

    // Connect to local HTTP server and post data
    let mut stream =
        TcpStream::connect(format!("127.0.0.1:{port}")).expect("connect to clipboard server");
    let payload = "{\"content\":\"webcal://p20-caldav.icloud.com/published/2/abc\"}";
    let req = format!(
        "POST /api/clipboard HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n\
         {payload}",
        payload.len()
    );
    stream.write_all(req.as_bytes()).unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.contains("200 OK"));
    assert!(response.contains("\"ok\":true"));

    // Tick app to consume notify channel
    thread::sleep(Duration::from_millis(50));
    let _ = app.update(Message::Tick);

    assert_eq!(
        pomelo_widgets::clipboard::read(),
        "webcal://p20-caldav.icloud.com/published/2/abc"
    );
}
