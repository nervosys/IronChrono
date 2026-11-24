#![allow(clippy::tests_outside_test_module)]
//! Tests of rkyv 0.7 validation (feature `rkyv-validation`): a forged archive that passes the
//! byte checks but breaks an invariant of the type must be rejected, as with rkyv 0.8.

use core::mem::size_of;

use rkyv::ser::serializers::AllocSerializer;
use rkyv::util::AlignedVec;
use rkyv::{Archive, Archived};

use crate::{
    DateTime, Datelike, FixedOffset, IsoWeek, Month, NaiveDate, NaiveDateTime, NaiveTime,
    TimeDelta, TimeZone, Timelike, Utc, Weekday,
};

fn ser<T: rkyv::Serialize<AllocSerializer<64>>>(value: &T) -> AlignedVec {
    rkyv::to_bytes::<_, 64>(value).unwrap()
}

fn forge<T: Archive>(bytes: &mut AlignedVec, offset: usize, data: &[u8]) {
    let root = bytes.len() - size_of::<Archived<T>>();
    bytes[root + offset..root + offset + data.len()].copy_from_slice(data);
}

fn accepts<T>(bytes: &[u8]) -> bool
where
    T: Archive,
    Archived<T>: for<'a> rkyv::CheckBytes<rkyv::validation::validators::DefaultValidator<'a>>,
{
    rkyv::check_archived_root::<T>(bytes).is_ok()
}

fn first_i32(bytes: &[u8]) -> i32 {
    i32::from_ne_bytes(bytes[..4].try_into().unwrap())
}

