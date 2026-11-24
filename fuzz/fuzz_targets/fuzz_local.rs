#![no_main]
//! Exercises the platform `Local` time zone through the public API only.
use chrono::prelude::*;
use chrono::{Duration, LocalResult, NaiveDateTime};
use libfuzzer_sys::fuzz_target;

fn check_offset(secs: i32) {
    assert!(secs.abs() <= 24 * 3600, "offset out of range: {}", secs);
}

fn check_local(naive: NaiveDateTime) {
    let r = Local.from_local_datetime(&naive);
    match r {
        LocalResult::Single(dt) => {
            assert_eq!(dt.naive_local(), naive);
            check_offset(dt.offset().local_minus_utc());
        }
        LocalResult::Ambiguous(a, b) => {
            assert_eq!(a.naive_local(), naive);
            assert_eq!(b.naive_local(), naive);
            assert!(a <= b);
            check_offset(a.offset().local_minus_utc());
            check_offset(b.offset().local_minus_utc());
        }
        LocalResult::None => {}
    }
    let _ = naive.and_local_timezone(Local);
    // from_utc_datetime round trip
    let dt = Local.from_utc_datetime(&naive);
    assert_eq!(dt.naive_utc(), naive);
    check_offset(dt.offset().local_minus_utc());
    assert_eq!(Local.offset_from_utc_datetime(&naive).local_minus_utc(), dt.offset().local_minus_utc());
}

fuzz_target!(|data: (i64, u32, i32, u32, u32, u8)| {
    let (secs, nanos, year, ordinal, sod, policy) = data;

    if let LocalResult::Single(dt) = Local.timestamp_opt(secs, nanos) {
        check_offset(dt.offset().local_minus_utc());
        assert_eq!(dt.timestamp(), secs);
        // naive_local panics by documentation when out of NaiveDateTime range; guard it.
        let off = Duration::seconds(dt.offset().local_minus_utc() as i64);
        if dt.naive_utc().checked_add_signed(off).is_some() {
            let _ = dt.naive_local();
        }
    }
    if let Some(utc) = DateTime::from_timestamp(secs, nanos) {
        let l = utc.with_timezone(&Local);
        assert_eq!(l.naive_utc(), utc.naive_utc());
        assert_eq!(l, utc);
        check_offset(l.offset().local_minus_utc());
        check_local(utc.naive_utc());
    }

    // Years far outside 1601..=30827, from the fuzzed year and fixed extremes.
    let mut years = vec![year, 0, -1, 1600, 1601, 30827, 30828, 262143, -262144, -262143];
    years.push(year.rem_euclid(600_000) - 300_000);
    for y in years {
        if let Some(d) = NaiveDate::from_yo_opt(y, 1 + ordinal % 366) {
            if let Some(n) = d.and_hms_opt(sod / 3600 % 24, sod / 60 % 60, sod % 60) {
                check_local(n);
                // neighbours across transitions
                let step = match policy % 4 { 0 => 1, 1 => 3600, 2 => 86400, _ => 7 * 86400 };
                for k in [-2i64, -1, 1, 2] {
                    if let Some(m) = n.checked_add_signed(Duration::seconds(step * k)) {
                        check_local(m);
                    }
                }
            }
        }
    }

    // NaiveDateTime::MIN/MAX neighbours.
    for base in [NaiveDateTime::MIN, NaiveDateTime::MAX] {
        for k in [0i64, 1, 3599, 3600, 86400, secs % 400_000_000] {
            let m = base.checked_add_signed(Duration::seconds(k)).or_else(|| base.checked_sub_signed(Duration::seconds(k)));
            if let Some(m) = m {
                check_local(m);
            }
        }
        let _ = Local.from_local_datetime(&base);
        let _ = base.and_local_timezone(Local);
    }
});
