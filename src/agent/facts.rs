//! A flat, serializable description of one instant, so an agent reads fields instead of
//! guessing calendar arithmetic.

use core::fmt;
use std::string::String;

use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::{DateTime, Datelike, NaiveDateTime, Offset, TimeZone, Timelike};

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const WEEKDAY_NAMES: [&str; 7] =
    ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// Everything worth knowing about one instant, precomputed.
///
/// Conventions: `unix_seconds` is whole seconds since the epoch (a leap second counts as the
/// following second's start: it shares `unix_seconds` with `:59` plus one, see `leap_second`);
/// `unix_nanos` is `None` outside roughly the years 1677 to 2262; `weekday_from_monday` is
/// 0 for Monday; `offset` is `+HH:MM` (or `+HH:MM:SS`), UTC being `+00:00`. Naive input never
/// gets an invented instant or offset: see [`naive_facts`].
///
/// ```
/// use chrono::agent::facts;
/// use chrono::{TimeZone, Utc};
///
/// let f = facts(&Utc.with_ymd_and_hms(2021, 1, 1, 12, 0, 0).unwrap());
/// assert_eq!((f.iso_year, f.iso_week), (2020, 53));
/// assert_eq!(f.weekday, "Friday");
/// assert_eq!(f.rfc3339, "2021-01-01T12:00:00+00:00");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    /// RFC 3339 rendering (a leap second shows as `:60`). For [`naive_facts`] it has no offset.
    pub rfc3339: String,
    /// Seconds since 1970-01-01T00:00:00Z (negative before); `None` for naive input.
    pub unix_seconds: Option<i64>,
    /// Nanoseconds since the epoch; `None` if that does not fit in an `i64` or input is naive.
    pub unix_nanos: Option<i64>,
    /// Proleptic Gregorian year (local).
    pub year: i32,
    /// Month 1-12.
    pub month: u32,
    /// English month name.
    pub month_name: &'static str,
    /// Day of month 1-31.
    pub day: u32,
    /// Day of year 1-366.
    pub ordinal: u32,
    /// ISO 8601 week-based year.
    pub iso_year: i32,
    /// ISO 8601 week 1-53.
    pub iso_week: u32,
    /// English weekday name.
    pub weekday: &'static str,
    /// 0 = Monday ... 6 = Sunday.
    pub weekday_from_monday: u32,
    /// Hour 0-23.
    pub hour: u32,
    /// Minute 0-59.
    pub minute: u32,
    /// Second 0-59 (a leap second is reported as 59 with `nanosecond >= 1_000_000_000`).
    pub second: u32,
    /// Nanosecond within the second; at or above 1e9 during a leap second.
    pub nanosecond: u32,
    /// UTC offset in seconds east of UTC; `None` for naive input.
    pub offset_seconds: Option<i32>,
    /// UTC offset as `+HH:MM` (`+HH:MM:SS` if it has seconds; UTC is `+00:00`, never `Z`);
    /// `None` for naive input. This is the only offset format used.
    pub offset: Option<String>,
    /// Whether `year` is a leap year.
    pub is_leap_year: bool,
    /// Days in this month.
    pub days_in_month: u32,
    /// Whether this instant is a leap second.
    pub leap_second: bool,
    /// True when the input had no offset (see [`naive_facts`]); then `unix_seconds`,
    /// `unix_nanos`, `offset_seconds` and `offset` are all `None`.
    pub naive: bool,
}

impl Serialize for Facts {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut st = serializer.serialize_struct("Facts", 22)?;
        st.serialize_field("rfc3339", &self.rfc3339)?;
        st.serialize_field("unix_seconds", &self.unix_seconds)?;
        st.serialize_field("unix_nanos", &self.unix_nanos)?;
        st.serialize_field("year", &self.year)?;
        st.serialize_field("month", &self.month)?;
        st.serialize_field("month_name", &self.month_name)?;
        st.serialize_field("day", &self.day)?;
        st.serialize_field("ordinal", &self.ordinal)?;
        st.serialize_field("iso_year", &self.iso_year)?;
        st.serialize_field("iso_week", &self.iso_week)?;
        st.serialize_field("weekday", &self.weekday)?;
        st.serialize_field("weekday_from_monday", &self.weekday_from_monday)?;
        st.serialize_field("hour", &self.hour)?;
        st.serialize_field("minute", &self.minute)?;
        st.serialize_field("second", &self.second)?;
        st.serialize_field("nanosecond", &self.nanosecond)?;
        st.serialize_field("offset_seconds", &self.offset_seconds)?;
        st.serialize_field("offset", &self.offset)?;
        st.serialize_field("is_leap_year", &self.is_leap_year)?;
        st.serialize_field("days_in_month", &self.days_in_month)?;
        st.serialize_field("leap_second", &self.leap_second)?;
        st.serialize_field("naive", &self.naive)?;
        st.end()
    }
}

fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn format_offset(secs: i32) -> String {
    let sign = if secs < 0 { '-' } else { '+' };
    let a = secs.unsigned_abs();
    let (h, m, s) = (a / 3600, a / 60 % 60, a % 60);
    if s == 0 {
        format!("{}{:02}:{:02}", sign, h, m)
    } else {
        format!("{}{:02}:{:02}:{:02}", sign, h, m, s)
    }
}

fn build(
    local: &NaiveDateTime,
    offset_seconds: Option<i32>,
    offset: Option<String>,
    unix_seconds: Option<i64>,
    unix_nanos: Option<i64>,
    rfc3339: String,
) -> Facts {
    let date = local.date();
    let weekday = date.weekday().num_days_from_monday();
    let iso = date.iso_week();
    let nano = local.nanosecond();
    Facts {
        rfc3339,
        unix_seconds,
        unix_nanos,
        year: date.year(),
        month: date.month(),
        month_name: MONTH_NAMES[date.month0() as usize],
        day: date.day(),
        ordinal: date.ordinal(),
        iso_year: iso.year(),
        iso_week: iso.week(),
        weekday: WEEKDAY_NAMES[weekday as usize],
        weekday_from_monday: weekday,
        hour: local.hour(),
        minute: local.minute(),
        second: local.second(),
        nanosecond: nano,
        offset_seconds,
        offset,
        is_leap_year: is_leap(date.year()),
        days_in_month: days_in_month(date.year(), date.month()),
        leap_second: nano >= 1_000_000_000,
        naive: offset_seconds.is_none(),
    }
}

/// Facts about a zoned instant. The calendar fields are local to the datetime's offset.
pub fn facts<Tz: TimeZone>(dt: &DateTime<Tz>) -> Facts
where
    Tz::Offset: fmt::Display,
{
    let secs = dt.offset().fix().local_minus_utc();
    build(
        &dt.naive_local(),
        Some(secs),
        Some(format_offset(secs)),
        Some(dt.timestamp()),
        dt.timestamp_nanos_opt(),
        dt.to_rfc3339(),
    )
}