#[test]
fn rkyv07_valid_values_round_trip() {
    fn rt<T>(value: T)
    where
        T: Archive + rkyv::Serialize<AllocSerializer<64>> + PartialEq + core::fmt::Debug,
        Archived<T>: for<'a> rkyv::CheckBytes<rkyv::validation::validators::DefaultValidator<'a>>
            + rkyv::Deserialize<T, rkyv::de::deserializers::SharedDeserializeMap>,
    {
        let bytes = ser(&value);
        assert_eq!(rkyv::from_bytes::<T>(&bytes).unwrap(), value);
    }
    rt(NaiveDate::MIN);
    rt(NaiveDate::MAX);
    rt(NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
    rt(NaiveTime::MIN);
    rt(NaiveTime::from_hms_nano_opt(23, 59, 59, 1_999_999_999).unwrap());
    rt(NaiveDateTime::MIN);
    rt(NaiveDateTime::MAX);
    rt(TimeDelta::MIN);
    rt(TimeDelta::MAX);
    rt(TimeDelta::seconds(-1) + TimeDelta::nanoseconds(-1));
    rt(FixedOffset::east_opt(86_399).unwrap());
    rt(FixedOffset::west_opt(86_399).unwrap());
    rt(NaiveDate::MIN.iso_week());
    rt(NaiveDate::MAX.iso_week());
    rt(Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap());
    rt(DateTime::<Utc>::MIN_UTC);
    rt(DateTime::<Utc>::MAX_UTC);
    rt(FixedOffset::east_opt(3600).unwrap().with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap());
    rt(Weekday::Sun);
    rt(Month::December);
}

/// Before the fix this archive passed `from_bytes`, and `month()` panicked.
#[test]
fn rkyv07_forged_ordinal_511_is_rejected() {
    let good = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
    let mut bytes = ser(&good);
    let yof = first_i32(&bytes);
    let bad = (yof & !0x1ff0) | (511 << 4);
    forge::<NaiveDate>(&mut bytes, 0, &bad.to_ne_bytes());
    assert!(rkyv::from_bytes::<NaiveDate>(&bytes).is_err());
    assert!(!accepts::<NaiveDate>(&bytes));
}

/// Before the fix this archive passed `from_bytes`, and `hour()` returned 25.
#[test]
fn rkyv07_forged_secs_90000_is_rejected() {
    let mut bytes = ser(&NaiveTime::from_hms_opt(1, 2, 3).unwrap());
    forge::<NaiveTime>(&mut bytes, 0, &90_000u32.to_ne_bytes());
    assert!(rkyv::from_bytes::<NaiveTime>(&bytes).is_err());
    let _ = NaiveTime::MIN.hour();
}

#[test]
fn rkyv07_forged_naive_date_is_rejected() {
    let good = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
    let yof = first_i32(&ser(&good));
    let rejected = |yof: i32| {
        let mut b = ser(&good);
        forge::<NaiveDate>(&mut b, 0, &yof.to_ne_bytes());
        !accepts::<NaiveDate>(&b) && rkyv::from_bytes::<NaiveDate>(&b).is_err()
    };
    assert!(rejected(0));
    assert!(rejected(yof & !0x1ff0)); // ordinal 0
    assert!(rejected((yof & !0x1ff0) | (367 << 4)));
    assert!(rejected(yof & !0xf)); // flags 0
    assert!(rejected(yof ^ 1)); // wrong year flags
    assert!(rejected(((NaiveDate::MAX.year() + 1) << 13) | (yof & 0x1fff)));
    assert!(rejected(((NaiveDate::MIN.year() - 1) << 13) | (yof & 0x1fff)));
    assert!(rejected(i32::MAX));
    assert!(rejected(i32::MIN));
    assert!(rejected(-1));
}

#[test]
fn rkyv07_forged_naive_date_matches_constructor() {
    for year in [-262_144, -400, -1, 0, 1, 1900, 2000, 2023, 2024, 2100, 262_142] {
        let Some(first) = NaiveDate::from_yo_opt(year, 1) else { continue };
        let flags = first_i32(&ser(&first)) & 0xf;
        for ordinal in 0..=511i32 {
            let yof = (year << 13) | (ordinal << 4) | flags;
            let mut b = ser(&first);
            forge::<NaiveDate>(&mut b, 0, &yof.to_ne_bytes());
            let expected = ordinal >= 1 && NaiveDate::from_yo_opt(year, ordinal as u32).is_some();
            assert_eq!(accepts::<NaiveDate>(&b), expected, "year {year} ordinal {ordinal}");
        }
    }
}

#[test]
fn rkyv07_forged_naive_time_is_rejected() {
    let good = NaiveTime::from_hms_nano_opt(12, 30, 30, 5).unwrap();
    let ok = |secs: u32, frac: u32| {
        let mut b = ser(&good);
        forge::<NaiveTime>(&mut b, 0, &secs.to_ne_bytes());
        forge::<NaiveTime>(&mut b, 4, &frac.to_ne_bytes());
        accepts::<NaiveTime>(&b)
    };
    assert!(!ok(86_400, 0));
    assert!(!ok(u32::MAX, 0));
    assert!(!ok(0, 1_000_000_000));
    assert!(!ok(58, 1_500_000_000));
    assert!(!ok(59, 2_000_000_000));
    assert!(!ok(0, u32::MAX));
    assert!(ok(86_399, 1_999_999_999));
    assert!(ok(59, 1_000_000_000));
    assert!(ok(86_399, 0));
    assert!(ok(0, 999_999_999));
}

#[test]
fn rkyv07_forged_time_delta_is_rejected() {
    let good = TimeDelta::seconds(5);
    let ok = |secs: i64, nanos: i32| {
        let mut b = ser(&good);
        forge::<TimeDelta>(&mut b, 0, &secs.to_ne_bytes());
        forge::<TimeDelta>(&mut b, 8, &nanos.to_ne_bytes());
        accepts::<TimeDelta>(&b)
    };
    assert!(ok(5, 0));
    assert!(ok(-1, 999_999_999));
    assert!(!ok(5, 1_000_000_000));
    assert!(!ok(5, -1));
    assert!(!ok(5, i32::MAX));
    assert!(!ok(i64::MAX, 0));
    assert!(!ok(i64::MIN, 0));
    let max = ser(&TimeDelta::MAX);
    let min = ser(&TimeDelta::MIN);
    let (max_secs, max_nanos) = (
        i64::from_ne_bytes(max[..8].try_into().unwrap()),
        i32::from_ne_bytes(max[8..12].try_into().unwrap()),
    );
    let (min_secs, min_nanos) = (
        i64::from_ne_bytes(min[..8].try_into().unwrap()),
        i32::from_ne_bytes(min[8..12].try_into().unwrap()),
    );
    assert!(ok(max_secs, max_nanos));
    assert!(!ok(max_secs, max_nanos + 1));
    assert!(ok(min_secs, min_nanos));
    assert!(!ok(min_secs, min_nanos - 1));
}

#[test]
fn rkyv07_forged_fixed_offset_is_rejected() {
    let good = FixedOffset::east_opt(3600).unwrap();
    let ok = |secs: i32| {
        let mut b = ser(&good);
        forge::<FixedOffset>(&mut b, 0, &secs.to_ne_bytes());
        accepts::<FixedOffset>(&b)
    };
    assert!(ok(86_399));
    assert!(ok(-86_399));
    assert!(!ok(86_400));
    assert!(!ok(-86_400));
    assert!(!ok(i32::MAX));
    assert!(!ok(i32::MIN));
}

#[test]
fn rkyv07_forged_iso_week_is_rejected() {
    let good: IsoWeek = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap().iso_week();
    let ywf = first_i32(&ser(&good));
    let ok = |ywf: i32| {
        let mut b = ser(&good);
        forge::<IsoWeek>(&mut b, 0, &ywf.to_ne_bytes());
        accepts::<IsoWeek>(&b)
    };
    assert!(ok(ywf));
    assert!(!ok(ywf & !0x3f0)); // week 0
    assert!(!ok((ywf & !0x3f0) | (54 << 4)));
    assert!(!ok((ywf & !0x3f0) | (53 << 4))); // 2023 has 52 weeks
    assert!(!ok(ywf ^ 1));
    assert!(!ok(i32::MAX));
    assert!(!ok(i32::MIN));
}

/// Composite types inherit the checks through their fields.
#[test]
fn rkyv07_forged_nested_values_are_rejected() {
    let dt = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap().and_hms_opt(1, 2, 3).unwrap();
    let mut b = ser(&dt);
    assert!(accepts::<NaiveDateTime>(&b));
    forge::<NaiveDateTime>(&mut b, 4, &90_000u32.to_ne_bytes());
    assert!(rkyv::from_bytes::<NaiveDateTime>(&b).is_err());
    let mut b = ser(&dt);
    forge::<NaiveDateTime>(&mut b, 0, &0i32.to_ne_bytes());
    assert!(rkyv::from_bytes::<NaiveDateTime>(&b).is_err());

    let dt = FixedOffset::east_opt(60).unwrap().with_ymd_and_hms(2020, 1, 1, 1, 2, 3).unwrap();
    let mut b = ser(&dt);
    assert!(rkyv::from_bytes::<DateTime<FixedOffset>>(&b).is_ok());
    let root = b.len() - size_of::<Archived<DateTime<FixedOffset>>>();
    b[root + 12..root + 16].copy_from_slice(&90_000i32.to_ne_bytes());
    assert!(rkyv::from_bytes::<DateTime<FixedOffset>>(&b).is_err());
}
