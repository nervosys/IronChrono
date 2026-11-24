//! Model Context Protocol server over stdio: JSON-RPC 2.0, one message per line.
//!
//! A line is at most `MAX_LINE` bytes; `id` is a string of at most 256 characters, an integer
//! or null; `jsonrpc` must be exactly "2.0". `tools/call` is served before `initialize`
//! (lenient on purpose: every tool is stateless).

use crate::output::{CliError, clip, guard};
use crate::spec::{self, Args};
use serde_json::{Value, json};
use std::io::{BufRead, Write};

pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// Longest accepted request line, in bytes (excluding the newline).
pub const MAX_LINE: usize = 1 << 20;
/// Longest accepted string `id`, in characters.
const MAX_ID_CHARS: usize = 256;

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Run one tool call; the structured value is the same JSON the CLI prints.
fn call_tool(name: &str, arguments: &Value) -> Value {
    let outcome = guard(|| {
        let cmd = spec::find(name).ok_or_else(|| {
            CliError::usage(format!("unknown tool {:?}", clip(name)), "See tools/list.")
        })?;
        let mut args = Args::new();
        match arguments {
            Value::Null => {}
            Value::Object(m) => {
                for (k, v) in m {
                    let s = match v {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => {
                            return Err(CliError::usage(
                                format!("argument {:?} must be a string", clip(k)),
                                "All tool arguments are strings.",
                            ));
                        }
                    };
                    args.insert(k.clone(), s);
                }
            }
            _ => {
                return Err(CliError::usage(
                    "arguments must be an object",
                    "Pass {\"input\": \"...\"}.",
                ));
            }
        }
        spec::validate(cmd, &args)?;
        crate::commands::run(name, &args)
    });
    let (structured, is_error) = match outcome {
        Ok(v) => (v, false),
        Err(e) => (e.to_json(), true),
    };
    json!({
        "content": [{"type": "text", "text": structured.to_string()}],
        "structuredContent": structured,
        "isError": is_error
    })
}

fn valid_id(id: &Value) -> bool {
    match id {
        Value::Null => true,
        Value::Number(_) => id.is_i64() || id.is_u64(),
        Value::String(s) => s.chars().count() <= MAX_ID_CHARS,
        _ => false,
    }
}

/// Handle one line; `None` means no response (a notification).
pub fn handle_line(line: &str) -> Option<Value> {
    let mut msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return Some(rpc_error(Value::Null, -32700, "Parse error")),
    };
    let Some(o) = msg.as_object_mut() else {
        return Some(rpc_error(Value::Null, -32600, "Invalid Request"));
    };
    let id = o.get("id").cloned();
    if id.as_ref().is_some_and(|i| !valid_id(i)) {
        return Some(rpc_error(Value::Null, -32600, "Invalid Request: bad id"));
    }
    if o.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Some(rpc_error(
            id.unwrap_or(Value::Null),
            -32600,
            "Invalid Request: jsonrpc must be \"2.0\"",
        ));
    }
    let Some(method) = o.get("method").and_then(Value::as_str).map(str::to_owned) else {
        return Some(rpc_error(id.unwrap_or(Value::Null), -32600, "Invalid Request"));
    };
    // No id: a notification, never answered.
    let id = id?;
    let params = o.remove("params").unwrap_or(Value::Null);
    Some(match method.as_str() {
        "initialize" => rpc_result(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "serverInfo": {"name": "ironchrono", "version": env!("CARGO_PKG_VERSION")},
                "capabilities": {"tools": {}},
                "instructions": "Time for agents. Every tool returns JSON; pass `now` for reproducible results. Errors carry code, message and hint."
            }),
        ),
        "ping" => rpc_result(id, json!({})),
        "tools/list" => rpc_result(id, json!({"tools": spec::tools()})),
        "tools/call" => {
            let Some(name) = params.get("name").and_then(Value::as_str) else {
                return Some(rpc_error(id, -32602, "Invalid params: name is required"));
            };
            let known = spec::tools().iter().any(|t| t["name"] == name);
            if !known {
                return Some(rpc_error(id, -32602, &format!("Unknown tool: {}", clip(name))));
            }
            rpc_result(id, call_tool(name, params.get("arguments").unwrap_or(&Value::Null)))
        }
        _ => rpc_error(id, -32601, &format!("Method not found: {}", clip(&method))),
    })
}

