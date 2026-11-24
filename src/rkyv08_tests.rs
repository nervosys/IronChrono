#![allow(clippy::tests_outside_test_module)]
//! Tests of the rkyv 0.8 integration (feature `rkyv-08`): round-trips, and forged archives
//! that must be rejected because they break an invariant of the type.

use core::mem::size_of;

use rkyv_08::api::high::{HighDeserializer, HighSerializer, HighValidator};
use rkyv_08::bytecheck::CheckBytes;
use rkyv_08::rancor::Error;
use rkyv_08::ser::allocator::ArenaHandle;
use rkyv_08::util::AlignedVec;
use rkyv_08::{Archive, Portable};

use crate::rkyv08::{
    ArchivedFixedOffset, ArchivedIsoWeek, ArchivedMonth, ArchivedNaiveDate, ArchivedNaiveDateTime,
    ArchivedNaiveTime, ArchivedTimeDelta, ArchivedWeekday,
};
use crate::{
    DateTime, Datelike, FixedOffset, Month, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta,
    TimeZone, Utc, Weekday,
};

fn ser<T>(value: &T) -> AlignedVec
where
    T: for<'a> rkyv_08::Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, Error>>,
{
    rkyv_08::to_bytes::<Error>(value).unwrap()
}

fn round_trip<T>(value: T)
where
    T: for<'a> rkyv_08::Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, Error>>
        + Archive
        + PartialEq
        + core::fmt::Debug,
    T::Archived: rkyv_08::Deserialize<T, HighDeserializer<Error>>
        + for<'a> CheckBytes<HighValidator<'a, Error>>,
{
    let bytes = ser(&value);
    assert_eq!(rkyv_08::from_bytes::<T, Error>(&bytes).unwrap(), value);
}

/// Overwrites `data` at `offset` from the start of the root object (which `to_bytes` places at
/// the end of the buffer).
fn forge<A>(bytes: &mut AlignedVec, offset: usize, data: &[u8]) {
    let root = bytes.len() - size_of::<A>();
    bytes[root + offset..root + offset + data.len()].copy_from_slice(data);
}

fn accepts<A>(bytes: &[u8]) -> bool
where
    A: for<'a> CheckBytes<HighValidator<'a, Error>> + Portable,
{
    rkyv_08::access::<A, Error>(bytes).is_ok()
}

fn date_yof(date: NaiveDate) -> i32 {
    let bytes = ser(&date);
    i32::from_le_bytes(bytes[..4].try_into().unwrap())
}

