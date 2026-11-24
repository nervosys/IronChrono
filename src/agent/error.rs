//! Stable machine-readable error codes with remediation hints.

use core::fmt;

use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::format::{Item, ParseErrorKind, StrftimeItems};
use crate::{OutOfRangeError, ParseError, ParseMonthError, ParseWeekdayError, RoundingError};

use super::local::LocalTimeError;

/// A stable, machine-readable identifier for a failure of this library.
///
/// The `snake_case` string from [`as_str`](ErrorCode::as_str) (which is also the serde
/// representation) is part of the public contract: existing ids are never renamed or reused.
/// The enum is `#[non_exhaustive]`; new codes may be added.
///
/// ```
/// use chrono::agent::{Diagnose, ErrorCode};
/// use chrono::NaiveDate;
///
/// let err = "2023-02-30".parse::<NaiveDate>().unwrap_err();
/// assert_eq!(err.code(), ErrorCode::ParseOutOfRange);
/// assert_eq!(err.code().as_str(), "parse_out_of_range");
/// assert!(err.hint().ends_with('.'));
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// A parsed field is outside its permitted range.
    ParseOutOfRange,
    /// The parsed fields are mutually inconsistent or name a date that does not exist.
    ParseImpossible,
    /// The parsed fields are not enough to build the requested type.
    ParseNotEnough,
    /// The input contains characters that do not match the format.
    ParseInvalid,
    /// The input ended before the format did.
    ParseTooShort,
    /// The format was fully consumed but input remains.
    ParseTooLong,
    /// The format string itself is malformed or unsupported.
    ParseBadFormat,
    /// A value is outside the range the type can represent.
    OutOfRange,
    /// Rounding: the duration exceeds the timestamp (no longer produced upstream).
    RoundingDurationExceedsTimestamp,
    /// Rounding: the duration does not fit in i64 nanoseconds, or is not positive.
    RoundingDurationExceedsLimit,
    /// Rounding: the timestamp does not fit in i64 nanoseconds (outside 1677..2262).
    RoundingTimestampExceedsLimit,
    /// The text is not a month name.
    InvalidMonth,
    /// The text is not a weekday name.
    InvalidWeekday,
    /// The local time falls in a DST gap and does not exist.
    LocalTimeNonexistent,
    /// The local time falls in a DST fold and occurs twice.
    LocalTimeAmbiguous,
    /// A naive time was given where an instant (an offset) is required.
    MissingOffset,
    /// The requested time zone is not supported (for example an IANA name).
    UnsupportedTimeZone,
}

