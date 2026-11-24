//! Black-box tests: run the built binary, check JSON, exit codes and schemas.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

const NOW: &str = "2024-03-01T12:00:00Z";

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ironchrono"))
}

/// Run (with `--now` fixed for the `now` command, the only one that reads the clock);
/// returns (exit code, parsed stdout).
fn run(args: &[&str]) -> (i32, Value) {
    let mut c = bin();
    c.args(args);
    if args.first() == Some(&"now") {
        c.args(["--now", NOW]);
    }
    let out = c.output().expect("spawn");
    let text = String::from_utf8(out.stdout).expect("utf8");
    let v: Value =
        serde_json::from_str(text.trim()).unwrap_or_else(|e| panic!("not JSON ({e}): {text:?}"));
    assert_eq!(text.trim().lines().count(), 1, "one object per invocation: {text:?}");
    (out.status.code().expect("exit code"), v)
}

fn ok(args: &[&str]) -> Value {
    let (code, v) = run(args);
    assert_eq!(code, 0, "{args:?} -> {v}");
    assert_eq!(v["ok"], json!(true), "{v}");
    check_output(&v);
    v
}

fn err(args: &[&str], exit: i32, code: &str) -> Value {
    let (c, v) = run(args);
    assert_eq!((c, v["error"]["code"].as_str()), (exit, Some(code)), "{args:?} -> {v}");
    assert_eq!(v["ok"], json!(false));
    assert!(v["error"]["message"].is_string() && v["error"]["hint"].is_string(), "{v}");
    check_output(&v);
    v
}

// ---- a tiny JSON Schema checker: type, required, properties, items, const, enum, $ref ----

fn schema_doc() -> Value {
    let out = bin().arg("schema").output().expect("spawn");
    let v: Value = serde_json::from_slice(&out.stdout).expect("schema JSON");
    v["schema"].clone()
}

fn check(root: &Value, schema: &Value, v: &Value, path: &str) {
    if let Some(r) = schema.get("$ref").and_then(Value::as_str) {
        let target =
            root.pointer(r.trim_start_matches('#')).unwrap_or_else(|| panic!("bad $ref {r}"));
        return check(root, target, v, path);
    }
    if let Some(c) = schema.get("const") {
        assert_eq!(c, v, "{path}: const");
    }
    if let Some(e) = schema.get("enum").and_then(Value::as_array) {
        assert!(e.contains(v), "{path}: {v} not in enum");
    }
    if let Some(t) = schema.get("type") {
        let types: Vec<&str> = match t {
            Value::String(s) => vec![s.as_str()],
            Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
            _ => vec![],
        };
        let actual_ok = types.iter().any(|t| match *t {
            "object" => v.is_object(),
            "array" => v.is_array(),
            "string" => v.is_string(),
            "integer" => v.is_i64() || v.is_u64(),
            "number" => v.is_number(),
            "boolean" => v.is_boolean(),
            "null" => v.is_null(),
            _ => false,
        });
        assert!(actual_ok, "{path}: {v} is not of type {types:?}");
    }
    if let (Some(req), Some(o)) = (schema.get("required").and_then(Value::as_array), v.as_object())
    {
        for k in req.iter().filter_map(Value::as_str) {
            assert!(o.contains_key(k), "{path}: missing required key {k:?} in {v}");
        }
    }
    if let (Some(props), Some(o)) =
        (schema.get("properties").and_then(Value::as_object), v.as_object())
    {
        for (k, sub) in props {
            if let Some(child) = o.get(k) {
                check(root, sub, child, &format!("{path}.{k}"));
            }
        }
    }
    if let (Some(items), Some(a)) = (schema.get("items"), v.as_array()) {
        for (i, x) in a.iter().enumerate() {
            check(root, items, x, &format!("{path}[{i}]"));
        }
    }
}

