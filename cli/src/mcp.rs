//! Model Context Protocol server over stdio: JSON-RPC 2.0, one message per line.

use crate::output::CliError;
use crate::spec::{self, Args};
use serde_json::{Value, json};
use std::io::{BufRead, Write};

pub const PROTOCOL_VERSION: &str = "2025-06-18";

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Run one tool call; the structured value is the same JSON the CLI prints.
fn call_tool(name: &str, arguments: &Value) -> Value {
    let outcome = (|| {
        let cmd = spec::find(name)
            .ok_or_else(|| CliError::usage(format!("unknown tool {name:?}"), "See tools/list."))?;
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
                                format!("argument {k:?} must be a string"),
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
    })();
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

/// Handle one line; `None` means no response (a notification).
pub fn handle_line(line: &str) -> Option<Value> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return Some(rpc_error(Value::Null, -32700, "Parse error")),
    };
    let Some(o) = msg.as_object() else {
        return Some(rpc_error(Value::Null, -32600, "Invalid Request"));
    };
    let id = o.get("id").cloned();
    let Some(method) = o.get("method").and_then(Value::as_str) else {
        return Some(rpc_error(id.unwrap_or(Value::Null), -32600, "Invalid Request"));
    };
    // No id: a notification, never answered.
    let id = id?;
    let params = o.get("params").cloned().unwrap_or(Value::Null);
    Some(match method {
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
                return Some(rpc_error(id, -32602, &format!("Unknown tool: {name}")));
            }
            rpc_result(id, call_tool(name, params.get("arguments").unwrap_or(&Value::Null)))
        }
        _ => rpc_error(id, -32601, &format!("Method not found: {method}")),
    })
}

pub fn serve() -> i32 {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { return 1 };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(resp) = handle_line(&line) {
            let mut out = stdout.lock();
            if writeln!(out, "{resp}").and_then(|()| out.flush()).is_err() {
                return 0;
            }
        }
    }
    0
}