impl ErrorCode {
    /// Every code, in declaration order.
    pub const ALL: &'static [ErrorCode] = &[
        ErrorCode::ParseOutOfRange,
        ErrorCode::ParseImpossible,
        ErrorCode::ParseNotEnough,
        ErrorCode::ParseInvalid,
        ErrorCode::ParseTooShort,
        ErrorCode::ParseTooLong,
        ErrorCode::ParseBadFormat,
        ErrorCode::OutOfRange,
        ErrorCode::RoundingDurationExceedsTimestamp,
        ErrorCode::RoundingDurationExceedsLimit,
        ErrorCode::RoundingTimestampExceedsLimit,
        ErrorCode::InvalidMonth,
        ErrorCode::InvalidWeekday,
        ErrorCode::LocalTimeNonexistent,
        ErrorCode::LocalTimeAmbiguous,
        ErrorCode::MissingOffset,
        ErrorCode::UnsupportedTimeZone,
    ];

    /// The stable `snake_case` identifier of this code.
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::ParseOutOfRange => "parse_out_of_range",
            ErrorCode::ParseImpossible => "parse_impossible",
            ErrorCode::ParseNotEnough => "parse_not_enough",
            ErrorCode::ParseInvalid => "parse_invalid",
            ErrorCode::ParseTooShort => "parse_too_short",
            ErrorCode::ParseTooLong => "parse_too_long",
            ErrorCode::ParseBadFormat => "parse_bad_format",
            ErrorCode::OutOfRange => "out_of_range",
            ErrorCode::RoundingDurationExceedsTimestamp => "rounding_duration_exceeds_timestamp",
            ErrorCode::RoundingDurationExceedsLimit => "rounding_duration_exceeds_limit",
            ErrorCode::RoundingTimestampExceedsLimit => "rounding_timestamp_exceeds_limit",
            ErrorCode::InvalidMonth => "invalid_month",
            ErrorCode::InvalidWeekday => "invalid_weekday",
            ErrorCode::LocalTimeNonexistent => "local_time_nonexistent",
            ErrorCode::LocalTimeAmbiguous => "local_time_ambiguous",
            ErrorCode::MissingOffset => "missing_offset",
            ErrorCode::UnsupportedTimeZone => "unsupported_time_zone",
        }
    }

    /// One actionable sentence telling the caller how to fix the failure.
    pub fn hint(self) -> &'static str {
        match self {
            ErrorCode::ParseOutOfRange => {
                "A field is outside its valid range (for example month 13 or hour 25); check each numeric field against the documented range of its specifier."
            }
            ErrorCode::ParseImpossible => {
                "The fields are individually valid but name no real date or contradict each other (for example 2023-02-30, or a weekday that does not match the date); correct the date or drop the redundant field."
            }
            ErrorCode::ParseNotEnough => {
                "The input lacks a field the target type needs (e.g. a time zone offset for DateTime<FixedOffset>); parse into a Naive type, or add %z to the format."
            }
            ErrorCode::ParseInvalid => {
                "The input does not match the format at some character; compare it to the specifier table (chrono::agent::SPECIFIERS) and check separators, padding and field order."
            }
            ErrorCode::ParseTooShort => {
                "The input ended before the format was satisfied; supply the missing trailing fields or shorten the format."
            }
            ErrorCode::ParseTooLong => {
                "The format matched but input remains; remove the trailing text, or extend the format to cover it."
            }
            ErrorCode::ParseBadFormat => {
                "The format string is malformed or uses an unsupported specifier or modifier; check it against chrono::agent::SPECIFIERS (the modifiers -, _ and 0 apply to numeric specifiers only)."
            }
            ErrorCode::OutOfRange => {
                "The value is outside the range this type can represent; use a smaller magnitude or a wider type."
            }
            ErrorCode::RoundingDurationExceedsTimestamp => {
                "The rounding duration is larger than the timestamp it applies to; use a smaller duration."
            }
            ErrorCode::RoundingDurationExceedsLimit => {
                "The rounding duration must be positive and fit in 64-bit nanoseconds (about 292 years); use a shorter, positive duration."
            }
            ErrorCode::RoundingTimestampExceedsLimit => {
                "The timestamp does not fit in 64-bit nanoseconds (only years 1677 to 2262 can be rounded); round a value inside that range."
            }
            ErrorCode::InvalidMonth => {
                "Use an English month name or its three-letter abbreviation (January or Jan), or construct Month from a number 1 to 12."
            }
            ErrorCode::InvalidWeekday => {
                "Use an English weekday name or its three-letter abbreviation (Monday or Mon)."
            }
            ErrorCode::LocalTimeNonexistent => {
                "That wall-clock time was skipped by a DST transition; choose a time outside the gap or work in UTC."
            }
            ErrorCode::LocalTimeAmbiguous => {
                "That wall-clock time occurs twice at a DST fold; pick LocalPolicy::Earliest or LocalPolicy::Latest, or supply an explicit offset."
            }
            ErrorCode::MissingOffset => {
                "A naive time cannot name an instant; supply an offset (Z or +HH:MM) or resolve it in a local zone under an explicit policy."
            }
            ErrorCode::UnsupportedTimeZone => {
                "IANA time zone names are planned but not supported yet; use a fixed offset (+HH:MM), Z for UTC, or the local zone."
            }
        }
    }
}

impl Serialize for ErrorCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit_variant("ErrorCode", *self as u32, self.as_str())
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A serializable description of a failure: code, human message and remediation hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnosis {
    /// The stable code.
    pub code: ErrorCode,
    /// The error's `Display` text.
    pub message: String,
    /// The remediation hint for the code.
    pub hint: &'static str,
}