/// Validate a real output against the envelope and the per-command schema.
fn check_output(v: &Value) {
    let root = schema_doc();
    if v["ok"] == json!(true) {
        check(&root, &root["$defs"]["success"], v, "$");
        let cmd = v["command"].as_str().unwrap();
        if let Some(s) = root["$defs"]["outputs"].get(cmd) {
            check(&root, s, v, "$");
        }
    } else {
        check(&root, &root["$defs"]["error"], v, "$");
    }
}

// ---- commands ----------------------------------------------------------------

#[test]
fn now_is_deterministic() {
    let v = ok(&["now"]);
    assert_eq!(v["command"], "now");
    assert_eq!(v["instant"]["year"], 2024);
    assert_eq!(v["instant"]["hour"], 12);
    assert_eq!(v["naive"], false);
    let v = ok(&["now", "--offset", "+05:30"]);
    assert_eq!(v["instant"]["hour"], 17);
    assert_eq!(v["instant"]["minute"], 30);
    assert_eq!(v["instant"]["offset_seconds"], 19800);
    ok(&["now", "--offset", "local"]);
    let out = bin().args(["now", "--now", "garbage"]).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["error"]["message"].as_str().unwrap().contains("--now"));
}

#[test]
fn parse_reports_the_matching_format() {
    let cases = [
        ("2024-03-01T12:00:00Z", "rfc3339", false),
        ("2024-03-01T12:00:00.5+02:00", "rfc3339", false),
        ("Fri, 01 Mar 2024 12:00:00 +0000", "rfc2822", false),
        ("2024-03-01 12:00:00+02:00", "rfc3339", false),
        ("2024-03-01 12:00:00 +02:00", "iso_offset_space", false),
        ("2024-03-01T12:00+02:00", "iso_offset_minutes", false),
        ("2024-03-01T12:00:00", "iso_naive_t", true),
        ("2024-03-01 12:00:00.25", "iso_naive_space", true),
        ("2024-03-01 12:00", "iso_naive_minutes_space", true),
        ("2024-03-01", "date", true),
        ("2024/03/01", "date_slash", true),
        ("@1709294400", "unix_seconds", false),
    ];
    for (input, matched, naive) in cases {
        let v = ok(&["parse", input]);
        assert_eq!(v["matched"], matched, "{input}");
        assert_eq!(v["naive"], naive, "{input}");
        assert_eq!(v["offset_assumed"], false);
    }
    assert_eq!(ok(&["parse", "@1709294400"])["instant"]["rfc3339"], "2024-03-01T12:00:00+00:00");
}

#[test]
fn parse_with_format_and_offset() {
    let v = ok(&["parse", "01/03/2024 08:15", "--format", "%d/%m/%Y %H:%M"]);
    assert_eq!((v["matched"].as_str(), v["naive"].as_bool()), (Some("format"), Some(true)));
    let v = ok(&["parse", "2024-03-01T12:00:00", "--offset", "-08:00"]);
    assert_eq!(v["naive"], false);
    assert_eq!(v["offset_assumed"], true);
    assert_eq!(v["instant"]["offset_seconds"], -28800);
    let v = ok(&["parse", "2024-03-01", "--offset", "Z"]);
    assert_eq!(v["instant"]["hour"], 0);
    // An aware input keeps its own offset.
    let v = ok(&["parse", "2024-03-01T12:00:00+02:00", "--offset", "-08:00"]);
    assert_eq!(v["instant"]["offset_seconds"], 7200);
    ok(&["parse", "2024-03-01T12:00:00", "--offset", "local", "--policy", "earliest"]);
}

#[test]
fn parse_errors() {
    let v = err(&["parse", "tomorrow"], 3, "parse_invalid");
    assert!(v["error"]["message"].as_str().unwrap().contains("rfc3339"));
    let (c, v) = run(&["parse", "2024-13-45T00:00:00Z"]);
    assert_eq!(c, 3, "{v}");
    let (c, v) = run(&["parse", "2024-03-01", "--format", "%Q"]);
    assert_eq!(c, 3, "{v}");
    err(&["parse", "x", "--offset", "Europe/Paris"], 3, "parse_invalid");
    err(&["parse", "2024-03-01", "--offset", "+25:00"], 3, "parse_out_of_range");
    err(&["parse", "2024-03-01", "--offset", "5"], 3, "parse_invalid");
    err(&["parse", "@99999999999999999999"], 3, "parse_invalid");
}

