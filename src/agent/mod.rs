//! IronChrono's agent layer: a surface designed for AI agents and other programs that
//! cannot eyeball a failure.
//!
//! Enable it with the `agent` feature. It adds no new behaviour to chrono; it wraps what is
//! there so that it is machine-readable and never guesses:
//!
//! * **Stable error codes** ([`ErrorCode`], [`Diagnose`], [`Diagnosis`]): every chrono error
//!   maps to a `snake_case` id that never changes, plus a one-sentence remediation hint.
//! * **No silent DST guesses** ([`ResolveLocal`], [`LocalPolicy`], [`LocalTimeError`]): a
//!   gap is always an error, a fold is an error unless you name `Earliest` or `Latest`.
//! * **Deterministic time** ([`Clock`], [`SystemClock`], [`FixedClock`], [`SteppingClock`]).
//! * **Self-describing formats** ([`SPECIFIERS`]): a table of every strftime specifier, with
//!   its exact output for [`REFERENCE_INSTANT_RFC3339`], verified by tests against chrono.
//! * **Flat facts** ([`facts`], [`naive_facts`]): ISO week, leap-year, leap-second and offset
//!   data for an instant in one serializable struct.
//!
//! Everything here implements `serde::Serialize`, so a tool can emit it as JSON.
//!
//! ```
//! use chrono::agent::{facts, Diagnose, ErrorCode, LocalPolicy, ResolveLocal};
//! use chrono::{DateTime, FixedOffset, TimeZone};
//!
//! // Errors are codes, not strings.
//! let err = DateTime::<FixedOffset>::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap_err();
//! assert_eq!(err.code(), ErrorCode::ParseNotEnough);
//! assert!(err.hint().contains("%z"));
//!
//! // Local times are resolved under an explicit policy.
//! let tz = FixedOffset::east_opt(3600).unwrap();
//! let dt = tz.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).resolve(LocalPolicy::Reject).unwrap();
//!
//! // And described as plain data.
//! assert_eq!(facts(&dt).offset.as_deref(), Some("+01:00"));
//! ```

mod clock;
mod error;
mod facts;
mod local;
mod spec;

pub use self::clock::{Clock, FixedClock, SteppingClock, SystemClock};
pub use self::error::{Diagnose, Diagnosis, ErrorCode, check_format};
pub use self::facts::{Facts, facts, naive_facts};
pub use self::local::{LocalPolicy, LocalTimeError, ResolveLocal};
pub use self::spec::{REFERENCE_INSTANT_RFC3339, SPECIFIERS, Specifier};