/// One request line read under the size cap.
#[derive(Debug, PartialEq, Eq)]
pub enum Read {
    Eof,
    Got,
    TooLong,
}

/// Read up to the next newline into `buf` (cleared first), keeping at most `max` bytes. Past the
/// cap the rest of the line is discarded as it arrives, so memory stays bounded.
pub fn read_line_bounded<R: BufRead>(
    r: &mut R,
    buf: &mut Vec<u8>,
    max: usize,
) -> std::io::Result<Read> {
    buf.clear();
    let mut over = false;
    let mut any = false;
    loop {
        let chunk = r.fill_buf()?;
        if chunk.is_empty() {
            return Ok(if over {
                Read::TooLong
            } else if any {
                Read::Got
            } else {
                Read::Eof
            });
        }
        any = true;
        let (take, found) = match chunk.iter().position(|&b| b == b'\n') {
            Some(p) => (p, true),
            None => (chunk.len(), false),
        };
        if !over {
            if buf.len() + take > max {
                over = true;
                buf.clear();
                buf.shrink_to(0);
            } else {
                buf.extend_from_slice(&chunk[..take]);
            }
        }
        r.consume(take + usize::from(found));
        if found {
            return Ok(if over { Read::TooLong } else { Read::Got });
        }
    }
}

fn reply(resp: &Value) -> bool {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{resp}").and_then(|()| out.flush()).is_ok()
}

pub fn serve() -> i32 {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut buf = Vec::new();
    loop {
        let resp = match read_line_bounded(&mut input, &mut buf, MAX_LINE) {
            Err(_) => return 1,
            Ok(Read::Eof) => return 0,
            Ok(Read::TooLong) => {
                Some(rpc_error(Value::Null, -32600, "Invalid Request: request too large"))
            }
            Ok(Read::Got) => match std::str::from_utf8(&buf) {
                Err(_) => Some(rpc_error(Value::Null, -32700, "Parse error: invalid UTF-8")),
                Ok(line) if line.trim().is_empty() => None,
                Ok(line) => handle_line(line),
            },
        };
        if let Some(resp) = resp {
            if !reply(&resp) {
                return 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_reader_caps_and_resyncs() {
        let data = b"ok\nXXXXXXXXXXXXXXXXXXXX\nnext\nlast";
        let mut r = std::io::BufReader::with_capacity(4, &data[..]);
        let mut buf = Vec::new();
        let mut got = Vec::new();
        loop {
            let l = read_line_bounded(&mut r, &mut buf, 8).unwrap();
            if l == Read::Eof {
                break;
            }
            got.push((l, buf.clone()));
        }
        assert_eq!(
            got,
            [
                (Read::Got, b"ok".to_vec()),
                (Read::TooLong, vec![]),
                (Read::Got, b"next".to_vec()),
                (Read::Got, b"last".to_vec()),
            ]
        );
    }

    #[test]
    fn ids_are_restricted() {
        let call = |id: &str| {
            handle_line(&format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"ping"}}"#)).unwrap()
        };
        assert!(call("1")["result"].is_object());
        assert!(call("\"a\"")["result"].is_object());
        assert!(call("null")["result"].is_object());
        for bad in ["1.5", "[1]", "{}", "true", &format!("\"{}\"", "x".repeat(257))] {
            assert_eq!(call(bad)["error"]["code"], -32600, "{bad}");
        }
    }

    #[test]
    fn jsonrpc_must_be_2_0() {
        let r = handle_line(r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#).unwrap();
        assert_eq!(r["error"]["code"], -32600);
        let r = handle_line(r#"{"id":1,"method":"ping"}"#).unwrap();
        assert_eq!(r["error"]["code"], -32600);
    }
}