#[test]
fn iana_names_are_rejected_clearly() {
    let v = err(&["convert", NOW, "--to", "America/New_York"], 3, "parse_invalid");
    assert!(v["error"]["hint"].as_str().unwrap().contains("IANA"));
    assert!(v["error"]["message"].as_str().unwrap().contains("--to"));
}

#[test]
fn format_command() {
    let v = ok(&["format", NOW, "%A, %d %B %Y %H:%M"]);
    assert_eq!(v["formatted"], "Friday, 01 March 2024 12:00");
    let v = ok(&["format", "2024-03-01T12:00:00", "%Y-%m-%d"]);
    assert_eq!(v["formatted"], "2024-03-01");
    assert_eq!(v["naive"], true);
    let v = ok(&["format", "2024-03-01T12:00:00", "%z", "--offset", "+02:00"]);
    assert_eq!(v["formatted"], "+0200");
    err(&["format", NOW, "%Q"], 3, "parse_bad_format");
    err(&["format", NOW, "%"], 3, "parse_bad_format");
    // %z on a naive input cannot be rendered.
    err(&["format", "2024-03-01T12:00:00", "%z"], 3, "parse_bad_format");
}

#[test]
fn convert_command() {
    let v = ok(&["convert", NOW, "--to", "+05:30"]);
    assert_eq!(v["instant"]["rfc3339"], "2024-03-01T17:30:00+05:30");
    assert_eq!(v["from"], "2024-03-01T12:00:00+00:00");
    let v = ok(&["convert", "2024-03-01T12:00:00+05:30", "--to=Z"]);
    assert_eq!(v["instant"]["hour"], 6);
    ok(&["convert", NOW, "--to", "local"]);
    ok(&["convert", "2024-03-01T12:00:00", "--offset", "+01:00", "--to", "Z"]);
    err(&["convert", "2024-03-01T12:00:00", "--to", "Z"], 3, "parse_not_enough");
    err(&["convert", NOW], 2, "usage");
}

#[test]
fn add_and_sub() {
    let v = ok(&["add", NOW, "P1DT2H30M"]);
    assert_eq!(v["instant"]["rfc3339"], "2024-03-02T14:30:00+00:00");
    assert_eq!(v["applied"]["seconds"], 95400);
    let v = ok(&["sub", NOW, "90s"]);
    assert_eq!(v["instant"]["rfc3339"], "2024-03-01T11:58:30+00:00");
    assert_eq!(v["command"], "sub");
    // Negative durations flip direction.
    let v = ok(&["add", NOW, "-1h"]);
    assert_eq!(v["instant"]["hour"], 11);
    let v = ok(&["sub", NOW, "-1h"]);
    assert_eq!(v["instant"]["hour"], 13);
    // Month-end clamping.
    let v = ok(&["add", "2024-01-31T00:00:00Z", "P1M"]);
    assert_eq!(v["instant"]["rfc3339"], "2024-02-29T00:00:00+00:00");
    let v = ok(&["add", "2023-01-31T00:00:00Z", "1mo"]);
    assert_eq!(v["instant"]["rfc3339"], "2023-02-28T00:00:00+00:00");
    let v = ok(&["sub", "2024-03-31T00:00:00Z", "1mo"]);
    assert_eq!(v["instant"]["rfc3339"], "2024-02-29T00:00:00+00:00");
    // Months first, then the exact part.
    let v = ok(&["add", "2024-01-31T00:00:00Z", "P1MT24H"]);
    assert_eq!(v["instant"]["rfc3339"], "2024-03-01T00:00:00+00:00");
    // Naive stays naive.
    let v = ok(&["add", "2024-03-01T12:00:00", "1d"]);
    assert_eq!(v["naive"], true);
    assert_eq!(v["instant"]["day"], 2);
}

