//! Command implementations. Each returns the success envelope or a `CliError`.

use crate::duration::{self, Span};
use crate::output::{CliError, EXIT_CODES, EXIT_INTERNAL, EXIT_USAGE, MAX_OUTPUT, clip, ok};
use crate::schema;
use crate::spec::Args;
use chrono::agent::{
    Clock, Diagnose, ErrorCode, FixedClock, LocalPolicy, LocalTimeError, REFERENCE_INSTANT_RFC3339,
    ResolveLocal, SPECIFIERS, SystemClock, facts, naive_facts,
};
use chrono::format::{Item, StrftimeItems};
use chrono::{
    DateTime, DurationRound, FixedOffset, Local, Months, NaiveDate, NaiveDateTime, TimeDelta, Utc,
};
use serde_json::{Value, json};
use std::fmt::{Display, Write};

/// Run a command; a result whose JSON exceeds `MAX_OUTPUT` bytes is an error.
pub fn run(cmd: &str, a: &Args) -> Result<Value, CliError> {
    limit_output(run_inner(cmd, a)?)
}

/// Refuse a result whose JSON exceeds `MAX_OUTPUT` bytes.
fn limit_output(v: Value) -> Result<Value, CliError> {
    let size = v.to_string().len();
    if size > MAX_OUTPUT {
        return Err(CliError::new(
            ErrorCode::OutOfRange,
            format!("the output would be {size} bytes; the limit is {MAX_OUTPUT}"),
        )
        .with_hint("Use a shorter format string or fewer specifiers."));
    }
    Ok(v)
}

fn run_inner(cmd: &str, a: &Args) -> Result<Value, CliError> {
    match cmd {
        "now" => now(a),
        "parse" => parse(a),
        "format" => format(a),
        "convert" => convert(a),
        "add" => add_sub(a, false),
        "sub" => add_sub(a, true),
        "diff" => diff(a),
        "round" => round(a, true),
        "trunc" => round(a, false),
        "specifiers" => specifiers(a),
        "errors" => errors(),
        "schema" => Ok(ok("schema", json!({"schema": schema::schema()}))),
        other => Err(CliError::usage(
            format!("unknown command {:?}", clip(other)),
            format!("Commands: {}.", crate::spec::names().join(", ")),
        )),
    }
}

fn get<'a>(a: &'a Args, k: &str) -> Option<&'a str> {
    a.get(k).map(String::as_str)
}

fn req<'a>(a: &'a Args, k: &str) -> Result<&'a str, CliError> {
    get(a, k).ok_or_else(|| CliError::usage(format!("missing argument {k:?}"), "Run with --help."))
}

// ---- zones -----------------------------------------------------------------

#[derive(Clone, Copy)]
enum Zone {
    Fixed(FixedOffset),
    Local,
}

fn utc_offset() -> FixedOffset {
    FixedOffset::east_opt(0).expect("zero offset")
}

