// Generated from rustdoc JSON of ironchrono (features serde,clock,unstable-locales,rkyv-32,arbitrary).
// Every public path of the real crate, named through the shim.
#![allow(unused_imports, deprecated)]

use chrono::Date as _;
use chrono::DateTime as _;
use chrono::Datelike as _;
use chrono::Days as _;
use chrono::Duration as _;
use chrono::DurationRound as _;
use chrono::FixedOffset as _;
use chrono::IsoWeek as _;
use chrono::Local as _;
use chrono::Locale as _;
use chrono::MAX_DATE as _;
use chrono::MAX_DATETIME as _;
use chrono::MIN_DATE as _;
use chrono::MIN_DATETIME as _;
use chrono::MappedLocalTime as _;
use chrono::Month as _;
use chrono::Months as _;
use chrono::NaiveDate as _;
use chrono::NaiveDateTime as _;
use chrono::NaiveTime as _;
use chrono::NaiveWeek as _;
use chrono::Offset as _;
use chrono::OutOfRange as _;
use chrono::OutOfRangeError as _;
use chrono::ParseError as _;
use chrono::ParseMonthError as _;
use chrono::ParseResult as _;
use chrono::ParseWeekdayError as _;
use chrono::RoundingError as _;
use chrono::SecondsFormat as _;
use chrono::SubsecRound as _;
use chrono::TimeDelta as _;
use chrono::TimeZone as _;
use chrono::Timelike as _;
use chrono::Utc as _;
use chrono::Weekday as _;
use chrono::WeekdaySet as _;
use chrono::format as _;
use chrono::format::Colons as _;
use chrono::format::DelayedFormat as _;
use chrono::format::Fixed as _;
use chrono::format::InternalFixed as _;
use chrono::format::InternalNumeric as _;
use chrono::format::Item as _;
use chrono::format::Locale as _;
use chrono::format::Numeric as _;
use chrono::format::OffsetFormat as _;
use chrono::format::OffsetPrecision as _;
use chrono::format::Pad as _;
use chrono::format::ParseError as _;
use chrono::format::ParseErrorKind as _;
use chrono::format::ParseResult as _;
use chrono::format::Parsed as _;
use chrono::format::SecondsFormat as _;
use chrono::format::StrftimeItems as _;
use chrono::format::format as _;
use chrono::format::format_item as _;
use chrono::format::parse as _;
use chrono::format::parse_and_remainder as _;
use chrono::format::strftime as _;
use chrono::format::strftime::StrftimeItems as _;
use chrono::naive as _;
use chrono::naive::Days as _;
use chrono::naive::IsoWeek as _;
use chrono::naive::MAX_DATE as _;
use chrono::naive::MAX_DATETIME as _;
use chrono::naive::MIN_DATE as _;
use chrono::naive::MIN_DATETIME as _;
use chrono::naive::NaiveDate as _;
use chrono::naive::NaiveDateDaysIterator as _;
use chrono::naive::NaiveDateTime as _;
use chrono::naive::NaiveDateWeeksIterator as _;
use chrono::naive::NaiveTime as _;
use chrono::naive::NaiveWeek as _;
use chrono::naive::serde as _;
use chrono::naive::serde::ts_microseconds as _;
use chrono::naive::serde::ts_microseconds::deserialize as _;
use chrono::naive::serde::ts_microseconds::serialize as _;
use chrono::naive::serde::ts_microseconds_option as _;
use chrono::naive::serde::ts_microseconds_option::deserialize as _;
use chrono::naive::serde::ts_microseconds_option::serialize as _;
use chrono::naive::serde::ts_milliseconds as _;
use chrono::naive::serde::ts_milliseconds::deserialize as _;
use chrono::naive::serde::ts_milliseconds::serialize as _;
use chrono::naive::serde::ts_milliseconds_option as _;
use chrono::naive::serde::ts_milliseconds_option::deserialize as _;
use chrono::naive::serde::ts_milliseconds_option::serialize as _;
use chrono::naive::serde::ts_nanoseconds as _;
use chrono::naive::serde::ts_nanoseconds::deserialize as _;
use chrono::naive::serde::ts_nanoseconds::serialize as _;
use chrono::naive::serde::ts_nanoseconds_option as _;
use chrono::naive::serde::ts_nanoseconds_option::deserialize as _;
use chrono::naive::serde::ts_nanoseconds_option::serialize as _;
use chrono::naive::serde::ts_seconds as _;
use chrono::naive::serde::ts_seconds::deserialize as _;
use chrono::naive::serde::ts_seconds::serialize as _;
use chrono::naive::serde::ts_seconds_option as _;
use chrono::naive::serde::ts_seconds_option::deserialize as _;
use chrono::naive::serde::ts_seconds_option::serialize as _;
use chrono::offset as _;
use chrono::offset::FixedOffset as _;
use chrono::offset::Local as _;
use chrono::offset::LocalResult as _;
use chrono::offset::MappedLocalTime as _;
use chrono::offset::Offset as _;
use chrono::offset::TimeZone as _;
use chrono::offset::Utc as _;
use chrono::prelude as _;
use chrono::prelude::Date as _;
use chrono::prelude::DateTime as _;
use chrono::prelude::Datelike as _;
use chrono::prelude::FixedOffset as _;
use chrono::prelude::Local as _;
use chrono::prelude::Locale as _;
use chrono::prelude::Month as _;
use chrono::prelude::NaiveDate as _;
use chrono::prelude::NaiveDateTime as _;
use chrono::prelude::NaiveTime as _;
use chrono::prelude::Offset as _;
use chrono::prelude::SecondsFormat as _;
use chrono::prelude::SubsecRound as _;
use chrono::prelude::TimeZone as _;
use chrono::prelude::Timelike as _;
use chrono::prelude::Utc as _;
use chrono::prelude::Weekday as _;
use chrono::rkyv as _;
use chrono::rkyv::ArchivedDateTime as _;
use chrono::rkyv::ArchivedDuration as _;
use chrono::rkyv::ArchivedFixedOffset as _;
use chrono::rkyv::ArchivedIsoWeek as _;
use chrono::rkyv::ArchivedLocal as _;
use chrono::rkyv::ArchivedMonth as _;
use chrono::rkyv::ArchivedNaiveDate as _;
use chrono::rkyv::ArchivedNaiveDateTime as _;
use chrono::rkyv::ArchivedNaiveTime as _;
use chrono::rkyv::ArchivedTimeDelta as _;
use chrono::rkyv::ArchivedUtc as _;
use chrono::rkyv::ArchivedWeekday as _;
use chrono::round as _;
use chrono::round::DurationRound as _;
use chrono::round::RoundingError as _;
use chrono::round::SubsecRound as _;
use chrono::serde as _;
use chrono::serde::ts_microseconds as _;
use chrono::serde::ts_microseconds::deserialize as _;
use chrono::serde::ts_microseconds::serialize as _;
use chrono::serde::ts_microseconds_option as _;
use chrono::serde::ts_microseconds_option::deserialize as _;
use chrono::serde::ts_microseconds_option::serialize as _;
use chrono::serde::ts_milliseconds as _;
use chrono::serde::ts_milliseconds::deserialize as _;
use chrono::serde::ts_milliseconds::serialize as _;
use chrono::serde::ts_milliseconds_option as _;
use chrono::serde::ts_milliseconds_option::deserialize as _;
use chrono::serde::ts_milliseconds_option::serialize as _;
use chrono::serde::ts_nanoseconds as _;
use chrono::serde::ts_nanoseconds::deserialize as _;
use chrono::serde::ts_nanoseconds::serialize as _;
use chrono::serde::ts_nanoseconds_option as _;
use chrono::serde::ts_nanoseconds_option::deserialize as _;
use chrono::serde::ts_nanoseconds_option::serialize as _;
use chrono::serde::ts_seconds as _;
use chrono::serde::ts_seconds::deserialize as _;
use chrono::serde::ts_seconds::serialize as _;
use chrono::serde::ts_seconds_option as _;
use chrono::serde::ts_seconds_option::deserialize as _;
use chrono::serde::ts_seconds_option::serialize as _;