#[test]
fn add_errors() {
    err(&["add", NOW, "1x"], 3, "parse_invalid");
    err(&["add", NOW, "P"], 3, "parse_invalid");
    err(&["add", NOW, "9223372036854775807d"], 4, "out_of_range");
    err(&["add", NOW, "P4000000000Y"], 4, "out_of_range");
    err(&["add", "@8210266876799", "1d"], 4, "out_of_range");
    err(&["add", "2024-03-01T00:00:00Z", "2000000y"], 4, "out_of_range");
    err(&["sub", "2024-03-01T00:00:00Z", "300000y"], 4, "out_of_range");
}

#[test]
fn diff_command() {
    let v = ok(&["diff", "2024-03-01T00:00:00Z", "2024-03-02T01:30:00Z"]);
    assert_eq!(v["seconds"], 91800);
    assert_eq!(v["nanos"], 0);
    assert_eq!(v["iso8601"], "P1DT1H30M");
    assert_eq!(v["human"], "1 day, 1 hour, 30 minutes");
    assert_eq!(v["negative"], false);
    let v = ok(&["diff", "2024-03-02T00:00:00.5Z", "2024-03-01T00:00:00Z"]);
    assert_eq!(v["seconds"], -86400);
    assert_eq!(v["nanos"], -500_000_000);
    assert_eq!(v["negative"], true);
    assert_eq!(v["iso8601"], "-P1DT0.5S");
    let v = ok(&["diff", NOW, NOW]);
    assert_eq!((v["iso8601"].as_str(), v["human"].as_str()), (Some("PT0S"), Some("0 seconds")));
    // Across offsets the instants, not the wall clocks, are compared.
    let v = ok(&["diff", "2024-03-01T12:00:00+02:00", "2024-03-01T12:00:00Z"]);
    assert_eq!(v["seconds"], 7200);
    err(&["diff", NOW, "2024-03-01T12:00:00"], 3, "parse_not_enough");
    ok(&["diff", NOW, "2024-03-01T12:00:00", "--offset", "Z"]);
    err(&["diff", NOW], 2, "usage");
}

#[test]
fn round_and_trunc() {
    let v = ok(&["round", "2024-03-01T12:34:56Z", "15m"]);
    assert_eq!(v["instant"]["rfc3339"], "2024-03-01T12:30:00+00:00");
    let v = ok(&["trunc", "2024-03-01T12:34:56Z", "1h"]);
    assert_eq!(v["instant"]["rfc3339"], "2024-03-01T12:00:00+00:00");
    assert_eq!(v["command"], "trunc");
    let v = ok(&["round", "2024-03-01T12:34:56", "PT1M"]);
    assert_eq!(v["naive"], true);
    assert_eq!(v["instant"]["minute"], 35);
    err(&["round", NOW, "1mo"], 3, "parse_invalid");
    err(&["round", NOW, "0s"], 3, "parse_invalid");
    err(&["round", NOW, "-5m"], 3, "parse_invalid");
    let (c, v) = run(&["round", NOW, "100000000d"]);
    assert_eq!(c, 4, "{v}");
}

#[test]
fn specifiers_command() {
    let v = ok(&["specifiers"]);
    let all = v["count"].as_u64().unwrap();
    assert!(all > 20);
    assert_eq!(v["specifiers"].as_array().unwrap().len() as u64, all);
    let g = v["groups"][0].as_str().unwrap().to_string();
    let v = ok(&["specifiers", "--group", &g]);
    assert!(v["count"].as_u64().unwrap() <= all);
    assert!(v["specifiers"].as_array().unwrap().iter().all(|s| s["group"] == g.as_str()));
    err(&["specifiers", "--group", "nope"], 2, "usage");
}