fn parse_zone(s: &str) -> Result<Zone, CliError> {
    let t = s.trim();
    let shown = clip(t);
    match t.to_ascii_lowercase().as_str() {
        "z" | "utc" => return Ok(Zone::Fixed(utc_offset())),
        "local" => return Ok(Zone::Local),
        _ => {}
    }
    let (sign, rest) = match t.as_bytes().first() {
        Some(b'+') => (1, &t[1..]),
        Some(b'-') => (-1, &t[1..]),
        _ => {
            return Err(if t.contains('/') || t.chars().any(|c| c.is_ascii_alphabetic()) {
                CliError::new(
                    ErrorCode::ParseInvalid,
                    format!(
                        "{shown:?} looks like an IANA time zone name, which is not supported yet"
                    ),
                )
                .with_hint(
                    "IANA zones are planned. Use a fixed offset (+HH:MM, -HH:MM), Z, or local.",
                )
            } else {
                CliError::new(ErrorCode::ParseInvalid, format!("{shown:?} is not a zone"))
                    .with_hint("Use +HH:MM, -HH:MM, Z or local.")
            });
        }
    };
    // Offsets are ASCII; a multi-byte character would make the byte split below unsound.
    let rest = if rest.is_ascii() { rest } else { "" };
    let (h, m) = match rest.split_once(':') {
        Some((h, m)) => (h, m),
        None if rest.len() == 4 => rest.split_at(2),
        None if rest.len() <= 2 => (rest, "0"),
        None => ("", ""),
    };
    let num = |x: &str| {
        if !x.is_empty() && x.len() <= 2 && x.bytes().all(|b| b.is_ascii_digit()) {
            x.parse::<i32>().ok()
        } else {
            None
        }
    };
    let (Some(h), Some(m)) = (num(h), num(m)) else {
        return Err(CliError::new(
            ErrorCode::ParseInvalid,
            format!("{shown:?} is not a valid offset"),
        )
        .with_hint("Use +HH:MM, -HH:MM, +HHMM, Z or local."));
    };
    if h > 23 || m > 59 {
        return Err(CliError::new(
            ErrorCode::ParseOutOfRange,
            format!("offset {shown:?} is out of range"),
        )
        .with_hint("Hours must be 0-23 and minutes 0-59."));
    }
    FixedOffset::east_opt(sign * (h * 3600 + m * 60)).map(Zone::Fixed).ok_or_else(|| {
        CliError::new(ErrorCode::ParseOutOfRange, format!("offset {shown:?} is out of range"))
    })
}

fn zone_arg(a: &Args, k: &str) -> Result<Option<Zone>, CliError> {
    get(a, k).map(|s| parse_zone(s).map_err(|e| e.with_prefix(&format!("--{k}")))).transpose()
}

fn policy(a: &Args) -> LocalPolicy {
    match get(a, "policy") {
        Some("earliest") => LocalPolicy::Earliest,
        Some("latest") => LocalPolicy::Latest,
        _ => LocalPolicy::Reject,
    }
}

fn local_err<T: Display>(e: LocalTimeError<T>) -> CliError
where
    LocalTimeError<T>: Diagnose,
{
    let mut err = CliError::from_diag(&e);
    if let LocalTimeError::Ambiguous { earliest, latest } = &e {
        err.details = Some(json!({"earliest": earliest.to_string(), "latest": latest.to_string()}));
        err.hint = "That wall-clock time occurs twice at a DST fold. Pass --policy earliest or --policy latest, or supply an explicit offset.".into();
    }
    err
}

fn to_zone(dt: DateTime<FixedOffset>, z: Zone) -> DateTime<FixedOffset> {
    match z {
        Zone::Fixed(o) => dt.with_timezone(&o),
        Zone::Local => dt.with_timezone(&Local).fixed_offset(),
    }
}

fn place(n: NaiveDateTime, z: Zone, p: LocalPolicy) -> Result<DateTime<FixedOffset>, CliError> {
    match z {
        Zone::Fixed(o) => n.and_local_timezone(o).resolve(p).map_err(local_err),
        Zone::Local => {
            n.and_local_timezone(Local).resolve(p).map(|d| d.fixed_offset()).map_err(local_err)
        }
    }
}

// ---- inputs ----------------------------------------------------------------

#[derive(Clone, Copy)]
enum Inst {
    Aware(DateTime<FixedOffset>),
    Naive(NaiveDateTime),
}

impl Inst {
    fn json(&self) -> Value {
        match self {
            Inst::Aware(d) => json!({"naive": false, "instant": facts(d)}),
            Inst::Naive(n) => json!({"naive": true, "instant": naive_facts(n)}),
        }
    }
}

struct Loaded {
    inst: Inst,
    matched: String,
    assumed: bool,
}

