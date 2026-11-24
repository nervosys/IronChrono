//! Envelopes, error classes and exit codes.

use chrono::agent::{Diagnose, ErrorCode};
use serde_json::{Map, Value, json};

pub const EXIT_OK: i32 = 0;
pub const EXIT_INTERNAL: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_PARSE: i32 = 3;
pub const EXIT_OUT_OF_RANGE: i32 = 4;
pub const EXIT_LOCAL_TIME: i32 = 5;

/// Longest accepted string argument, in bytes (CLI and MCP alike).
pub const MAX_ARG: usize = 4096;
/// Longest formatted output, in bytes.
pub const MAX_OUTPUT: usize = 65536;
/// Longest echo of caller input inside an error message, in characters.
pub const MAX_ECHO: usize = 200;

/// Caller input as it may appear in an error message: at most `MAX_ECHO`
/// characters, with `…` when cut. Use as `{:?}` of `clip(s)`.
pub fn clip(s: &str) -> String {
    match s.char_indices().nth(MAX_ECHO) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.to_string(),
    }
}

/// Run `f`, mapping a panic to the documented internal error.
pub fn guard<T>(f: impl FnOnce() -> Result<T, CliError>) -> Result<T, CliError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .unwrap_or_else(|_| Err(CliError::internal("an internal error occurred")))
}

/// Escape control, C1 and bidi characters so output text cannot steer a terminal.
fn esc_with(s: &str, keep_newline: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let bad = (c.is_control() && !(keep_newline && c == '\n'))
            || matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}' | '\u{061C}');
        if bad {
            out.push_str(&format!("\\u{{{:x}}}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

pub fn esc(s: &str) -> String {
    esc_with(s, false)
}

/// Exit-code table, as `(code, meaning)`; shown in `--help` and `errors`.
pub const EXIT_CODES: &[(i32, &str)] = &[
    (EXIT_OK, "ok"),
    (EXIT_INTERNAL, "internal error"),
    (
        EXIT_USAGE,
        "usage error: unknown command or flag, missing, invalid or over-long (4096 bytes) argument",
    ),
    (EXIT_PARSE, "parse error: input, format, offset or duration could not be understood"),
    (
        EXIT_OUT_OF_RANGE,
        "out of range: the result is not representable, rounding failed, or the output exceeds 65536 bytes",
    ),
    (EXIT_LOCAL_TIME, "local time is ambiguous or nonexistent (see --policy)"),
];

/// The exit code of a library error code.
pub fn exit_for(code: ErrorCode) -> i32 {
    match code {
        ErrorCode::OutOfRange
        | ErrorCode::RoundingDurationExceedsTimestamp
        | ErrorCode::RoundingDurationExceedsLimit
        | ErrorCode::RoundingTimestampExceedsLimit => EXIT_OUT_OF_RANGE,
        ErrorCode::LocalTimeNonexistent | ErrorCode::LocalTimeAmbiguous => EXIT_LOCAL_TIME,
        _ => EXIT_PARSE,
    }
}

#[derive(Debug, Clone)]
pub struct CliError {
    pub code: String,
    pub message: String,
    pub hint: String,
    pub exit: i32,
    pub details: Option<Value>,
}

impl CliError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        CliError {
            code: code.as_str().to_string(),
            message: message.into(),
            hint: code.hint().to_string(),
            exit: exit_for(code),
            details: None,
        }
    }

    pub fn usage(message: impl Into<String>, hint: impl Into<String>) -> Self {
        CliError {
            code: "usage".into(),
            message: message.into(),
            hint: hint.into(),
            exit: EXIT_USAGE,
            details: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        CliError {
            code: "internal".into(),
            message: message.into(),
            hint: "This is a bug in ironchrono; please report it.".into(),
            exit: EXIT_INTERNAL,
            details: None,
        }
    }

    pub fn from_diag<D: Diagnose>(e: &D) -> Self {
        let d = e.diagnosis();
        CliError {
            code: d.code.as_str().to_string(),
            message: d.message,
            hint: d.hint.to_string(),
            exit: exit_for(d.code),
            details: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }

    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.message = format!("{prefix}: {}", self.message);
        self
    }

    pub fn to_json(&self) -> Value {
        let mut err = Map::new();
        err.insert("code".into(), json!(self.code));
        err.insert("message".into(), json!(self.message));
        err.insert("hint".into(), json!(self.hint));
        if let Some(d) = &self.details {
            err.insert("details".into(), d.clone());
        }
        json!({"ok": false, "error": Value::Object(err)})
    }
}

/// Wrap a command's fields in the success envelope.
pub fn ok(command: &str, fields: Value) -> Value {
    let mut m = Map::new();
    m.insert("ok".into(), json!(true));
    m.insert("command".into(), json!(command));
    if let Value::Object(f) = fields {
        for (k, v) in f {
            m.insert(k, v);
        }
    }
    Value::Object(m)
}

/// Human rendering for `--text`.
pub fn render_text(v: &Value) -> String {
    if v["ok"] == json!(false) {
        let e = &v["error"];
        return format!(
            "error[{}]: {}\nhint: {}",
            esc(e["code"].as_str().unwrap_or("?")),
            esc(e["message"].as_str().unwrap_or("")),
            esc(e["hint"].as_str().unwrap_or(""))
        );
    }
    let s = |x: &Value| esc(x.as_str().unwrap_or(""));
    match v["command"].as_str().unwrap_or("") {
        "format" => s(&v["formatted"]),
        "diff" => s(&v["human"]),
        "version" => format!("ironchrono {}", s(&v["version"])),
        "specifiers" => v["specifiers"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|x| {
                        format!(
                            "{:<6} {} (e.g. {}){}",
                            s(&x["spec"]),
                            s(&x["meaning"]),
                            s(&x["example"]),
                            if x["parses"] == json!(false) { " [format only]" } else { "" }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default(),
        "errors" => v["errors"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|x| {
                        format!("{:<36} exit {}  {}", s(&x["code"]), x["exit_code"], s(&x["hint"]))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default(),
        _ => {
            if v["instant"].is_object() {
                let naive = if v["naive"] == json!(true) { " (naive, no offset)" } else { "" };
                format!("{}{}", s(&v["instant"]["rfc3339"]), naive)
            } else {
                esc_with(&serde_json::to_string_pretty(v).unwrap_or_default(), true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_cuts_long_input() {
        assert_eq!(clip("abc"), "abc");
        let c = clip(&"é".repeat(500));
        assert_eq!(c.chars().count(), MAX_ECHO + 1);
        assert!(c.ends_with('…'));
    }

    #[test]
    fn guard_maps_a_panic_to_internal() {
        let r: Result<(), CliError> = guard(|| panic!("boom"));
        let e = r.unwrap_err();
        assert_eq!((e.code.as_str(), e.exit), ("internal", EXIT_INTERNAL));
        assert!(!e.message.contains("boom"));
    }

    #[test]
    fn esc_escapes_controls_and_bidi() {
        assert_eq!(esc("a\x1b[2J\u{202e}b\u{85}\x7f\n"), r"a\u{1b}[2J\u{202e}b\u{85}\u{7f}\u{a}");
    }
}