#[test]
fn errors_command_lists_the_vocabulary() {
    let v = ok(&["errors"]);
    let codes: Vec<&str> =
        v["errors"].as_array().unwrap().iter().map(|e| e["code"].as_str().unwrap()).collect();
    for c in
        ["parse_invalid", "out_of_range", "local_time_ambiguous", "local_time_nonexistent", "usage"]
    {
        assert!(codes.contains(&c), "{c} missing from {codes:?}");
    }
    let exit_of = |c: &str| {
        v["errors"].as_array().unwrap().iter().find(|e| e["code"] == c).unwrap()["exit_code"]
            .clone()
    };
    assert_eq!(exit_of("parse_invalid"), 3);
    assert_eq!(exit_of("out_of_range"), 4);
    assert_eq!(exit_of("local_time_ambiguous"), 5);
    assert_eq!(exit_of("usage"), 2);
}

#[test]
fn usage_errors_and_exit_code_two() {
    err(&["frobnicate"], 2, "usage");
    err(&["parse"], 2, "usage");
    err(&["parse", "a", "b"], 2, "usage");
    err(&["parse", "x", "--bogus", "1"], 2, "usage");
    err(&["parse", "x", "--policy", "sometimes"], 2, "usage");
    err(&["parse", "x", "--offset"], 2, "usage");
    err(&["parse", "x", "--offset", "Z", "--offset", "Z"], 2, "usage");
    let out = bin().output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"]["code"], "usage");
}

#[test]
fn negative_numbers_are_positional() {
    // `-1h` must not be mistaken for a flag; `--` also works.
    ok(&["add", NOW, "-1h"]);
    let out = bin().args(["add", "--", NOW, "-1h"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn text_mode() {
    let out = bin().args(["parse", NOW, "--text"]).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "2024-03-01T12:00:00+00:00");
    let out = bin().args(["diff", NOW, "2024-03-01T13:00:00Z", "--text"]).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "1 hour");
    let out = bin().args(["parse", "nope", "--text"]).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("error[parse_invalid]"));
}

#[test]
fn help_and_version() {
    let out = bin().arg("--help").output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let h = String::from_utf8_lossy(&out.stdout);
    for needle in [
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
        "mcp",
        "Exit codes",
        "  3  ",
        "  5  ",
    ] {
        assert!(h.contains(needle), "help lacks {needle:?}");
    }
    let out = bin().args(["add", "--help"]).output().unwrap();
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("Usage: ironchrono add <input> <duration>")
    );
    let out = bin().arg("--version").output().unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
    let out = bin().args(["--version", "--text"]).output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        format!("ironchrono {}", env!("CARGO_PKG_VERSION"))
    );
}

// ---- schema --------------------------------------------------------------------

#[test]
fn schema_describes_every_command() {
    let s = schema_doc();
    assert_eq!(s["$schema"], "https://json-schema.org/draft/2020-12/schema");
    for c in [
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
    ] {
        assert!(s["$defs"]["outputs"][c].is_object(), "no output schema for {c}");
    }
    // Every command above is exercised against its schema through `ok()`.
    for args in [
        vec!["now"],
        vec!["parse", NOW],
        vec!["format", NOW, "%Y"],
        vec!["convert", NOW, "--to", "Z"],
        vec!["add", NOW, "1h"],
        vec!["sub", NOW, "1h"],
        vec!["diff", NOW, NOW],
        vec!["round", NOW, "1h"],
        vec!["trunc", NOW, "1h"],
        vec!["specifiers"],
        vec!["errors"],
        vec!["schema"],
    ] {
        ok(&args);
    }
    // The checker itself must be able to fail.
    let v = ok(&["now"]);
    let mut broken = v.clone();
    broken["instant"].as_object_mut().unwrap().remove("year");
    let root = schema_doc();
    let r =
        std::panic::catch_unwind(|| check(&root, &root["$defs"]["outputs"]["now"], &broken, "$"));
    assert!(r.is_err(), "validator accepted an output missing a required key");
}

// ---- MCP -----------------------------------------------------------------------

struct Mcp {
    child: std::process::Child,
    out: BufReader<std::process::ChildStdout>,
}

