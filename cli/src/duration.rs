//! Duration syntax: ISO 8601 (`P1DT2H30M`) and compact (`90s`, `1h30m`, `1mo`).
//!
//! A [`Span`] is a calendar part (months) plus an exact part (a `TimeDelta`).
//! Years are 12 months. Calendar months are applied first, with month-end
//! clamping (Jan 31 + 1 month = Feb 28/29), then the exact part.

use crate::output::{CliError, clip};
use chrono::TimeDelta;
use chrono::agent::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub months: i64,
    pub delta: TimeDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanError {
    Syntax(String),
    Overflow(String),
}

impl From<SpanError> for CliError {
    fn from(e: SpanError) -> Self {
        match e {
            SpanError::Syntax(m) => CliError::new(ErrorCode::ParseInvalid, m).with_hint(
                "Use ISO 8601 (P1DT2H30M, PT90S, P1M) or compact units: ns us ms s m h d w mo y (e.g. 90s, 1h30m, 2w, 1mo).",
            ),
            SpanError::Overflow(m) => CliError::new(ErrorCode::OutOfRange, m),
        }
    }
}

impl Span {
    pub fn negated(self) -> Result<Span, SpanError> {
        Ok(Span {
            months: self.months.checked_neg().ok_or_else(overflow)?,
            delta: TimeDelta::zero().checked_sub(&self.delta).ok_or_else(overflow)?,
        })
    }

    fn add_months(&mut self, n: i64, per: i64) -> Result<(), SpanError> {
        let m = n.checked_mul(per).and_then(|m| self.months.checked_add(m)).ok_or_else(overflow)?;
        // `Months` is a u32, so the calendar part has to fit in one.
        if m.unsigned_abs() > u64::from(u32::MAX) {
            return Err(overflow());
        }
        self.months = m;
        Ok(())
    }

    fn add_delta(&mut self, d: Option<TimeDelta>) -> Result<(), SpanError> {
        let d = d.ok_or_else(overflow)?;
        self.delta = self.delta.checked_add(&d).ok_or_else(overflow)?;
        Ok(())
    }
}

fn overflow() -> SpanError {
    SpanError::Overflow("duration is too large to represent".into())
}

fn syntax(msg: impl Into<String>) -> SpanError {
    SpanError::Syntax(msg.into())
}

pub fn parse(input: &str) -> Result<Span, SpanError> {
    let s = input.trim();
    let (neg, body) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    if body.is_empty() {
        return Err(syntax(format!("empty duration {:?}", clip(input))));
    }
    let span =
        if body.starts_with(['P', 'p']) { iso(&body[1..], input)? } else { compact(body, input)? };
    if neg { span.negated() } else { Ok(span) }
}

fn digits(s: &str) -> (&str, &str) {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    s.split_at(end)
}

fn int(d: &str) -> Result<i64, SpanError> {
    d.parse::<i64>().map_err(|_| overflow())
}

fn iso(mut rest: &str, input: &str) -> Result<Span, SpanError> {
    let mut span = Span { months: 0, delta: TimeDelta::zero() };
    let mut in_time = false;
    let mut rank = 0u8;
    let mut count = 0usize;
    let mut after_t = false;
    while !rest.is_empty() {
        if rest.starts_with(['T', 't']) {
            if in_time {
                return Err(syntax(format!("second 'T' in {:?}", clip(input))));
            }
            in_time = true;
            after_t = true;
            rest = &rest[1..];
            continue;
        }
        let (whole, r) = digits(rest);
        if whole.is_empty() {
            return Err(syntax(format!("expected a number in {:?}", clip(input))));
        }
        let (frac, r) = match r.strip_prefix(['.', ',']) {
            Some(r2) => {
                let (f, r3) = digits(r2);
                if f.is_empty() || f.len() > 9 {
                    return Err(syntax(format!(
                        "fraction needs 1 to 9 digits in {:?}",
                        clip(input)
                    )));
                }
                (Some(f), r3)
            }
            None => (None, r),
        };
        let unit = r
            .chars()
            .next()
            .ok_or_else(|| syntax(format!("number without a unit in {:?}", clip(input))))?;
        rest = &r[unit.len_utf8()..];
        let n = int(whole)?;
        let (this_rank, ok_frac) = match (in_time, unit.to_ascii_uppercase()) {
            (false, 'Y') => (1, false),
            (false, 'M') => (2, false),
            (false, 'W') => (3, false),
            (false, 'D') => (4, false),
            (true, 'H') => (5, false),
            (true, 'M') => (6, false),
            (true, 'S') => (7, true),
            _ => return Err(syntax(format!("unexpected unit {:?} in {:?}", unit, clip(input)))),
        };
        if frac.is_some() && !ok_frac {
            return Err(syntax(format!("only seconds may have a fraction in {:?}", clip(input))));
        }
        if this_rank <= rank {
            return Err(syntax(format!("units out of order or repeated in {:?}", clip(input))));
        }
        rank = this_rank;
        count += 1;
        after_t = false;
        match this_rank {
            1 => span.add_months(n, 12)?,
            2 => span.add_months(n, 1)?,
            3 => span.add_delta(TimeDelta::try_weeks(n))?,
            4 => span.add_delta(TimeDelta::try_days(n))?,
            5 => span.add_delta(TimeDelta::try_hours(n))?,
            6 => span.add_delta(TimeDelta::try_minutes(n))?,
            _ => {
                span.add_delta(TimeDelta::try_seconds(n))?;
                if let Some(f) = frac {
                    let nanos: i64 = format!("{f:0<9}").parse().map_err(|_| overflow())?;
                    span.add_delta(Some(TimeDelta::nanoseconds(nanos)))?;
                }
            }
        }
    }
    if count == 0 || after_t {
        return Err(syntax(format!("{:?} has no components", clip(input))));
    }
    Ok(span)
}

