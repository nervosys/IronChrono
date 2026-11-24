//! Envelopes, error classes and exit codes.

use chrono::agent::{Diagnose, ErrorCode};
use serde_json::{Map, Value, json};

pub const EXIT_OK: i32 = 0;
pub const EXIT_INTERNAL: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_PARSE: i32 = 3;
pub const EXIT_OUT_OF_RANGE: i32 = 4;
pub const EXIT_LOCAL_TIME: i32 = 5;

/// Exit-code table, as `(code, meaning)`; shown in `--help` and `errors`.
pub const EXIT_CODES: &[(i32, &str)] = &[
    (EXIT_OK, "ok"),
    (EXIT_INTERNAL, "internal error"),
    (EXIT_USAGE, "usage error: unknown command or flag, missing or invalid argument"),
    (EXIT_PARSE, "parse error: input, format, offset or duration could not be understood"),
    (EXIT_OUT_OF_RANGE, "out of range: the result is not representable, or rounding failed"),
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
            e["code"].as_str().unwrap_or("?"),
            e["message"].as_str().unwrap_or(""),
            e["hint"].as_str().unwrap_or("")
        );
    }
    let s = |x: &Value| x.as_str().unwrap_or("").to_string();
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
                serde_json::to_string_pretty(v).unwrap_or_default()
            }
        }
    }
}