impl Mcp {
    fn start() -> Self {
        let mut child =
            bin().arg("mcp").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        Mcp { child, out }
    }
    fn send_raw(&mut self, line: &str) {
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{line}").unwrap();
        stdin.flush().unwrap();
    }
    fn recv(&mut self) -> Value {
        let mut line = String::new();
        self.out.read_line(&mut line).unwrap();
        serde_json::from_str(line.trim()).unwrap_or_else(|e| panic!("bad line {line:?}: {e}"))
    }
    fn request(&mut self, v: Value) -> Value {
        self.send_raw(&v.to_string());
        self.recv()
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        let _ = self.child.wait();
    }
}

#[test]
fn mcp_session() {
    let mut m = Mcp::start();
    let r = m.request(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}));
    assert_eq!(r["id"], 1);
    assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(r["result"]["serverInfo"]["name"], "ironchrono");
    assert_eq!(r["result"]["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(r["result"]["capabilities"]["tools"].is_object());

    // A notification gets no response: the next line read must answer the ping.
    m.send_raw(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    let r = m.request(json!({"jsonrpc":"2.0","id":"p","method":"ping"}));
    assert_eq!((r["id"].as_str(), r["result"].clone()), (Some("p"), json!({})));

    let r = m.request(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}));
    let tools = r["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
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
            "errors"
        ]
    );
    for t in tools {
        assert_eq!(t["inputSchema"]["type"], "object");
        assert!(t["inputSchema"]["properties"].is_object());
        assert!(t["description"].is_string());
    }
    let parse = tools.iter().find(|t| t["name"] == "parse").unwrap();
    assert_eq!(parse["inputSchema"]["required"], json!(["input"]));

    let r = m.request(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"parse","arguments":{"input":"2024-03-01T12:00:00Z"}}}));
    let res = &r["result"];
    assert_eq!(res["isError"], false);
    assert_eq!(res["structuredContent"]["matched"], "rfc3339");
    assert_eq!(res["content"][0]["type"], "text");
    let echoed: Value = serde_json::from_str(res["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(&echoed, &res["structuredContent"]);
    // Same JSON the CLI prints.
    assert_eq!(echoed, ok(&["parse", "2024-03-01T12:00:00Z"]));

    // A failing tool is a result with isError, not a JSON-RPC error.
    let r = m.request(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"add","arguments":{"input":NOW,"duration":"1x"}}}));
    assert_eq!(r["result"]["isError"], true);
    assert_eq!(r["result"]["structuredContent"]["error"]["code"], "parse_invalid");
    let r = m.request(json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"now","arguments":{"now":NOW,"offset":"+01:00"}}}));
    assert_eq!(r["result"]["structuredContent"]["instant"]["hour"], 13);
    let r = m.request(json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"parse","arguments":{}}}));
    assert_eq!(r["result"]["isError"], true);
    assert_eq!(r["result"]["structuredContent"]["error"]["code"], "usage");
    let r =
        m.request(json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nope"}}));
    assert_eq!(r["error"]["code"], -32602);

    let r = m.request(json!({"jsonrpc":"2.0","id":8,"method":"resources/list"}));
    assert_eq!(r["error"]["code"], -32601);
    assert_eq!(r["id"], 8);

    m.send_raw("{not json");
    let r = m.recv();
    assert_eq!(r["error"]["code"], -32700);
    assert!(r["id"].is_null());

    m.send_raw("[1,2]");
    assert_eq!(m.recv()["error"]["code"], -32600);

    // Still alive after the bad input.
    let r = m.request(json!({"jsonrpc":"2.0","id":9,"method":"ping"}));
    assert_eq!(r["id"], 9);
}

// ---- security hardening ----------------------------------------------------------

fn tool_call(id: u64, name: &str, args: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":args}})
}

fn stdout_json(out: &std::process::Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"))
}