#[test]
fn rkyv08_round_trips() {
    round_trip(NaiveDate::MIN);
    round_trip(NaiveDate::MAX);
    round_trip(NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
    round_trip(NaiveTime::MIN);
    round_trip(NaiveTime::from_hms_nano_opt(23, 59, 59, 1_999_999_999).unwrap()); // leap second
    round_trip(NaiveTime::from_hms_nano_opt(23, 59, 59, 999_999_999).unwrap());
    round_trip(NaiveDateTime::MIN);
    round_trip(NaiveDateTime::MAX);
    round_trip(TimeDelta::MIN);
    round_trip(TimeDelta::MAX);
    round_trip(TimeDelta::zero());
    round_trip(TimeDelta::seconds(-1) + TimeDelta::nanoseconds(-1)); // negative, fractional
    round_trip(TimeDelta::nanoseconds(-5));
    round_trip(FixedOffset::east_opt(86_399).unwrap());
    round_trip(FixedOffset::west_opt(86_399).unwrap());
    round_trip(Utc);
    round_trip(Weekday::Sun);
    round_trip(Month::December);
    round_trip(NaiveDate::MIN.iso_week());
    round_trip(NaiveDate::MAX.iso_week());
    round_trip(Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap());
    round_trip(DateTime::<Utc>::MIN_UTC);
    round_trip(DateTime::<Utc>::MAX_UTC);
    let east = FixedOffset::east_opt(3600 * 5 + 1800).unwrap();
    round_trip(east.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap());
}

#[test]
fn rkyv08_archived_compare_and_deserialize() {
    let date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let bytes = ser(&date);
    let archived = rkyv_08::access::<ArchivedNaiveDate, Error>(&bytes).unwrap();
    assert!(*archived == date);
    assert!(*archived < NaiveDate::from_ymd_opt(2000, 1, 2).unwrap());
    assert_eq!(rkyv_08::deserialize::<NaiveDate, Error>(archived).unwrap(), date);
}

#[test]
fn rkyv08_forged_naive_date_is_rejected() {
    let good = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
    let yof = date_yof(good);

    let mut bytes = ser(&good);
    assert!(accepts::<ArchivedNaiveDate>(&bytes));

    // yof == 0: the NonZero check.
    forge::<ArchivedNaiveDate>(&mut bytes, 0, &0i32.to_le_bytes());
    assert!(!accepts::<ArchivedNaiveDate>(&bytes));

    let reject = |yof: i32| {
        let mut b = ser(&good);
        forge::<ArchivedNaiveDate>(&mut b, 0, &yof.to_le_bytes());
        assert!(!accepts::<ArchivedNaiveDate>(&b), "yof {yof:#x} was accepted");
        assert!(rkyv_08::from_bytes::<NaiveDate, Error>(&b).is_err());
    };
    reject(yof & !0x1ff0); // ordinal 0
    reject((yof & !0x1ff0) | (367 << 4)); // ordinal 367
    reject((yof & !0x1ff0) | (511 << 4)); // ordinal 511
    reject(yof & !0xf); // flags 0
    reject(yof ^ 0b0001); // wrong year flags
    reject(((NaiveDate::MAX.year() + 1) << 13) | (yof & 0x1fff)); // year past MAX
    reject(((NaiveDate::MIN.year() - 1) << 13) | (yof & 0x1fff)); // year before MIN
    reject(i32::MAX);
    reject(i32::MIN);
    reject(-1);
}

/// For several years and every ordinal, a forged archive is accepted exactly when the date
/// exists; this ties the validator to the real constructor.
#[test]
fn rkyv08_forged_naive_date_matches_constructor() {
    for year in [-262_144, -400, -1, 0, 1, 1900, 2000, 2023, 2024, 2100, 262_142] {
        let first = match NaiveDate::from_yo_opt(year, 1) {
            Some(d) => d,
            None => continue,
        };
        let flags = date_yof(first) & 0xf;
        for ordinal in 0..=511i32 {
            let yof = (year << 13) | (ordinal << 4) | flags;
            let mut b = ser(&first);
            forge::<ArchivedNaiveDate>(&mut b, 0, &yof.to_le_bytes());
            let expected = ordinal >= 1 && NaiveDate::from_yo_opt(year, ordinal as u32).is_some();
            assert_eq!(accepts::<ArchivedNaiveDate>(&b), expected, "year {year} ordinal {ordinal}");
        }
    }
}

#[test]
fn rkyv08_unchecked_forged_naive_date_does_not_deserialize() {
    let mut bytes = ser(&NaiveDate::from_ymd_opt(2023, 6, 15).unwrap());
    forge::<ArchivedNaiveDate>(&mut bytes, 0, &0x7fff_fff0i32.to_le_bytes());
    // SAFETY: the archive has the right size and alignment; its content is deliberately invalid,
    // which `deserialize` must catch.
    let archived = unsafe { rkyv_08::access_unchecked::<ArchivedNaiveDate>(&bytes) };
    assert!(rkyv_08::deserialize::<NaiveDate, Error>(archived).is_err());
}

#[test]
fn rkyv08_forged_naive_time_is_rejected() {
    let good = NaiveTime::from_hms_nano_opt(12, 30, 30, 5).unwrap();
    let mut bytes = ser(&good);
    assert!(accepts::<ArchivedNaiveTime>(&bytes));

    let forged = |secs: u32, frac: u32| {
        let mut b = ser(&good);
        forge::<ArchivedNaiveTime>(&mut b, 0, &secs.to_le_bytes());
        forge::<ArchivedNaiveTime>(&mut b, 4, &frac.to_le_bytes());
        accepts::<ArchivedNaiveTime>(&b)
    };
    assert!(!forged(86_400, 0));
    assert!(!forged(u32::MAX, 0));
    assert!(!forged(0, 1_000_000_000)); // a leap second only exists in second 59 of a minute
    assert!(!forged(58, 1_500_000_000));
    assert!(!forged(59, 2_000_000_000));
    assert!(!forged(0, u32::MAX));
    // The valid neighbours are accepted.
    assert!(forged(86_399, 1_999_999_999));
    assert!(forged(59, 1_000_000_000));
    assert!(forged(86_399, 0));
    assert!(forged(0, 999_999_999));
    forge::<ArchivedNaiveTime>(&mut bytes, 0, &86_400u32.to_le_bytes());
    assert!(rkyv_08::from_bytes::<NaiveTime, Error>(&bytes).is_err());
}

#[test]
fn rkyv08_forged_time_delta_is_rejected() {
    let good = TimeDelta::seconds(5);
    let forged = |secs: i64, nanos: i32| {
        let mut b = ser(&good);
        forge::<ArchivedTimeDelta>(&mut b, 0, &secs.to_le_bytes());
        forge::<ArchivedTimeDelta>(&mut b, 8, &nanos.to_le_bytes());
        accepts::<ArchivedTimeDelta>(&b)
    };
    assert!(forged(5, 0));
    assert!(forged(-1, 999_999_999));
    assert!(!forged(5, 1_000_000_000));
    assert!(!forged(5, -1));
    assert!(!forged(5, i32::MAX));
    assert!(!forged(i64::MAX, 0));
    assert!(!forged(i64::MIN, 0));
    // One nanosecond past either end of the range.
    let max = ser(&TimeDelta::MAX);
    let min = ser(&TimeDelta::MIN);
    let max_secs = i64::from_le_bytes(max[..8].try_into().unwrap());
    let max_nanos = i32::from_le_bytes(max[8..12].try_into().unwrap());
    let min_secs = i64::from_le_bytes(min[..8].try_into().unwrap());
    let min_nanos = i32::from_le_bytes(min[8..12].try_into().unwrap());
    assert!(forged(max_secs, max_nanos));
    assert!(!forged(max_secs, max_nanos + 1));
    assert!(forged(min_secs, min_nanos));
    assert!(!forged(min_secs, min_nanos - 1));
}

#[test]
fn rkyv08_forged_fixed_offset_is_rejected() {
    let good = FixedOffset::east_opt(3600).unwrap();
    let forged = |secs: i32| {
        let mut b = ser(&good);
        forge::<ArchivedFixedOffset>(&mut b, 0, &secs.to_le_bytes());
        accepts::<ArchivedFixedOffset>(&b)
    };
    assert!(forged(86_399));
    assert!(forged(-86_399));
    assert!(forged(0));
    assert!(!forged(86_400));
    assert!(!forged(-86_400));
    assert!(!forged(i32::MAX));
    assert!(!forged(i32::MIN));
}

#[test]
fn rkyv08_forged_iso_week_is_rejected() {
    let good = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap().iso_week();
    let ywf = {
        let b = ser(&good);
        i32::from_le_bytes(b[..4].try_into().unwrap())
    };
    let forged = |ywf: i32| {
        let mut b = ser(&good);
        forge::<ArchivedIsoWeek>(&mut b, 0, &ywf.to_le_bytes());
        accepts::<ArchivedIsoWeek>(&b)
    };
    assert!(forged(ywf));
    assert!(!forged(ywf & !0x3f0)); // week 0
    assert!(!forged((ywf & !0x3f0) | (54 << 4))); // week 54
    assert!(!forged((ywf & !0x3f0) | (63 << 4)));
    assert!(!forged(ywf ^ 1)); // wrong year flags
    assert!(!forged(i32::MAX));
    assert!(!forged(i32::MIN));
    // 2023 has 52 ISO weeks, so week 53 does not exist.
    assert!(!forged((ywf & !0x3f0) | (53 << 4)));
}

#[test]
fn rkyv08_forged_enum_discriminants_are_rejected() {
    let mut b = ser(&Weekday::Mon);
    assert!(accepts::<ArchivedWeekday>(&b));
    forge::<ArchivedWeekday>(&mut b, 0, &[7]);
    assert!(!accepts::<ArchivedWeekday>(&b));
    let mut b = ser(&Month::January);
    assert!(accepts::<ArchivedMonth>(&b));
    forge::<ArchivedMonth>(&mut b, 0, &[12]);
    assert!(!accepts::<ArchivedMonth>(&b));
}

/// An invalid component nested inside a composite type is found too.
#[test]
fn rkyv08_forged_nested_values_are_rejected() {
    let dt = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap().and_hms_opt(1, 2, 3).unwrap();
    let mut b = ser(&dt);
    assert!(accepts::<ArchivedNaiveDateTime>(&b));
    // `time.secs` sits after the 4 byte date.
    forge::<ArchivedNaiveDateTime>(&mut b, 4, &90_000u32.to_le_bytes());
    assert!(!accepts::<ArchivedNaiveDateTime>(&b));
    assert!(rkyv_08::from_bytes::<NaiveDateTime, Error>(&b).is_err());
    let mut b = ser(&dt);
    forge::<ArchivedNaiveDateTime>(&mut b, 0, &0i32.to_le_bytes());
    assert!(!accepts::<ArchivedNaiveDateTime>(&b));

    let dt = FixedOffset::east_opt(60).unwrap().with_ymd_and_hms(2020, 1, 1, 1, 2, 3).unwrap();
    let mut b = ser(&dt);
    assert!(rkyv_08::from_bytes::<DateTime<FixedOffset>, Error>(&b).is_ok());
    // The offset follows the 12 byte `NaiveDateTime`.
    let root = b.len() - 16;
    b[root + 12..root + 16].copy_from_slice(&90_000i32.to_le_bytes());
    assert!(rkyv_08::from_bytes::<DateTime<FixedOffset>, Error>(&b).is_err());
}

#[test]
fn rkyv08_truncated_archives_are_rejected() {
    let bytes = ser(&NaiveDateTime::MAX);
    assert!(rkyv_08::access::<ArchivedNaiveDateTime, Error>(&bytes[..4]).is_err());
    assert!(rkyv_08::access::<ArchivedNaiveDateTime, Error>(&[]).is_err());
}
