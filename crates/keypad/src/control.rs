//! Control channel: JSON lines over TCP on 127.0.0.1, one command per line.
//!
//! Request: `{"cmd":"mapping.set","params":{...}}`.
//! Reply: `{"ok":true,"result":...}` or `{"ok":false,"error":"..."}`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use keypad_engine::Engine;
use serde_json::{Value, json};

/// Longest accepted request line, in bytes.
pub const MAX_LINE: usize = 65536;

/// Runs one request line against the engine and returns the JSON reply (no newline).
#[cfg(test)]
pub fn handle_line(engine: &mut Engine, line: &str) -> String {
    handle(engine, line).0
}

/// Runs one line. The flag is true when the line was not a request at all (too long, not
/// JSON, no `cmd`): the session then ends, so a web page posting to this port cannot slip a
/// command in after its HTTP header lines.
fn handle(engine: &mut Engine, line: &str) -> (String, bool) {
    let err = |msg: String| json!({ "ok": false, "error": msg }).to_string();
    if line.len() > MAX_LINE {
        return (err(format!("request longer than {MAX_LINE} bytes")), true);
    }
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return (err(format!("invalid JSON: {e}")), true),
    };
    let Some(cmd) = req.get("cmd").and_then(Value::as_str) else {
        return (err("missing string field `cmd`".into()), true);
    };
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));
    match engine.run(cmd, params) {
        Ok(result) => (json!({ "ok": true, "result": result }).to_string(), false),
        Err(e) => (err(e.to_string()), false),
    }
}

/// Reads request lines until the end of the stream or the first line that is not a request.
fn serve_with<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    mut run: impl FnMut(&str) -> (String, bool),
) -> std::io::Result<()> {
    while let Some(line) = read_line_capped(reader)? {
        if line.is_empty() {
            continue;
        }
        let (reply, close) = run(&line);
        writer.write_all(reply.as_bytes())?;
        writer.write_all(b"\n")?;
        if close {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
fn serve_lines<R: BufRead, W: Write>(
    engine: &mut Engine,
    reader: &mut R,
    writer: &mut W,
) -> std::io::Result<()> {
    serve_with(reader, writer, |line| handle(engine, line))
}

/// Reads one line of at most `MAX_LINE` bytes; `None` at end of stream.
fn read_line_capped<R: BufRead>(r: &mut R) -> std::io::Result<Option<String>> {
    let mut buf = Vec::new();
    let n = r
        .by_ref()
        .take(MAX_LINE as u64 + 1)
        .read_until(b'\n', &mut buf)?;
    if n == 0 {
        return Ok(None);
    }
    if buf.len() > MAX_LINE && buf.last() != Some(&b'\n') {
        // Skip the rest of the oversized line.
        let mut rest = Vec::new();
        r.read_until(b'\n', &mut rest)?;
    }
    Ok(Some(String::from_utf8_lossy(&buf).trim_end().to_string()))
}

fn serve_client(stream: TcpStream, engine: &Arc<Mutex<Engine>>) -> std::io::Result<()> {
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    serve_with(&mut reader, &mut writer, |line| {
        let mut e = engine.lock().unwrap_or_else(|e| e.into_inner());
        handle(&mut e, line)
    })
}

/// Listens on `127.0.0.1:port` in background threads.
pub fn spawn(port: u16, engine: Arc<Mutex<Engine>>) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let engine = engine.clone();
            std::thread::spawn(move || {
                if let Err(e) = serve_client(stream, &engine) {
                    eprintln!("control: client error: {e}");
                }
            });
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use keypad_engine::NullBackend;
    use proptest::prelude::*;
    use serde_json::Value;

    fn engine() -> Engine {
        Engine::new(Box::new(NullBackend), None)
    }

    fn reply(line: &str) -> Value {
        serde_json::from_str(&handle_line(&mut engine(), line)).unwrap()
    }

    #[test]
    fn valid_command_is_ok() {
        let r = reply(r#"{"cmd":"mode.toggle","params":{}}"#);
        assert_eq!(r["ok"], true);
        assert_eq!(r["result"]["enabled"], false);
    }

    #[test]
    fn missing_params_defaults_to_empty() {
        assert_eq!(reply(r#"{"cmd":"mode.toggle"}"#)["ok"], true);
    }

    #[test]
    fn command_error_is_reported() {
        let r = reply(r#"{"cmd":"nope","params":{}}"#);
        assert_eq!(r["ok"], false);
        assert!(r["error"].as_str().unwrap().contains("nope"));
    }

    #[test]
    fn invalid_json_is_not_ok() {
        assert_eq!(reply("{not json")["ok"], false);
    }

    #[test]
    fn oversized_line_is_not_ok() {
        let line = format!(r#"{{"cmd":"mode.toggle","pad":"{}"}}"#, "x".repeat(70_000));
        assert_eq!(reply(&line)["ok"], false);
    }

    // Final review #8: a browser can POST to 127.0.0.1; the HTTP preamble must end the session
    // before any body line runs.
    #[test]
    fn http_preamble_closes_the_connection() {
        let mut e = engine();
        let input = "POST / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n{\"cmd\":\"app.quit\"}\n";
        let mut out = Vec::new();
        serve_lines(
            &mut e,
            &mut std::io::Cursor::new(input.as_bytes()),
            &mut out,
        )
        .unwrap();
        assert!(
            e.take_requests().is_empty(),
            "a command ran after an invalid line"
        );
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 1, "{text}");
    }

    #[test]
    fn valid_lines_keep_the_connection_open() {
        let mut e = engine();
        let input = "{\"cmd\":\"mode.toggle\"}\n{\"cmd\":\"app.quit\"}\n";
        let mut out = Vec::new();
        serve_lines(
            &mut e,
            &mut std::io::Cursor::new(input.as_bytes()),
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap().lines().count(), 2);
        assert!(!e.take_requests().is_empty());
    }

    proptest! {
        #[test]
        fn any_line_gets_a_json_reply(s in ".{0,300}") {
            let out = handle_line(&mut engine(), &s);
            prop_assert!(serde_json::from_str::<Value>(&out).is_ok());
        }
    }
}