#[test]
fn non_ascii_offset_is_a_parse_error_not_a_panic() {
    for bad in ["+a\u{20ac}", "+\u{20ac}a", "-\u{20ac}\u{20ac}\u{20ac}"] {
        let out = bin().args(["convert", NOW, "--to", bad]).output().unwrap();
        assert_eq!(out.status.code(), Some(3), "{bad:?}");
        let v = stdout_json(&out);
        assert_eq!(v["error"]["code"], "parse_invalid", "{v}");
        assert!(String::from_utf8_lossy(&out.stderr).is_empty(), "{:?}", out.stderr);
        let out = bin().args(["parse", "2024-03-01T12:00:00", "--offset", bad]).output().unwrap();
        assert_eq!(out.status.code(), Some(3), "{bad:?}");
    }
}

#[test]
fn mcp_survives_a_non_ascii_offset() {
    let mut m = Mcp::start();
    let r = m.request(tool_call(1, "convert", json!({"input": NOW, "to": "+a\u{20ac}"})));
    assert_eq!(r["result"]["isError"], true, "{r}");
    assert_eq!(r["result"]["structuredContent"]["error"]["code"], "parse_invalid");
    let r = m.request(json!({"jsonrpc":"2.0","id":2,"method":"ping"}));
    assert_eq!(r["id"], 2);
}

#[test]
fn non_utf8_argument_is_a_usage_error() {
    let bad = {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            std::ffi::OsString::from_wide(&[0x70, 0xD800, 0x71])
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            std::ffi::OsString::from_vec(vec![0x70, 0xFF, 0x71])
        }
    };
    let out = bin().arg("parse").arg(bad).output().unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert_eq!(stdout_json(&out)["error"]["code"], "usage");
    assert!(String::from_utf8_lossy(&out.stderr).is_empty());
}

#[test]
fn mcp_invalid_utf8_line_is_answered_and_the_server_lives() {
    let mut m = Mcp::start();
    {
        let stdin = m.child.stdin.as_mut().unwrap();
        stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"\xff\xfe\"}\n").unwrap();
        stdin.flush().unwrap();
    }
    let r = m.recv();
    assert_eq!(r["error"]["code"], -32700, "{r}");
    assert!(r["id"].is_null());
    let r = m.request(json!({"jsonrpc":"2.0","id":2,"method":"ping"}));
    assert_eq!(r["id"], 2);
}

#[test]
fn mcp_oversized_line_is_refused_and_the_stream_resyncs() {
    let mut m = Mcp::start();
    let big =
        format!(r#"{{"jsonrpc":"2.0","id":1,"method":"ping","pad":"{}"}}"#, "A".repeat(2 << 20));
    // Write from a thread: the server must answer while we are still sending.
    let mut stdin = m.child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        let _ = writeln!(stdin, "{big}");
        let _ = writeln!(stdin, r#"{{"jsonrpc":"2.0","id":2,"method":"ping"}}"#);
        let _ = stdin.flush();
        stdin
    });
    let r = m.recv();
    assert_eq!(r["error"]["code"], -32600, "{r}");
    assert!(r["error"]["message"].as_str().unwrap().contains("too large"));
    assert!(r["id"].is_null());
    let r = m.recv();
    assert_eq!(r["id"], 2);
    m.child.stdin = Some(writer.join().unwrap());
}

#[test]
fn mcp_restricts_ids_and_jsonrpc_version() {
    let mut m = Mcp::start();
    for id in [json!(1.5), json!([1]), json!({"a":1}), json!(true), json!("x".repeat(257))] {
        let r = m.request(json!({"jsonrpc":"2.0","id":id,"method":"ping"}));
        assert_eq!(r["error"]["code"], -32600, "{id}");
        assert!(r["id"].is_null());
    }
    let r = m.request(json!({"jsonrpc":"2.0","id":"x".repeat(256),"method":"ping"}));
    assert!(r["result"].is_object());
    let r = m.request(json!({"jsonrpc":"1.0","id":3,"method":"ping"}));
    assert_eq!(r["error"]["code"], -32600);
    let r = m.request(json!({"id":4,"method":"ping"}));
    assert_eq!(r["error"]["code"], -32600);
    // tools/call works before initialize (documented leniency).
    let r = m.request(tool_call(5, "parse", json!({"input": NOW})));
    assert_eq!(r["result"]["isError"], false);
}