const AWARE_FORMATS: &[(&str, &str)] = &[
    ("iso_offset_space", "%Y-%m-%d %H:%M:%S%.f %:z"),
    ("iso_offset_minutes", "%Y-%m-%dT%H:%M%:z"),
];
const NAIVE_FORMATS: &[(&str, &str)] = &[
    ("iso_naive_t", "%Y-%m-%dT%H:%M:%S%.f"),
    ("iso_naive_space", "%Y-%m-%d %H:%M:%S%.f"),
    ("iso_naive_minutes_t", "%Y-%m-%dT%H:%M"),
    ("iso_naive_minutes_space", "%Y-%m-%d %H:%M"),
];
const DATE_FORMATS: &[(&str, &str)] = &[("date", "%Y-%m-%d"), ("date_slash", "%Y/%m/%d")];

fn midnight(d: NaiveDate) -> NaiveDateTime {
    d.and_hms_opt(0, 0, 0).expect("midnight exists")
}

fn parse_auto(s: &str) -> Result<(Inst, String), CliError> {
    let t = s.trim();
    if let Some(u) = t.strip_prefix('@') {
        return match u.parse::<i64>().ok().and_then(|n| DateTime::from_timestamp(n, 0)) {
            Some(d) => Ok((Inst::Aware(d.fixed_offset()), "unix_seconds".into())),
            None => Err(CliError::new(
                ErrorCode::ParseInvalid,
                format!("{:?} is not a unix timestamp in range", clip(t)),
            )
            .with_hint("Use @<integer seconds since 1970-01-01T00:00:00Z>.")),
        };
    }
    let first = match DateTime::parse_from_rfc3339(t) {
        Ok(d) => return Ok((Inst::Aware(d), "rfc3339".into())),
        Err(e) => e,
    };
    if let Ok(d) = DateTime::parse_from_rfc2822(t) {
        return Ok((Inst::Aware(d), "rfc2822".into()));
    }
    for (name, f) in AWARE_FORMATS {
        if let Ok(d) = DateTime::parse_from_str(t, f) {
            return Ok((Inst::Aware(d), (*name).into()));
        }
    }
    for (name, f) in NAIVE_FORMATS {
        if let Ok(n) = NaiveDateTime::parse_from_str(t, f) {
            return Ok((Inst::Naive(n), (*name).into()));
        }
    }
    for (name, f) in DATE_FORMATS {
        if let Ok(d) = NaiveDate::parse_from_str(t, f) {
            return Ok((Inst::Naive(midnight(d)), (*name).into()));
        }
    }
    let mut names = vec!["rfc3339", "rfc2822", "unix_seconds (@N)"];
    names.extend(AWARE_FORMATS.iter().chain(NAIVE_FORMATS).chain(DATE_FORMATS).map(|x| x.0));
    let mut err = CliError::from_diag(&first);
    if !matches!(err.code.as_str(), "parse_out_of_range" | "parse_impossible") {
        err.code = ErrorCode::ParseInvalid.as_str().into();
    }
    err.message =
        format!("{:?} matches none of the supported formats ({})", clip(t), names.join(", "));
    err.hint = "Use RFC 3339 (2024-03-01T12:00:00Z), or pass --format <strftime>.".into();
    Err(err)
}

fn parse_with_format(s: &str, f: &str) -> Result<(Inst, String), CliError> {
    let aware = DateTime::parse_from_str(s, f);
    if let Ok(d) = aware {
        return Ok((Inst::Aware(d), "format".into()));
    }
    let naive = NaiveDateTime::parse_from_str(s, f);
    if let Ok(n) = naive {
        return Ok((Inst::Naive(n), "format".into()));
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, f) {
        return Ok((Inst::Naive(midnight(d)), "format".into()));
    }
    let wants_offset =
        f.contains("%z") || f.contains("%:z") || f.contains("%#z") || f.contains("%Z");
    let err = if wants_offset { aware.err() } else { naive.err() };
    Err(err.map(|e| CliError::from_diag(&e)).unwrap_or_else(|| CliError::internal("parse failed")))
}

