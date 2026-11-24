//! Hand-written JSON Schema (draft 2020-12) for every envelope and output.

use serde_json::{Map, Value, json};

fn obj(required: &[&str], props: Value) -> Value {
    json!({"type": "object", "required": required, "properties": props})
}

fn string() -> Value {
    json!({"type": "string"})
}
fn integer() -> Value {
    json!({"type": "integer"})
}
fn boolean() -> Value {
    json!({"type": "boolean"})
}
fn facts_ref() -> Value {
    json!({"$ref": "#/$defs/facts"})
}

/// Success fields shared by every instant-returning command.
fn instant_props(extra: Value) -> (Vec<&'static str>, Value) {
    let mut p = Map::new();
    p.insert("naive".into(), boolean());
    p.insert("instant".into(), facts_ref());
    if let Value::Object(e) = extra {
        p.extend(e);
    }
    (vec!["ok", "command", "naive", "instant"], Value::Object(p))
}

fn instant_output(command: &str, extra_required: &[&'static str], extra: Value) -> Value {
    let (mut req, props) = instant_props(extra);
    req.extend_from_slice(extra_required);
    with_envelope(command, &req, props)
}

fn with_envelope(command: &str, required: &[&str], props: Value) -> Value {
    let mut p = Map::new();
    p.insert("ok".into(), json!({"const": true}));
    p.insert("command".into(), json!({"const": command}));
    if let Value::Object(e) = props {
        p.extend(e);
    }
    obj(required, Value::Object(p))
}

fn span() -> Value {
    obj(
        &["months", "seconds", "nanos"],
        json!({"months": integer(), "seconds": integer(), "nanos": integer()}),
    )
}

/// The output schema of one command (success envelope).
pub fn output_schema(command: &str) -> Option<Value> {
    Some(match command {
        "now" => instant_output("now", &[], json!({})),
        "parse" => instant_output(
            "parse",
            &["matched", "offset_assumed"],
            json!({"matched": string(), "offset_assumed": boolean()}),
        ),
        "format" => instant_output(
            "format",
            &["formatted", "format"],
            json!({"formatted": string(), "format": string()}),
        ),
        "convert" => instant_output("convert", &["from"], json!({"from": string()})),
        "add" | "sub" => instant_output(command, &["applied"], json!({"applied": span()})),
        "round" | "trunc" => instant_output(command, &["step"], json!({"step": span()})),
        "diff" => with_envelope(
            "diff",
            &["ok", "command", "seconds", "nanos", "total_nanos", "negative", "iso8601", "human"],
            json!({
                "seconds": integer(),
                "nanos": integer(),
                "total_nanos": {"type": ["integer", "null"]},
                "negative": boolean(),
                "iso8601": string(),
                "human": string()
            }),
        ),
        "specifiers" => with_envelope(
            "specifiers",
            &["ok", "command", "reference_instant", "groups", "count", "specifiers"],
            json!({
                "reference_instant": string(),
                "groups": {"type": "array", "items": string()},
                "count": integer(),
                "specifiers": {"type": "array", "items": obj(
                    &["spec", "meaning", "example", "parses", "group"],
                    json!({"spec": string(), "meaning": string(), "example": string(), "parses": boolean(), "group": string()}))}
            }),
        ),
        "errors" => with_envelope(
            "errors",
            &["ok", "command", "errors", "exit_codes"],
            json!({
                "errors": {"type": "array", "items": obj(
                    &["code", "hint", "exit_code"],
                    json!({"code": string(), "hint": string(), "exit_code": integer()}))},
                "exit_codes": {"type": "array", "items": obj(
                    &["exit_code", "meaning"], json!({"exit_code": integer(), "meaning": string()}))}
            }),
        ),
        "schema" => with_envelope(
            "schema",
            &["ok", "command", "schema"],
            json!({"schema": {"type": "object"}}),
        ),
        _ => return None,
    })
}

pub const OUTPUT_COMMANDS: &[&str] = &[
    "now",
    "parse",
    "format",
    "convert",
    "add",
    "sub",
    "diff",
    "round",
    "trunc",
    "specifiers",
    "errors",
    "schema",
];

pub fn schema() -> Value {
    let mut outputs = Map::new();
    for c in OUTPUT_COMMANDS {
        outputs.insert((*c).into(), output_schema(c).expect("known command"));
    }
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://github.com/nervosys/IronChrono/ironchrono.schema.json",
        "title": "ironchrono output",
        "description": "Every invocation prints exactly one JSON object: a success envelope or an error envelope. Exit code 0 iff ok is true.",
        "oneOf": [{"$ref": "#/$defs/success"}, {"$ref": "#/$defs/error"}],
        "$defs": {
            "success": {
                "type": "object",
                "required": ["ok", "command"],
                "properties": {"ok": {"const": true}, "command": string()}
            },
            "error": {
                "type": "object",
                "required": ["ok", "error"],
                "properties": {
                    "ok": {"const": false},
                    "error": obj(&["code", "message", "hint"], json!({
                        "code": {"type": "string", "description": "Stable snake_case code; see `errors`"},
                        "message": string(),
                        "hint": string(),
                        "details": {"type": "object"}
                    }))
                }
            },
            "facts": facts_schema(),
            "outputs": Value::Object(outputs)
        }
    })
}

pub const FACT_KEYS: &[&str] = &[
    "rfc3339",
    "unix_seconds",
    "unix_nanos",
    "year",
    "month",
    "month_name",
    "day",
    "ordinal",
    "iso_year",
    "iso_week",
    "weekday",
    "weekday_from_monday",
    "hour",
    "minute",
    "second",
    "nanosecond",
    "offset_seconds",
    "offset",
    "is_leap_year",
    "days_in_month",
    "leap_second",
];

fn facts_schema() -> Value {
    let int_or_null = json!({"type": ["integer", "null"]});
    json!({
        "type": "object",
        "required": FACT_KEYS,
        "properties": {
            "rfc3339": string(),
            "unix_seconds": int_or_null,
            "unix_nanos": int_or_null,
            "year": integer(), "month": integer(), "month_name": string(), "day": integer(), "ordinal": integer(),
            "iso_year": integer(), "iso_week": integer(),
            "weekday": string(), "weekday_from_monday": integer(),
            "hour": integer(), "minute": integer(), "second": integer(), "nanosecond": integer(),
            "offset_seconds": int_or_null, "offset": {"type": ["string", "null"]},
            "is_leap_year": boolean(), "days_in_month": integer(), "leap_second": boolean()
        }
    })
}