#[test]
fn arguments_over_4096_bytes_are_a_usage_error() {
    let long = "1".repeat(4097);
    let out = bin().args(["parse", &long]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v = stdout_json(&out);
    assert_eq!(v["error"]["code"], "usage");
    let msg = v["error"]["message"].as_str().unwrap();
    assert!(msg.contains("input") && msg.contains("4096"), "{msg}");
    assert!(msg.len() < 400, "message must not echo the argument");
    // Exactly at the limit is accepted (it fails to parse, but not as a usage error).
    let at = "1".repeat(4096);
    let out = bin().args(["parse", &at]).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    let mut m = Mcp::start();
    let r = m.request(tool_call(1, "parse", json!({"input": long})));
    assert_eq!(r["result"]["isError"], true);
    assert_eq!(r["result"]["structuredContent"]["error"]["code"], "usage");
}

#[test]
fn echoed_input_in_errors_is_cut_to_200_chars() {
    let long = "x".repeat(3000);
    let cases: Vec<Vec<&str>> = vec![
        vec!["parse", &long],
        vec!["add", NOW, &long],
        vec!["convert", NOW, "--to", &long],
        vec!["frobnicate", &long],
        vec!["parse", NOW, "--policy", &long],
    ];
    for args in cases {
        let out = bin().args(&args).output().unwrap();
        let v = stdout_json(&out);
        let msg = v["error"]["message"].as_str().unwrap();
        assert!(!msg.contains(&"x".repeat(201)), "{args:?}: {} bytes", msg.len());
    }
    let out = bin().args(["parse", NOW, &format!("--{}", "y".repeat(3000))]).output().unwrap();
    let msg = stdout_json(&out)["error"]["message"].as_str().unwrap().to_string();
    assert!(!msg.contains(&"y".repeat(201)), "{msg}");
    assert!(msg.contains('\u{2026}'));
}

#[test]
fn text_mode_escapes_control_and_bidi_characters() {
    let out =
        bin().args(["format", NOW, "\x1b[2J\u{202e}x\u{85}%Y\u{2066}", "--text"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(!s.contains('\x1b') && !s.contains('\u{202e}') && !s.contains('\u{85}'), "{s:?}");
    assert!(!s.contains('\u{2066}'), "{s:?}");
    assert!(s.contains("\\u{1b}") && s.contains("\\u{202e}"), "{s:?}");
    // Errors are escaped too, including the unknown-flag message (JSON or text).
    let out = bin().args(["parse", NOW, "--\x1b[31m", "--text"]).output().unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(!s.contains('\x1b'), "{s:?}");
    let out = bin().args(["parse", NOW, "--\x1b[31m"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(!String::from_utf8_lossy(&out.stdout).contains('\x1b'));
}

#[test]
fn global_flags_count_only_before_double_dash_and_not_as_values() {
    // After `--`, --help is the input.
    let out = bin().args(["parse", "--", "--help"]).output().unwrap();
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    assert_eq!(stdout_json(&out)["ok"], false);
    // As a flag's value, --text is the value (and the output stays JSON).
    let out = bin().args(["parse", "2024-03-01", "--format", "--text"]).output().unwrap();
    assert!(out.stdout.starts_with(b"{"), "{}", String::from_utf8_lossy(&out.stdout));
    assert_eq!(stdout_json(&out)["ok"], false);
    // --help before `--` still works.
    let out = bin().args(["parse", "-h"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains("Usage: ironchrono parse"));
    // --text still works in front of the command.
    let out = bin().args(["--text", "parse", NOW]).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "2024-03-01T12:00:00+00:00");
}

#[test]
fn help_on_a_closed_pipe_does_not_panic() {
    let mut child =
        bin().arg("--help").stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"), "{:?}", out.stderr);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn now_flag_is_only_accepted_by_now() {
    err(&["parse", NOW, "--now", "garbage"], 2, "usage");
    err(&["errors", "--now", NOW], 2, "usage");
    let out = bin().args(["now", "--now", NOW]).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
}