/// Parse the argument `key` as a timestamp; a naive result is placed in
/// `--offset` when that is given.
fn load(a: &Args, key: &str, fmt: Option<&str>) -> Result<Loaded, CliError> {
    let s = req(a, key)?;
    let (inst, matched) = match fmt {
        Some(f) => parse_with_format(s, f)?,
        None => parse_auto(s)?,
    };
    match (inst, zone_arg(a, "offset")?) {
        (Inst::Naive(n), Some(z)) => {
            Ok(Loaded { inst: Inst::Aware(place(n, z, policy(a))?), matched, assumed: true })
        }
        (inst, _) => Ok(Loaded { inst, matched, assumed: false }),
    }
}

fn need_aware(i: Inst, what: &str) -> Result<DateTime<FixedOffset>, CliError> {
    match i {
        Inst::Aware(d) => Ok(d),
        Inst::Naive(_) => Err(CliError::new(ErrorCode::ParseNotEnough, format!("{what} needs an offset but the input has none"))
            .with_hint("Pass --offset <+HH:MM|Z|local> to say where the wall time is, or give an RFC 3339 input with an offset.")),
    }
}

// ---- commands --------------------------------------------------------------

fn now(a: &Args) -> Result<Value, CliError> {
    let clock: Box<dyn Clock> = match get(a, "now") {
        Some(s) => Box::new(FixedClock(parse_now(s)?)),
        None => Box::new(SystemClock),
    };
    let z = zone_arg(a, "offset")?.unwrap_or(Zone::Fixed(utc_offset()));
    let dt = to_zone(clock.now().fixed_offset(), z);
    Ok(ok("now", Inst::Aware(dt).json()))
}

fn parse_now(s: &str) -> Result<DateTime<Utc>, CliError> {
    DateTime::parse_from_rfc3339(s.trim())
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| CliError::from_diag(&e).with_prefix("--now"))
}

fn parse(a: &Args) -> Result<Value, CliError> {
    let l = load(a, "input", get(a, "format"))?;
    let mut v = l.inst.json();
    v["matched"] = json!(l.matched);
    v["offset_assumed"] = json!(l.assumed);
    Ok(ok("parse", v))
}

fn format(a: &Args) -> Result<Value, CliError> {
    let l = load(a, "input", None)?;
    let f = req(a, "format")?;
    let items = StrftimeItems::new(f);
    if items.clone().any(|i| matches!(i, Item::Error)) {
        return Err(CliError::new(
            ErrorCode::ParseBadFormat,
            format!("invalid strftime format {:?}", clip(f)),
        ));
    }
    let mut out = String::new();
    let r = match &l.inst {
        Inst::Aware(d) => write!(out, "{}", d.format_with_items(items)),
        Inst::Naive(n) => write!(out, "{}", n.format_with_items(items)),
    };
    if r.is_err() {
        return Err(CliError::new(
            ErrorCode::ParseBadFormat,
            format!("format {:?} cannot be applied to this input", clip(f)),
        )
        .with_hint(
            "The format uses offset or zone fields but the input has no offset; pass --offset.",
        ));
    }
    let mut v = l.inst.json();
    v["formatted"] = json!(out);
    v["format"] = json!(f);
    Ok(ok("format", v))
}

fn convert(a: &Args) -> Result<Value, CliError> {
    let l = load(a, "input", None)?;
    let to = parse_zone(req(a, "to")?).map_err(|e| e.with_prefix("--to"))?;
    let d = need_aware(l.inst, "convert")?;
    let mut v = Inst::Aware(to_zone(d, to)).json();
    v["from"] = json!(d.to_rfc3339());
    Ok(ok("convert", v))
}

fn span_json(s: &Span) -> Value {
    json!({"months": s.months, "seconds": s.delta.num_seconds(), "nanos": s.delta.subsec_nanos()})
}

fn out_of_range(what: &str) -> CliError {
    CliError::new(
        ErrorCode::OutOfRange,
        format!("{what} is outside the representable range of dates"),
    )
}