/// Facts about a naive (zone-less) datetime. It does not invent an instant: `naive` is true and
/// `unix_seconds`, `unix_nanos`, `offset_seconds` and `offset` are `None`; `rfc3339` is the
/// ISO 8601 text without an offset. Calendar fields (ISO week, leap year, ...) are filled in.
pub fn naive_facts(dt: &NaiveDateTime) -> Facts {
    let rfc3339 = dt.and_utc().to_rfc3339();
    let rfc3339 = match rfc3339.strip_suffix("+00:00") {
        Some(s) => String::from(s),
        None => rfc3339,
    };
    build(dt, None, None, None, None, rfc3339)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FixedOffset, NaiveDate, Utc};

    #[test]
    fn leap_year_day() {
        let f = facts(&Utc.with_ymd_and_hms(2024, 2, 29, 23, 59, 58).unwrap());
        assert_eq!(f.rfc3339, "2024-02-29T23:59:58+00:00");
        assert_eq!(f.unix_seconds, Some(1_709_251_198));
        assert_eq!(f.unix_nanos, Some(1_709_251_198_000_000_000));
        assert_eq!((f.year, f.month, f.day, f.ordinal), (2024, 2, 29, 60));
        assert_eq!(f.month_name, "February");
        assert_eq!(f.weekday, "Thursday");
        assert_eq!(f.weekday_from_monday, 3);
        assert!(f.is_leap_year);
        assert_eq!(f.days_in_month, 29);
        assert_eq!((f.iso_year, f.iso_week), (2024, 9));
        assert_eq!((f.hour, f.minute, f.second, f.nanosecond), (23, 59, 58, 0));
        assert_eq!((f.offset_seconds, f.offset.as_deref()), (Some(0), Some("+00:00")));
        assert!(!f.leap_second && !f.naive);
    }

    #[test]
    fn non_leap_century_years() {
        assert!(!facts(&Utc.with_ymd_and_hms(1900, 3, 1, 0, 0, 0).unwrap()).is_leap_year);
        let f = facts(&Utc.with_ymd_and_hms(2000, 2, 1, 0, 0, 0).unwrap());
        assert!(f.is_leap_year);
        assert_eq!(f.days_in_month, 29);
        assert_eq!(facts(&Utc.with_ymd_and_hms(2023, 2, 1, 0, 0, 0).unwrap()).days_in_month, 28);
        assert_eq!(facts(&Utc.with_ymd_and_hms(2023, 4, 1, 0, 0, 0).unwrap()).days_in_month, 30);
        assert_eq!(facts(&Utc.with_ymd_and_hms(2023, 12, 1, 0, 0, 0).unwrap()).days_in_month, 31);
    }

    #[test]
    fn iso_week_year_boundaries() {
        let f = facts(&Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap());
        assert_eq!((f.year, f.iso_year, f.iso_week), (2021, 2020, 53));
        assert_eq!(f.weekday, "Friday");
        let f = facts(&Utc.with_ymd_and_hms(2024, 12, 30, 0, 0, 0).unwrap());
        assert_eq!((f.year, f.iso_year, f.iso_week), (2024, 2025, 1));
        assert_eq!(f.weekday_from_monday, 0);
        let f = facts(&Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap());
        assert_eq!((f.iso_year, f.iso_week, f.weekday), (2022, 52, "Sunday"));
        assert_eq!(f.weekday_from_monday, 6);
    }

    #[test]
    fn leap_second() {
        let dt = NaiveDate::from_ymd_opt(2016, 12, 31)
            .unwrap()
            .and_hms_nano_opt(23, 59, 59, 1_500_000_000)
            .unwrap()
            .and_utc();
        let f = facts(&dt);
        assert!(f.leap_second);
        assert_eq!((f.second, f.nanosecond), (59, 1_500_000_000));
        assert!(f.rfc3339.starts_with("2016-12-31T23:59:60.5"), "{}", f.rfc3339);
    }

    #[test]
    fn negative_offset_uses_local_fields() {
        let tz = FixedOffset::west_opt(5 * 3600 + 30 * 60).unwrap();
        let f = facts(&tz.with_ymd_and_hms(2024, 1, 1, 0, 15, 0).unwrap());
        assert_eq!((f.offset_seconds, f.offset.as_deref()), (Some(-19800), Some("-05:30")));
        assert_eq!((f.year, f.month, f.day, f.hour, f.minute), (2024, 1, 1, 0, 15));
        assert_eq!(f.rfc3339, "2024-01-01T00:15:00-05:30");
        assert_eq!(f.unix_seconds, Some(1_704_087_900));
        let f = facts(
            &FixedOffset::east_opt(3600 + 30)
                .unwrap()
                .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
                .unwrap(),
        );
        assert_eq!(f.offset.as_deref(), Some("+01:00:30"));
    }

    #[test]
    fn pre_1970_is_negative() {
        let f = facts(&Utc.with_ymd_and_hms(1969, 12, 31, 23, 59, 59).unwrap());
        assert_eq!(f.unix_seconds, Some(-1));
        assert_eq!(f.unix_nanos, Some(-1_000_000_000));
        let f = facts(&Utc.with_ymd_and_hms(1900, 1, 1, 0, 0, 0).unwrap());
        assert_eq!(f.unix_seconds, Some(-2_208_988_800));
        assert_eq!(f.weekday, "Monday");
    }

    #[test]
    fn unix_nanos_is_none_beyond_i64() {
        let f = facts(&Utc.with_ymd_and_hms(2262, 4, 11, 23, 47, 16).unwrap());
        assert!(f.unix_nanos.is_some());
        let f = facts(&Utc.with_ymd_and_hms(2262, 4, 12, 0, 0, 0).unwrap());
        assert_eq!(f.unix_nanos, None);
        assert_eq!(f.unix_seconds, Some(9_223_372_800));
        assert_eq!(facts(&Utc.with_ymd_and_hms(1677, 9, 21, 0, 0, 0).unwrap()).unix_nanos, None);
        assert!(facts(&Utc.with_ymd_and_hms(1677, 9, 22, 0, 0, 0).unwrap()).unix_nanos.is_some());
    }

    #[test]
    fn naive_convention() {
        let n =
            NaiveDate::from_ymd_opt(2001, 7, 8).unwrap().and_hms_milli_opt(0, 34, 59, 250).unwrap();
        let f = naive_facts(&n);
        assert!(f.naive);
        assert_eq!((f.unix_seconds, f.unix_nanos, f.offset_seconds), (None, None, None));
        assert_eq!(f.offset, None);
        assert_eq!(f.rfc3339, "2001-07-08T00:34:59.250");
        assert_eq!(f.weekday, "Sunday");
        assert_eq!((f.iso_year, f.iso_week), (2001, 27));
        assert_eq!(f.nanosecond, 250_000_000);
        let z = facts(&n.and_utc());
        assert!(!z.naive);
        assert_eq!(z.unix_seconds, Some(994_552_499));
        let z = Facts {
            rfc3339: f.rfc3339.clone(),
            naive: true,
            unix_seconds: None,
            unix_nanos: None,
            offset_seconds: None,
            offset: None,
            ..z
        };
        assert_eq!(z, f);
    }
}