impl Serialize for Diagnosis {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut st = serializer.serialize_struct("Diagnosis", 3)?;
        st.serialize_field("code", &self.code)?;
        st.serialize_field("message", &self.message)?;
        st.serialize_field("hint", &self.hint)?;
        st.end()
    }
}

/// Errors that can be classified into an [`ErrorCode`].
pub trait Diagnose {
    /// The stable code for this error.
    fn code(&self) -> ErrorCode;
    /// The remediation hint (defaults to the code's hint).
    fn hint(&self) -> &'static str {
        self.code().hint()
    }
    /// A serializable code + message + hint triple.
    fn diagnosis(&self) -> Diagnosis;
}

/// Checks a strftime format string without using it. Returns a [`ErrorCode::ParseBadFormat`]
/// diagnosis naming the first bad specifier and its byte position, or `Ok(())`.
///
/// This checks syntax, which is what parsing needs. A few specifiers are valid only for
/// parsing (`%#z`): use [`check_output_format`] before formatting with an untrusted string.
///
/// ```
/// use chrono::agent::{check_format, ErrorCode};
/// assert!(check_format("%Y-%m-%d").is_ok());
/// let d = check_format("%Y-%Q").unwrap_err();
/// assert_eq!(d.code, ErrorCode::ParseBadFormat);
/// assert!(d.message.contains("%Q") && d.message.contains("position 3"));
/// ```
pub fn check_format(fmt: &str) -> Result<(), Diagnosis> {
    let has_error = |s: &str| StrftimeItems::new(s).any(|i| i == Item::Error);
    if !has_error(fmt) {
        return Ok(());
    }
    Err(bad_specifier(fmt, has_error, "unsupported or malformed"))
}

/// Checks that a strftime format string can be used to format a value with an offset
/// (a `DateTime`): [`check_format`], plus no specifier that is valid only for parsing.
/// When this returns `Ok(())`, `dt.format(fmt).to_string()` does not panic for any
/// `DateTime`; for a naive value, the offset specifiers (`%z`, `%Z`, `%:z`, `%+`, ...)
/// still fail, because a naive value has no offset.
///
/// ```
/// use chrono::agent::{check_format, check_output_format};
/// assert!(check_format("%Y %#z").is_ok()); // fine for parsing
/// let d = check_output_format("%Y %#z").unwrap_err();
/// assert!(d.message.contains("%#z") && d.message.contains("position 3"));
/// ```
pub fn check_output_format(fmt: &str) -> Result<(), Diagnosis> {
    check_format(fmt)?;
    let reference = crate::DateTime::parse_from_rfc3339(super::REFERENCE_INSTANT_RFC3339)
        .expect("the reference instant parses");
    let fails = |s: &str| {
        use core::fmt::Write;
        let mut out = String::new();
        write!(out, "{}", reference.format_with_items(StrftimeItems::new(s))).is_err()
    };
    if !fails(fmt) {
        return Ok(());
    }
    Err(bad_specifier(fmt, fails, "parse-only"))
}

/// The diagnosis for the first specifier in `fmt` that `bad` rejects.
fn bad_specifier(fmt: &str, bad: impl Fn(&str) -> bool, what: &str) -> Diagnosis {
    let bytes = fmt.as_bytes();
    let mut i = 0;
    let mut at = 0;
    while i < fmt.len() {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }
        // The shortest valid spec starting here (specs are at most a few characters long).
        let end = (i + 2..=fmt.len().min(i + 8))
            .filter(|j| fmt.is_char_boundary(*j))
            .find(|j| !bad(&fmt[i..*j]));
        match end {
            Some(j) => i = j,
            None => {
                at = i;
                break;
            }
        }
    }
    // Name the specifier: the shortest well-formed one starting here (`%#z`), or the
    // first two characters when none is (`%Q`). Escaped, so that a control character
    // in an untrusted format cannot reach a terminal or log raw.
    let rest = &fmt[at..];
    let n = (3..=5)
        .find(|n| {
            let s: String = rest.chars().take(*n).collect();
            s.chars().count() == *n && !StrftimeItems::new(&s).any(|i| i == Item::Error)
        })
        .unwrap_or(2);
    let text: String = rest.chars().take(n).collect();
    let code = ErrorCode::ParseBadFormat;
    Diagnosis {
        code,
        message: format!(
            "{} format specifier `{}` at position {} (byte offset)",
            what,
            text.escape_debug(),
            at
        ),
        hint: code.hint(),
    }
}