fn apply(i: Inst, s: &Span) -> Result<Inst, CliError> {
    let m = Months::new(
        u32::try_from(s.months.unsigned_abs()).map_err(|_| out_of_range("the result"))?,
    );
    let r = match i {
        Inst::Aware(d) => {
            let d = if s.months >= 0 { d.checked_add_months(m) } else { d.checked_sub_months(m) };
            d.and_then(|d| d.checked_add_signed(s.delta)).map(Inst::Aware)
        }
        Inst::Naive(n) => {
            let n = if s.months >= 0 { n.checked_add_months(m) } else { n.checked_sub_months(m) };
            n.and_then(|n| n.checked_add_signed(s.delta)).map(Inst::Naive)
        }
    };
    r.ok_or_else(|| out_of_range("the result"))
}

fn add_sub(a: &Args, negate: bool) -> Result<Value, CliError> {
    let l = load(a, "input", None)?;
    let mut s = duration::parse(req(a, "duration")?)?;
    if negate {
        s = s.negated()?;
    }
    let r = apply(l.inst, &s)?;
    let mut v = r.json();
    v["applied"] = json!(span_json(&s));
    Ok(ok(if negate { "sub" } else { "add" }, v))
}

fn iso_duration(total_nanos: i128) -> String {
    let neg = total_nanos < 0;
    let n = total_nanos.unsigned_abs();
    let (secs, nanos) = (n / 1_000_000_000, n % 1_000_000_000);
    let (d, rem) = (secs / 86_400, secs % 86_400);
    let (h, m, s) = (rem / 3600, rem % 3600 / 60, rem % 60);
    let mut out = String::from(if neg { "-P" } else { "P" });
    if d > 0 {
        out.push_str(&format!("{d}D"));
    }
    if h + m + s + nanos > 0 || d == 0 {
        out.push('T');
        if h > 0 {
            out.push_str(&format!("{h}H"));
        }
        if m > 0 {
            out.push_str(&format!("{m}M"));
        }
        if s > 0 || nanos > 0 || (h == 0 && m == 0) {
            if nanos > 0 {
                let frac = format!("{nanos:09}");
                out.push_str(&format!("{s}.{}S", frac.trim_end_matches('0')));
            } else {
                out.push_str(&format!("{s}S"));
            }
        }
    }
    out
}

fn human_duration(total_nanos: i128) -> String {
    let neg = total_nanos < 0;
    let n = total_nanos.unsigned_abs();
    let (secs, nanos) = (n / 1_000_000_000, n % 1_000_000_000);
    let mut parts = Vec::new();
    let mut unit = |v: u128, name: &str| {
        if v > 0 {
            parts.push(format!("{v} {name}{}", if v == 1 { "" } else { "s" }));
        }
    };
    unit(secs / 86_400, "day");
    unit(secs % 86_400 / 3600, "hour");
    unit(secs % 3600 / 60, "minute");
    if nanos > 0 {
        let frac = format!("{nanos:09}");
        parts.push(format!("{}.{} seconds", secs % 60, frac.trim_end_matches('0')));
    } else {
        unit(secs % 60, "second");
    }
    if parts.is_empty() {
        return "0 seconds".into();
    }
    format!("{}{}", if neg { "-" } else { "" }, parts.join(", "))
}

fn diff(a: &Args) -> Result<Value, CliError> {
    let (x, y) = (load(a, "a", None)?, load(a, "b", None)?);
    let d: TimeDelta = match (x.inst, y.inst) {
        (Inst::Aware(p), Inst::Aware(q)) => q.signed_duration_since(p),
        (Inst::Naive(p), Inst::Naive(q)) => q.signed_duration_since(p),
        _ => {
            return Err(CliError::new(
                ErrorCode::ParseNotEnough,
                "one input has an offset and the other does not",
            )
            .with_hint("Give both with offsets, or pass --offset so naive inputs get one."));
        }
    };
    let total = i128::from(d.num_seconds()) * 1_000_000_000 + i128::from(d.subsec_nanos());
    Ok(ok(
        "diff",
        json!({
            "seconds": d.num_seconds(),
            "nanos": d.subsec_nanos(),
            "total_nanos": d.num_nanoseconds(),
            "negative": total < 0,
            "iso8601": iso_duration(total),
            "human": human_duration(total),
        }),
    ))
}