#[cfg(test)]
mod serde_snapshot {
    use super::*;
    use crate::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};

    fn json<T: serde::Serialize>(v: &T) -> std::string::String {
        serde_json::to_string(v).unwrap()
    }

    #[test]
    fn serde_snapshot() {
        let mut out = std::vec::Vec::new();
        for c in ErrorCode::ALL {
            out.push(json(c));
        }
        out.push(json(&check_format("%Y-%Q").unwrap_err()));
        let dt = FixedOffset::east_opt(9 * 3600 + 1800)
            .unwrap()
            .with_ymd_and_hms(2021, 1, 1, 12, 0, 0)
            .unwrap();
        out.push(json(&facts(&dt)));
        out.push(json(&facts(&Utc.with_ymd_and_hms(2016, 12, 31, 23, 59, 59).unwrap())));
        out.push(json(&naive_facts(
            &NaiveDate::from_ymd_opt(2024, 2, 29).unwrap().and_hms_nano_opt(1, 2, 3, 4).unwrap(),
        )));
        for p in [LocalPolicy::Reject, LocalPolicy::Earliest, LocalPolicy::Latest] {
            out.push(json(&p));
        }
        out.push(json(&LocalTimeError::<DateTime<FixedOffset>>::Nonexistent));
        out.push(json(&LocalTimeError::Ambiguous { earliest: dt, latest: dt }));
        out.push(json(&SPECIFIERS[0]));
        let expected: &[&str] = &[
            "\"parse_out_of_range\"",
            "\"parse_impossible\"",
            "\"parse_not_enough\"",
            "\"parse_invalid\"",
            "\"parse_too_short\"",
            "\"parse_too_long\"",
            "\"parse_bad_format\"",
            "\"out_of_range\"",
            "\"rounding_duration_exceeds_timestamp\"",
            "\"rounding_duration_exceeds_limit\"",
            "\"rounding_timestamp_exceeds_limit\"",
            "\"invalid_month\"",
            "\"invalid_weekday\"",
            "\"local_time_nonexistent\"",
            "\"local_time_ambiguous\"",
            "\"missing_offset\"",
            "\"unsupported_time_zone\"",
            "{\"code\":\"parse_bad_format\",\"message\":\"unsupported or malformed format specifier `%Q` at position 3 (byte offset)\",\"hint\":\"The format string is malformed or uses an unsupported specifier or modifier; check it against chrono::agent::SPECIFIERS (the modifiers -, _ and 0 apply to numeric specifiers only).\"}",
            "{\"rfc3339\":\"2021-01-01T12:00:00+09:30\",\"unix_seconds\":1609468200,\"unix_nanos\":1609468200000000000,\"year\":2021,\"month\":1,\"month_name\":\"January\",\"day\":1,\"ordinal\":1,\"iso_year\":2020,\"iso_week\":53,\"weekday\":\"Friday\",\"weekday_from_monday\":4,\"hour\":12,\"minute\":0,\"second\":0,\"nanosecond\":0,\"offset_seconds\":34200,\"offset\":\"+09:30\",\"is_leap_year\":false,\"days_in_month\":31,\"leap_second\":false,\"naive\":false}",
            "{\"rfc3339\":\"2016-12-31T23:59:59+00:00\",\"unix_seconds\":1483228799,\"unix_nanos\":1483228799000000000,\"year\":2016,\"month\":12,\"month_name\":\"December\",\"day\":31,\"ordinal\":366,\"iso_year\":2016,\"iso_week\":52,\"weekday\":\"Saturday\",\"weekday_from_monday\":5,\"hour\":23,\"minute\":59,\"second\":59,\"nanosecond\":0,\"offset_seconds\":0,\"offset\":\"+00:00\",\"is_leap_year\":true,\"days_in_month\":31,\"leap_second\":false,\"naive\":false}",
            "{\"rfc3339\":\"2024-02-29T01:02:03.000000004\",\"unix_seconds\":null,\"unix_nanos\":null,\"year\":2024,\"month\":2,\"month_name\":\"February\",\"day\":29,\"ordinal\":60,\"iso_year\":2024,\"iso_week\":9,\"weekday\":\"Thursday\",\"weekday_from_monday\":3,\"hour\":1,\"minute\":2,\"second\":3,\"nanosecond\":4,\"offset_seconds\":null,\"offset\":null,\"is_leap_year\":true,\"days_in_month\":29,\"leap_second\":false,\"naive\":true}",
            "\"reject\"",
            "\"earliest\"",
            "\"latest\"",
            "\"nonexistent\"",
            "{\"ambiguous\":{\"earliest\":\"2021-01-01T12:00:00+09:30\",\"latest\":\"2021-01-01T12:00:00+09:30\"}}",
            "{\"spec\":\"%Y\",\"meaning\":\"The full proleptic Gregorian year, zero-padded to 4 digits. chrono supports years from -262144 to 262143. Note: years before 1 BCE or after 9999 CE, require an initial sign (+/-).\",\"example\":\"2001\",\"parses\":true,\"group\":\"date\"}",
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn local_policy_deserialize() {
        for (s, p) in [
            ("\"reject\"", LocalPolicy::Reject),
            ("\"earliest\"", LocalPolicy::Earliest),
            ("\"latest\"", LocalPolicy::Latest),
        ] {
            assert_eq!(serde_json::from_str::<LocalPolicy>(s).unwrap(), p);
        }
        for bad in ["\"Reject\"", "\"nope\"", "1", "null"] {
            assert!(serde_json::from_str::<LocalPolicy>(bad).is_err());
        }
        let e = serde_json::from_str::<LocalPolicy>("\"nope\"").unwrap_err().to_string();
        assert!(e.contains("unknown variant `nope`") && e.contains("earliest"), "{e}");
    }
}