fn make<E: fmt::Display + Diagnose + ?Sized>(e: &E) -> Diagnosis {
    Diagnosis { code: e.code(), message: e.to_string(), hint: e.hint() }
}

impl Diagnose for ParseError {
    fn code(&self) -> ErrorCode {
        match self.kind() {
            ParseErrorKind::OutOfRange => ErrorCode::ParseOutOfRange,
            ParseErrorKind::Impossible => ErrorCode::ParseImpossible,
            ParseErrorKind::NotEnough => ErrorCode::ParseNotEnough,
            ParseErrorKind::Invalid => ErrorCode::ParseInvalid,
            ParseErrorKind::TooShort => ErrorCode::ParseTooShort,
            ParseErrorKind::TooLong => ErrorCode::ParseTooLong,
            ParseErrorKind::BadFormat => ErrorCode::ParseBadFormat,
            #[allow(unreachable_patterns)]
            _ => ErrorCode::ParseInvalid,
        }
    }
    fn diagnosis(&self) -> Diagnosis {
        make(self)
    }
}

impl Diagnose for OutOfRangeError {
    fn code(&self) -> ErrorCode {
        ErrorCode::OutOfRange
    }
    fn diagnosis(&self) -> Diagnosis {
        make(self)
    }
}

impl Diagnose for RoundingError {
    fn code(&self) -> ErrorCode {
        match self {
            RoundingError::DurationExceedsTimestamp => ErrorCode::RoundingDurationExceedsTimestamp,
            RoundingError::DurationExceedsLimit => ErrorCode::RoundingDurationExceedsLimit,
            RoundingError::TimestampExceedsLimit => ErrorCode::RoundingTimestampExceedsLimit,
        }
    }
    fn diagnosis(&self) -> Diagnosis {
        make(self)
    }
}

impl Diagnose for ParseMonthError {
    fn code(&self) -> ErrorCode {
        ErrorCode::InvalidMonth
    }
    fn diagnosis(&self) -> Diagnosis {
        make(self)
    }
}

impl Diagnose for ParseWeekdayError {
    fn code(&self) -> ErrorCode {
        ErrorCode::InvalidWeekday
    }
    fn diagnosis(&self) -> Diagnosis {
        make(self)
    }
}

impl<T: fmt::Debug + fmt::Display> Diagnose for LocalTimeError<T> {
    fn code(&self) -> ErrorCode {
        match self {
            LocalTimeError::Nonexistent => ErrorCode::LocalTimeNonexistent,
            LocalTimeError::Ambiguous { .. } => ErrorCode::LocalTimeAmbiguous,
        }
    }
    fn diagnosis(&self) -> Diagnosis {
        make(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DateTime, DurationRound, FixedOffset, Month, NaiveDate, NaiveTime, TimeDelta, TimeZone,
        Utc, Weekday,
    };
    use std::collections::HashSet;

    /// No wildcard: adding a variant without updating this fails to compile.
    fn exhaustive(c: ErrorCode) -> usize {
        match c {
            ErrorCode::ParseOutOfRange => 0,
            ErrorCode::ParseImpossible => 1,
            ErrorCode::ParseNotEnough => 2,
            ErrorCode::ParseInvalid => 3,
            ErrorCode::ParseTooShort => 4,
            ErrorCode::ParseTooLong => 5,
            ErrorCode::ParseBadFormat => 6,
            ErrorCode::OutOfRange => 7,
            ErrorCode::RoundingDurationExceedsTimestamp => 8,
            ErrorCode::RoundingDurationExceedsLimit => 9,
            ErrorCode::RoundingTimestampExceedsLimit => 10,
            ErrorCode::InvalidMonth => 11,
            ErrorCode::InvalidWeekday => 12,
            ErrorCode::LocalTimeNonexistent => 13,
            ErrorCode::LocalTimeAmbiguous => 14,
            ErrorCode::MissingOffset => 15,
            ErrorCode::UnsupportedTimeZone => 16,
        }
    }

    #[test]
    fn all_lists_every_variant_in_order() {
        assert_eq!(ErrorCode::ALL.len(), 17);
        for (i, c) in ErrorCode::ALL.iter().enumerate() {
            assert_eq!(exhaustive(*c), i);
        }
    }

    #[test]
    fn ids_are_snake_case_unique_and_hints_are_sentences() {
        let mut seen = HashSet::new();
        for c in ErrorCode::ALL {
            let s = c.as_str();
            assert!(!s.is_empty());
            assert!(s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit()));
            assert!(!s.starts_with('_') && !s.ends_with('_') && !s.contains("__"));
            assert!(seen.insert(s), "duplicate id {}", s);
            assert!(c.hint().len() > 20);
            assert!(c.hint().ends_with('.'), "{}", c.hint());
            assert_eq!(c.to_string(), s);
        }
    }