// the prelude, by name

// ---- identity: the shim's types ARE ironchrono's types ----
// (`ironchrono` is a dev-dependency; its library name is `chrono`, so it is
// renamed to `iron`.)
fn takes_iron_utc(d: iron::DateTime<iron::Utc>) -> i64 {
    d.timestamp()
}
fn takes_shim_utc(d: chrono::DateTime<chrono::Utc>) -> i64 {
    d.timestamp()
}

#[test]
fn shim_and_ironchrono_share_types() {
    let shim: chrono::DateTime<chrono::Utc> = chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    assert_eq!(takes_iron_utc(shim), 1_700_000_000);
    let iron: iron::DateTime<iron::Utc> = iron::DateTime::from_timestamp(5, 0).unwrap();
    assert_eq!(takes_shim_utc(iron), 5);
    let n: iron::NaiveDateTime = chrono::NaiveDate::from_ymd_opt(2024, 2, 29).unwrap().and_hms_opt(1, 2, 3).unwrap();
    let _: chrono::NaiveDateTime = n;
    let _: iron::TimeDelta = chrono::Duration::seconds(3);
    assert_eq!(std::any::TypeId::of::<chrono::Utc>(), std::any::TypeId::of::<iron::Utc>());
}

// ---- the prelude, by glob, and the traits it carries ----
mod prelude_glob {
    use chrono::prelude::*;
    #[test]
    fn prelude_works() {
        let d = Utc.with_ymd_and_hms(2024, 3, 1, 12, 0, 0).unwrap();
        assert_eq!(d.year(), 2024);
        assert_eq!(d.hour(), 12);
        assert_eq!(d.weekday(), Weekday::Fri);
        assert_eq!(d.format("%Y-%m-%d").to_string(), "2024-03-01");
        let _ = FixedOffset::east_opt(3600).unwrap();
        let _ = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let _ = Month::January;
        let _ = d.to_rfc3339_opts(SecondsFormat::Millis, true);
        let _ = d.trunc_subsecs(0);
        let _: Locale = Locale::en_US;
    }
}

