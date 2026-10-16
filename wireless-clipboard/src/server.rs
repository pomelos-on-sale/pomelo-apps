//! Lightweight HTTP server for wireless clipboard sync.
//!
//! Provides:
//! - `GET /`: A mobile-friendly responsive web page for entering text.
//! - `GET /api/clipboard`: Returns current clipboard text in JSON format.
//! - `POST /api/clipboard`: Receives text from phone and stores into system clipboard.
//! - `POST /api/clear`: Clears system clipboard.
//! - `OPTIONS *`: CORS preflight responses.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct ClipboardPayload {
    content: String,
}

#[derive(Debug, Serialize)]
struct ApiResponse<'a> {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cleared: Option<bool>,
}

pub struct HttpServer {
    running: Arc<AtomicBool>,
    buffer: Arc<RwLock<String>>,
}

impl HttpServer {
    pub fn start(preferred_port: u16, notify_tx: Sender<String>) -> std::io::Result<(Self, u16)> {
        let listener = match TcpListener::bind(("0.0.0.0", preferred_port)) {
            Ok(l) => l,
            Err(_) => TcpListener::bind(("0.0.0.0", 0))?,
        };
        let bound_port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let buffer = Arc::new(RwLock::new(String::new()));
        let buffer_clone = Arc::clone(&buffer);

        thread::Builder::new()
            .name("clipboard-httpd".into())
            .spawn(move || {
                while running_clone.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _addr)) => {
                            handle_connection(stream, &notify_tx, &buffer_clone);
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(50));
                        }
                        Err(_) => {
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                }
            })?;

        Ok((Self { running, buffer }, bound_port))
    }

    pub fn set_text(&self, text: &str) {
        if let Ok(mut guard) = self.buffer.write() {
            guard.clear();
            guard.push_str(text);
        }
    }

    pub fn get_text(&self) -> String {
        self.buffer.read().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn handle_connection(mut stream: TcpStream, notify_tx: &Sender<String>, clipboard_buf: &Arc<RwLock<String>>) {
    // Crucial for lwIP: an accepted socket may inherit non-blocking status.
    // Reset to blocking mode so timeouts take effect properly.
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(2000)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(2000)));
    let _ = stream.set_nodelay(true);

    // Allocate buffer on heap to prevent FreeRTOS thread stack overflow
    let mut buffer = vec![0u8; 4096];
    let mut total_read = 0;

    // Read HTTP headers
    loop {
        if total_read >= buffer.len() {
            break;
        }
        match stream.read(&mut buffer[total_read..]) {
            Ok(0) => break,
            Ok(n) => {
                total_read += n;
                if buffer[..total_read].windows(4).any(|w| w == b"\r\n\r\n")
                    || buffer[..total_read].windows(2).any(|w| w == b"\n\n")
                {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if total_read == 0 {
        return;
    }

    let header_end_pos = if let Some(p) = buffer[..total_read].windows(4).position(|w| w == b"\r\n\r\n") {
        p + 4
    } else if let Some(p) = buffer[..total_read].windows(2).position(|w| w == b"\n\n") {
        p + 2
    } else {
        total_read
    };

    let (method, path, content_length) = {
        let header_str = String::from_utf8_lossy(&buffer[..header_end_pos]);
        let mut lines = header_str.lines();
        let request_line = lines.next().unwrap_or("");
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("/").to_string();

        let mut content_length = 0usize;
        for line in lines {
            let lower = line.to_ascii_lowercase();
            if let Some(val) = lower.strip_prefix("content-length:") {
                if let Ok(len) = val.trim().parse::<usize>() {
                    content_length = len.min(65536);
                }
            }
        }
        (method, path, content_length)
    };

    // Handle CORS preflight
    if method == "OPTIONS" {
        send_response(&mut stream, 204, "No Content", "text/plain", "");
        return;
    }

    // Ignore mobile browser icon requests
    if path == "/favicon.ico" || path.starts_with("/apple-touch-icon") {
        send_response(&mut stream, 404, "Not Found", "text/plain", "Not Found");
        return;
    }

    // Read remaining body if not fully received in header chunk
    let body_bytes_read = total_read.saturating_sub(header_end_pos);
    if content_length > body_bytes_read && content_length <= 65536 {
        let needed_total = header_end_pos + content_length;
        if buffer.len() < needed_total {
            buffer.resize(needed_total, 0);
        }
        while total_read < needed_total {
            match stream.read(&mut buffer[total_read..needed_total]) {
                Ok(0) => break,
                Ok(n) => {
                    total_read += n;
                }
                Err(_) => break,
            }
        }
    }

    let body_str = String::from_utf8_lossy(&buffer[header_end_pos..total_read]);

    if method == "GET" && (path == "/" || path == "/index.html") {
        let current_text = clipboard_buf.read().map(|g| g.clone()).unwrap_or_default();
        let body = render_web_page(&current_text);
        send_response(&mut stream, 200, "OK", "text/html; charset=utf-8", &body);
    } else if method == "GET" && path == "/api/clipboard" {
        let current_text = clipboard_buf.read().map(|g| g.clone()).unwrap_or_default();
        let resp = ApiResponse {
            ok: true,
            content: Some(&current_text),
            cleared: None,
        };
        let body = serde_json::to_string(&resp).unwrap_or_else(|_| "{\"ok\":false}".into());
        send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", &body);
    } else if method == "POST" && (path == "/api/clipboard" || path == "/send") {
        let content = parse_content_payload(&body_str);
        if !content.is_empty() {
            if let Ok(mut guard) = clipboard_buf.write() {
                guard.clear();
                guard.push_str(&content);
            }
            let _ = notify_tx.send(content.clone());
        }
        let resp = ApiResponse {
            ok: true,
            content: Some(&content),
            cleared: None,
        };
        let body = serde_json::to_string(&resp).unwrap_or_else(|_| "{\"ok\":false}".into());
        send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", &body);
    } else if method == "POST" && path == "/api/clear" {
        if let Ok(mut guard) = clipboard_buf.write() {
            guard.clear();
        }
        let _ = notify_tx.send(String::new());
        let resp = ApiResponse {
            ok: true,
            content: None,
            cleared: Some(true),
        };
        let body = serde_json::to_string(&resp).unwrap_or_else(|_| "{\"ok\":false}".into());
        send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", &body);
    } else {
        send_response(&mut stream, 404, "Not Found", "text/plain", "Not Found");
    }
}

fn parse_content_payload(raw: &str) -> String {
    let trimmed = raw.trim();

    // 1. Try parsing JSON: {"content": "..."}
    if let Ok(payload) = serde_json::from_str::<ClipboardPayload>(trimmed) {
        return payload.content;
    }

    // 2. Fallback to form-urlencoded: content=...
    if let Some(encoded) = trimmed.strip_prefix("content=") {
        return url_decode(encoded);
    }

    // 3. Fallback to raw string
    raw.to_string()
}

fn url_decode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(val);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            result.push(b' ');
        } else {
            result.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn send_response(stream: &mut TcpStream, code: u16, status: &str, content_type: &str, body: &str) {
    let header = format!(
        "HTTP/1.1 {code} {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    if !body.is_empty() {
        let _ = stream.write_all(body.as_bytes());
    }
    let _ = stream.flush();
}

fn render_web_page(_current_clipboard: &str) -> String {
    r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
<title>🍊 Pomelo 无线剪贴板</title>
<style>
  :root {
    --bg: #0d1117;
    --card: #161b22;
    --border: #30363d;
    --accent: #58a6ff;
    --accent-hover: #79c0ff;
    --text: #f0f6fc;
    --text-dim: #8b949e;
    --success: #3fb950;
    --btn-sec-bg: #21262d;
    --btn-sec-hover: #30363d;
  }
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }
  body {
    background-color: var(--bg);
    color: var(--text);
    padding: 24px 16px;
    display: flex;
    justify-content: center;
  }
  .container {
    max-width: 480px;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .header {
    text-align: center;
    padding-bottom: 4px;
  }
  .header h1 {
    font-size: 22px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }
  .toast {
    position: fixed;
    top: 24px;
    left: 50%;
    transform: translate(-50%, -24px);
    opacity: 0;
    pointer-events: none;
    border-radius: 12px;
    padding: 12px 22px;
    font-size: 14px;
    font-weight: 500;
    text-align: center;
    border: 1px solid var(--success);
    background: rgba(22, 27, 34, 0.92);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    color: var(--success);
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.45);
    z-index: 1000;
    transition: opacity 0.25s ease, transform 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    white-space: nowrap;
    max-width: 90vw;
  }
  .toast.show {
    opacity: 1;
    transform: translate(-50%, 0);
    pointer-events: auto;
  }
  .card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .card-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-dim);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  textarea {
    width: 100%;
    min-height: 140px;
    background: #090d13;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 12px;
    color: var(--text);
    font-size: 15px;
    line-height: 1.4;
    resize: vertical;
    outline: none;
    transition: border-color 0.2s;
  }
  textarea:focus {
    border-color: var(--accent);
  }
  .btn-row {
    display: flex;
    gap: 12px;
  }
  button {
    flex: 1;
    padding: 12px;
    border-radius: 10px;
    border: none;
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    outline: none;
  }
  .btn-primary {
    background: var(--accent);
    color: #0d1117;
  }
  .btn-primary:hover {
    background: var(--accent-hover);
  }
  .btn-primary:active {
    transform: scale(0.98);
  }
  .btn-secondary {
    background: var(--btn-sec-bg);
    color: var(--text);
    border: 1px solid var(--border);
  }
  .btn-secondary:hover {
    background: var(--btn-sec-hover);
  }
  .btn-secondary:active {
    transform: scale(0.98);
  }
  .footer-tip {
    font-size: 12px;
    color: var(--text-dim);
    text-align: center;
    line-height: 1.5;
  }
</style>
</head>
<body>
<div id="toast" class="toast"></div>
<div class="container">
  <div class="header">
    <h1>🍊 Pomelo 无线剪贴板</h1>
  </div>

  <div class="card">
    <div class="card-title">
      <span>剪贴板文本</span>
      <span id="charCount" style="font-size: 12px;">0 字</span>
    </div>
    <textarea id="inputText" placeholder="在此输入或粘贴文本..."></textarea>
    <div class="btn-row">
      <button class="btn-primary" onclick="sendClipboard()">发送</button>
      <button class="btn-secondary" onclick="pullClipboard()">拉取</button>
    </div>
  </div>

  <div class="footer-tip">
    💡 提示：在设备的屏幕键盘上点击 📋 图标即可一键粘贴！
  </div>
</div>

<script>
  const input = document.getElementById('inputText');
  const count = document.getElementById('charCount');
  const toast = document.getElementById('toast');

  input.addEventListener('input', () => {
    count.innerText = input.value.length + ' 字';
  });

  let toastTimer = null;
  function showToast(msg, isSuccess = true) {
    if (toastTimer) clearTimeout(toastTimer);
    toast.innerText = msg;
    const color = isSuccess ? 'var(--success)' : 'var(--accent)';
    toast.style.borderColor = color;
    toast.style.color = color;
    toast.classList.add('show');
    toastTimer = setTimeout(() => {
      toast.classList.remove('show');
    }, 2800);
  }

  async function sendClipboard() {
    const val = input.value;
    if (!val) {
      showToast('⚠️ 发送内容不能为空', false);
      return;
    }
    try {
      const resp = await fetch('/api/clipboard', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content: val })
      });
      if (resp.ok) {
        showToast('✅ 已成功发送到设备！', true);
      } else {
        alert('发送失败: HTTP ' + resp.status);
      }
    } catch (e) {
      alert('发送失败，请确保手机与设备连接在同一 Wi-Fi');
    }
  }

  async function pullClipboard() {
    try {
      const resp = await fetch('/api/clipboard');
      if (resp.ok) {
        const data = await resp.json();
        const text = data.content || '';
        input.value = text;
        count.innerText = text.length + ' 字';
        if (text) {
          showToast('📥 已成功拉取设备剪贴板！', true);
          if (navigator.clipboard && navigator.clipboard.writeText) {
            navigator.clipboard.writeText(text).catch(() => {});
          }
        } else {
          showToast('ℹ️ 设备剪贴板为空', false);
        }
      } else {
        alert('拉取失败: HTTP ' + resp.status);
      }
    } catch (e) {
      alert('拉取失败，请确保手机与设备连接在同一 Wi-Fi');
    }
  }