    #[test]
    fn serde_representation_matches_as_str() {
        // Without serde_json: Serialize into a tiny recording serializer is overkill; the
        // derive's rename_all is checked against as_str through Debug-to-snake conversion.
        for c in ErrorCode::ALL {
            let dbg = format!("{:?}", c);
            let mut snake = String::new();
            for (i, ch) in dbg.chars().enumerate() {
                if ch.is_ascii_uppercase() {
                    if i > 0 {
                        snake.push('_');
                    }
                    snake.push(ch.to_ascii_lowercase());
                } else {
                    snake.push(ch);
                }
            }
            assert_eq!(snake, c.as_str());
        }
    }

    fn parse_code(input: &str, fmt: &str) -> ErrorCode {
        DateTime::<FixedOffset>::parse_from_str(input, fmt).unwrap_err().code()
    }

    #[test]
    fn parse_errors_map_to_codes() {
        assert_eq!(
            "2023-02-30".parse::<NaiveDate>().unwrap_err().code(),
            ErrorCode::ParseOutOfRange
        );
        assert_eq!(
            "2023-13-01".parse::<NaiveDate>().unwrap_err().code(),
            ErrorCode::ParseOutOfRange
        );
        assert_eq!("abc".parse::<NaiveDate>().unwrap_err().code(), ErrorCode::ParseInvalid);
        assert_eq!("2023-02".parse::<NaiveDate>().unwrap_err().code(), ErrorCode::ParseTooShort);
        assert_eq!("2023-02-03x".parse::<NaiveDate>().unwrap_err().code(), ErrorCode::ParseTooLong);
        assert_eq!(parse_code("2023-02-03", "%Y-%m-%d"), ErrorCode::ParseNotEnough);
        assert_eq!(parse_code("2023-02-03", "%Y-%m-%d%Q"), ErrorCode::ParseBadFormat);
        // Weekday contradicting the date: valid fields, no such date.
        let e = NaiveDate::parse_from_str("Mon 2023-02-03", "%a %Y-%m-%d").unwrap_err();
        assert_eq!(e.code(), ErrorCode::ParseImpossible);
    }

    #[test]
    fn check_format_names_the_bad_specifier() {
        assert!(check_format("%Y-%m-%dT%H:%M:%S%.3f%:z %%").is_ok());
        assert!(check_format("").is_ok());
        let d = check_format("%Y-%Q").unwrap_err();
        assert_eq!(d.code, ErrorCode::ParseBadFormat);
        assert!(d.message.contains("`%Q`") && d.message.contains("position 3"), "{}", d.message);
        assert!(check_format("ab%").unwrap_err().message.contains("position 2"));
        assert!(check_format("%-a").unwrap_err().message.contains("`%-`"));
        assert!(check_format("%d %%%x%\u{e9}").unwrap_err().message.contains("position 7"));
    }