fn compact(mut rest: &str, input: &str) -> Result<Span, SpanError> {
    let mut span = Span { months: 0, delta: TimeDelta::zero() };
    while !rest.is_empty() {
        let (num, r) = digits(rest);
        if num.is_empty() {
            return Err(syntax(format!("expected a number in {:?}", clip(input))));
        }
        let uend = r.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(r.len());
        let (unit, r) = r.split_at(uend);
        let n = int(num)?;
        match unit {
            "ns" => span.add_delta(Some(TimeDelta::nanoseconds(n)))?,
            "us" => span.add_delta(Some(TimeDelta::microseconds(n)))?,
            "ms" => span.add_delta(TimeDelta::try_milliseconds(n))?,
            "s" => span.add_delta(TimeDelta::try_seconds(n))?,
            "m" | "min" => span.add_delta(TimeDelta::try_minutes(n))?,
            "h" => span.add_delta(TimeDelta::try_hours(n))?,
            "d" => span.add_delta(TimeDelta::try_days(n))?,
            "w" => span.add_delta(TimeDelta::try_weeks(n))?,
            "mo" => span.add_months(n, 1)?,
            "y" => span.add_months(n, 12)?,
            "" => return Err(syntax(format!("number {num} has no unit in {:?}", clip(input)))),
            u => return Err(syntax(format!("unknown unit {:?} in {:?}", clip(u), clip(input)))),
        }
        rest = r;
    }
    Ok(span)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: &str) -> i64 {
        parse(s).unwrap().delta.num_seconds()
    }

    #[test]
    fn compact_units() {
        assert_eq!(secs("90s"), 90);
        assert_eq!(secs("15m"), 900);
        assert_eq!(secs("2h"), 7200);
        assert_eq!(secs("3d"), 259_200);
        assert_eq!(secs("1w"), 604_800);
        assert_eq!(secs("1h30m"), 5400);
        assert_eq!(parse("1mo").unwrap().months, 1);
        assert_eq!(parse("2y").unwrap().months, 24);
        assert_eq!(parse("1500ms").unwrap().delta, TimeDelta::milliseconds(1500));
    }

    #[test]
    fn iso_forms() {
        let s = parse("P1DT2H30M").unwrap();
        assert_eq!(s.delta.num_seconds(), 86_400 + 9000);
        assert_eq!(parse("P1M").unwrap().months, 1);
        assert_eq!(parse("P1Y2M").unwrap().months, 14);
        assert_eq!(parse("PT0.5S").unwrap().delta, TimeDelta::milliseconds(500));
        assert_eq!(parse("PT1,25S").unwrap().delta, TimeDelta::milliseconds(1250));
        assert_eq!(secs("P2W"), 1_209_600);
        assert_eq!(secs("p1dt1h"), 90_000);
        assert_eq!(parse("PT0S").unwrap().delta, TimeDelta::zero());
        // "M" is months before the T and minutes after it.
        let s = parse("P1MT1M").unwrap();
        assert_eq!((s.months, s.delta.num_seconds()), (1, 60));
    }

    #[test]
    fn negatives() {
        assert_eq!(secs("-90s"), -90);
        let s = parse("-P1Y1DT1H").unwrap();
        assert_eq!(s.months, -12);
        assert_eq!(s.delta.num_seconds(), -(86_400 + 3600));
        assert_eq!(parse("+5m").unwrap().delta.num_seconds(), 300);
        assert_eq!(parse("1h").unwrap().negated().unwrap().delta.num_seconds(), -3600);
    }

    #[test]
    fn syntax_errors() {
        for bad in [
            "", "-", "P", "PT", "P1DT", "1x", "5", "h", "P1M1Y", "P1D1D", "PT1.5M", "P1.5D",
            "PT1.S", "P1", "PTT1S", "1.5h", "1 h", "T1H",
        ] {
            assert!(matches!(parse(bad), Err(SpanError::Syntax(_))), "{bad:?} -> {:?}", parse(bad));
        }
    }

    #[test]
    fn overflow_is_out_of_range() {
        for big in [
            "9223372036854775807d",
            "99999999999999999999s",
            "P99999999999999999999D",
            "5000000000mo",
            "P400000000Y",
            "1000000000000w",
        ] {
            assert!(
                matches!(parse(big), Err(SpanError::Overflow(_))),
                "{big:?} -> {:?}",
                parse(big)
            );
        }
        let e: CliError = parse("9223372036854775807d").unwrap_err().into();
        assert_eq!(e.code, "out_of_range");
        assert_eq!(e.exit, crate::output::EXIT_OUT_OF_RANGE);
    }
}