</script>
</body>
</html>"#.to_string()
}

#[allow(dead_code)]
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_payload_json() {
        // Standard JSON
        assert_eq!(
            parse_content_payload(r#"{"content":"hello world"}"#),
            "hello world"
        );

        // JSON with spaces and newlines
        assert_eq!(
            parse_content_payload("{\n  \"content\" : \"spaces and tabs\"\n}"),
            "spaces and tabs"
        );

        // JSON with escaped characters, quotes, and newlines
        assert_eq!(
            parse_content_payload(r#"{"content":"line1\nline2 with \"quotes\" and \\slash"}"#),
            "line1\nline2 with \"quotes\" and \\slash"
        );

        // JSON with Unicode and emojis
        assert_eq!(
            parse_content_payload(r#"{"content":"\u4e2d\u6587 \ud83d\ude00"}"#),
            "中文 😀"
        );
    }

    #[test]
    fn test_parse_content_payload_fallback() {
        // Form urlencoded fallback
        assert_eq!(
            parse_content_payload("content=hello+world%21"),
            "hello world!"
        );

        // Plain raw text fallback
        assert_eq!(
            parse_content_payload("plain unencoded text"),
            "plain unencoded text"
        );
    }

    #[test]
    fn test_api_response_serialization() {
        let resp = ApiResponse {
            ok: true,
            content: Some("test \"quote\" & \n newline"),
            cleared: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert_eq!(json, r#"{"ok":true,"content":"test \"quote\" & \n newline"}"#);

        let clear_resp = ApiResponse {
            ok: true,
            content: None,
            cleared: Some(true),
        };
        let clear_json = serde_json::to_string(&clear_resp).unwrap();
        assert_eq!(clear_json, r#"{"ok":true,"cleared":true}"#);
    }
}