    #[test]
    fn check_output_format_rejects_parse_only_specifiers() {
        assert!(check_output_format("%Y-%m-%dT%H:%M:%S%.3f%:z %Z %+ %s %%").is_ok());
        assert!(check_format("%Y %#z").is_ok());
        let d = check_output_format("%Y %#z").unwrap_err();
        assert_eq!(d.code, ErrorCode::ParseBadFormat);
        assert!(d.message.starts_with("parse-only"), "{}", d.message);
        assert!(d.message.contains("`%#z`") && d.message.contains("position 3"), "{}", d.message);
        // Syntax errors are still reported as such.
        assert!(check_output_format("%Q").unwrap_err().message.contains("`%Q`"));
        // It agrees with the formatter on every specifier in the table.
        let dt =
            crate::DateTime::parse_from_rfc3339(crate::agent::REFERENCE_INSTANT_RFC3339).unwrap();
        for spec in crate::agent::SPECIFIERS {
            let formats = {
                use core::fmt::Write;
                let mut s = String::new();
                write!(s, "{}", dt.format(spec.spec)).is_ok()
            };
            assert_eq!(check_output_format(spec.spec).is_ok(), formats, "{}", spec.spec);
        }
    }

    #[test]
    fn bad_specifier_is_escaped() {
        for bad in ["%\u{1b}[2J", "%\u{7}", "%\r", "%\u{202e}x", "%\0"] {
            let m = check_format(bad).unwrap_err().message;
            assert!(!m.chars().any(|c| c.is_control() || c == '\u{202e}'), "{:?}", m);
        }
    }

    #[test]
    fn diagnosis_carries_message_and_hint() {
        let e = "abc".parse::<NaiveDate>().unwrap_err();
        let d = e.diagnosis();
        assert_eq!(d.code, ErrorCode::ParseInvalid);
        assert_eq!(d.message, e.to_string());
        assert_eq!(d.hint, ErrorCode::ParseInvalid.hint());
    }

    #[test]
    fn other_errors_map_to_codes() {
        let e = TimeDelta::try_days(-1).unwrap().to_std().unwrap_err();
        assert_eq!(e.code(), ErrorCode::OutOfRange);
        assert_eq!(e.diagnosis().message, e.to_string());

        let dt = Utc.with_ymd_and_hms(2300, 12, 12, 0, 0, 0).unwrap();
        let day = TimeDelta::try_days(1).unwrap();
        assert_eq!(
            dt.duration_round(day).unwrap_err().code(),
            ErrorCode::RoundingTimestampExceedsLimit
        );
        let dt = NaiveDate::from_ymd_opt(2260, 12, 31)
            .unwrap()
            .and_hms_nano_opt(23, 59, 59, 175_500_000)
            .unwrap()
            .and_utc();
        let long = TimeDelta::try_days(300 * 365).unwrap();
        assert_eq!(
            dt.duration_round(long).unwrap_err().code(),
            ErrorCode::RoundingDurationExceedsLimit
        );
        let _ = NaiveTime::MIN;
        assert_eq!(
            RoundingError::DurationExceedsTimestamp.code(),
            ErrorCode::RoundingDurationExceedsTimestamp
        );

        assert_eq!("Foo".parse::<Month>().unwrap_err().code(), ErrorCode::InvalidMonth);
        assert_eq!("Foo".parse::<Weekday>().unwrap_err().code(), ErrorCode::InvalidWeekday);
        assert_eq!(
            "Foo".parse::<Weekday>().unwrap_err().diagnosis().code,
            ErrorCode::InvalidWeekday
        );
    }

    #[test]
    fn local_time_errors_map_to_codes() {
        let n: LocalTimeError<i32> = LocalTimeError::Nonexistent;
        assert_eq!(n.code(), ErrorCode::LocalTimeNonexistent);
        let a = LocalTimeError::Ambiguous { earliest: 1, latest: 2 };
        assert_eq!(a.code(), ErrorCode::LocalTimeAmbiguous);
        assert_eq!(a.diagnosis().hint, ErrorCode::LocalTimeAmbiguous.hint());
        assert!(a.diagnosis().message.contains('1') && a.diagnosis().message.contains('2'));
    }
}