// ---- ts_* serde helpers do work through the shim ----
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct Ts {
    #[serde(with = "chrono::serde::ts_seconds")]
    s: chrono::DateTime<chrono::Utc>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    ms: chrono::DateTime<chrono::Utc>,
    #[serde(with = "chrono::serde::ts_microseconds")]
    us: chrono::DateTime<chrono::Utc>,
    #[serde(with = "chrono::serde::ts_nanoseconds")]
    ns: chrono::DateTime<chrono::Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    so: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(with = "chrono::serde::ts_milliseconds_option")]
    mo: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(with = "chrono::serde::ts_microseconds_option")]
    uo: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(with = "chrono::serde::ts_nanoseconds_option")]
    no: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(with = "chrono::naive::serde::ts_seconds")]
    n: chrono::NaiveDateTime,
}

#[test]
fn serde_ts_helpers_round_trip() {
    let d = chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    let t = Ts { s: d, ms: d, us: d, ns: d, so: Some(d), mo: None, uo: Some(d), no: None, n: d.naive_utc() };
    let json = serde_json::to_string(&t).unwrap();
    assert_eq!(serde_json::from_str::<Ts>(&json).unwrap(), t);
}

#[test]
fn misc_items() {
    let _ = chrono::Months::new(3);
    let _ = chrono::Days::new(2);
    let ws = chrono::WeekdaySet::from_array([chrono::Weekday::Mon, chrono::Weekday::Sun]);
    assert_eq!(ws.len(), 2);
    let r: chrono::MappedLocalTime<u8> = chrono::LocalResult::Single(1);
    assert_eq!(r.single(), Some(1));
    #[allow(deprecated)]
    {
        let _ = (chrono::MIN_DATE, chrono::MAX_DATE);
    }
    let _ = chrono::MIN_DATETIME;
    let items: Vec<chrono::format::Item> = chrono::format::strftime::StrftimeItems::new("%Y").collect();
    let mut p = chrono::format::Parsed::new();
    chrono::format::parse(&mut p, "2024", items.iter()).unwrap();
    assert_eq!(p.year, Some(2024));
    let w = chrono::NaiveDate::from_ymd_opt(2024, 5, 5).unwrap().week(chrono::Weekday::Mon);
    let _: chrono::naive::NaiveWeek = w;
    let _: chrono::naive::IsoWeek = chrono::NaiveDate::from_ymd_opt(2024, 5, 5).unwrap().iso_week();
}