fn round(a: &Args, nearest: bool) -> Result<Value, CliError> {
    let l = load(a, "input", None)?;
    let s = duration::parse(req(a, "duration")?)?;
    if s.months != 0 {
        return Err(CliError::new(
            ErrorCode::ParseInvalid,
            "months and years cannot be a rounding step",
        )
        .with_hint("Use exact units: ns us ms s m h d w."));
    }
    if s.delta <= TimeDelta::zero() {
        return Err(CliError::new(ErrorCode::ParseInvalid, "the rounding step must be positive")
            .with_hint("Use a positive duration such as 15m."));
    }
    let r = match l.inst {
        Inst::Aware(d) => {
            if nearest { d.duration_round(s.delta) } else { d.duration_trunc(s.delta) }
                .map(Inst::Aware)
                .map_err(|e| CliError::from_diag(&e))
        }
        Inst::Naive(n) => {
            if nearest { n.duration_round(s.delta) } else { n.duration_trunc(s.delta) }
                .map(Inst::Naive)
                .map_err(|e| CliError::from_diag(&e))
        }
    }?;
    let mut v = r.json();
    v["step"] = json!(span_json(&s));
    Ok(ok(if nearest { "round" } else { "trunc" }, v))
}

fn specifiers(a: &Args) -> Result<Value, CliError> {
    let group = get(a, "group");
    let mut groups: Vec<&str> = SPECIFIERS.iter().map(|s| s.group).collect();
    groups.sort_unstable();
    groups.dedup();
    if let Some(g) = group {
        if !groups.contains(&g) {
            return Err(CliError::usage(
                format!("unknown group {:?}", clip(g)),
                format!("Groups: {}.", groups.join(", ")),
            ));
        }
    }
    let list: Vec<_> = SPECIFIERS.iter().filter(|s| group.is_none_or(|g| s.group == g)).collect();
    Ok(ok(
        "specifiers",
        json!({"reference_instant": REFERENCE_INSTANT_RFC3339, "groups": groups, "count": list.len(), "specifiers": list}),
    ))
}

fn errors() -> Result<Value, CliError> {
    let mut list: Vec<Value> = ErrorCode::ALL
        .iter()
        .map(|c| json!({"code": c.as_str(), "hint": c.hint(), "exit_code": crate::output::exit_for(*c)}))
        .collect();
    list.push(json!({"code": "usage", "hint": "Run `ironchrono <command> --help`. Each argument is at most 4096 bytes.", "exit_code": EXIT_USAGE}));
    list.push(json!({"code": "internal", "hint": "A bug in ironchrono; please report it.", "exit_code": EXIT_INTERNAL}));
    let codes: Vec<Value> =
        EXIT_CODES.iter().map(|(c, m)| json!({"exit_code": c, "meaning": m})).collect();
    Ok(ok("errors", json!({"errors": list, "exit_codes": codes})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_over_the_limit_is_out_of_range() {
        let big = json!({"formatted": "x".repeat(MAX_OUTPUT)});
        let e = limit_output(big).unwrap_err();
        assert_eq!((e.code.as_str(), e.exit), ("out_of_range", 4));
        assert!(e.hint.contains("format"));
        assert!(limit_output(json!({"formatted": "x".repeat(1000)})).is_ok());
    }

    #[test]
    fn the_largest_format_stays_under_the_limit() {
        // 4096 bytes of `%+` is the most one argument can expand to.
        let mut a = Args::new();
        a.insert("input".into(), "2024-03-01T12:00:00Z".into());
        a.insert("format".into(), "%+".repeat(2048));
        assert!(run("format", &a).is_ok());
    }
}
