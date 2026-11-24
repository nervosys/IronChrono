//! Deterministic sweep of the Windows `Local` path against the current system time zone.
//! Does not change system settings.
#![cfg(windows)]

use chrono::{DateTime, Local, LocalResult, NaiveDate, NaiveDateTime, TimeZone};

fn check_offset(s: i32) {
    assert!(s.abs() <= 24 * 3600, "offset {s}");
}

fn check(naive: NaiveDateTime) -> (u8, u8) {
    let mut kind = (0, 0);
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) => {
            assert_eq!(dt.naive_local(), naive);
            check_offset(dt.offset().local_minus_utc());
        }
        LocalResult::Ambiguous(a, b) => {
            assert_eq!(a.naive_local(), naive);
            assert_eq!(b.naive_local(), naive);
            assert!(a < b, "ambiguous order {a:?} {b:?} for {naive}");
            kind.0 = 1;
        }
        LocalResult::None => kind.1 = 1,
    }
    let dt = Local.from_utc_datetime(&naive);
    assert_eq!(dt.naive_utc(), naive);
    check_offset(dt.offset().local_minus_utc());
    // Round trip: a UTC instant converted to local, then back from local, must contain it.
    // `naive_local` panics (documented) if the offset pushes it out of NaiveDateTime's range.
    let off = chrono::Duration::seconds(dt.offset().local_minus_utc() as i64);
    let Some(local) = naive.checked_add_signed(off) else { return kind };
    match Local.from_local_datetime(&local) {
        LocalResult::Single(b) => assert_eq!(b.naive_utc(), naive, "round trip {naive}"),
        LocalResult::Ambiguous(a, b) => {
            assert!(a.naive_utc() == naive || b.naive_utc() == naive, "round trip {naive}")
        }
        LocalResult::None => panic!("local time of an existing instant is None: {naive}"),
    }
    kind
}

#[test]
fn sweep_1600_to_2100() {
    let start = std::time::Instant::now();
    let (mut n, mut amb, mut gap) = (0u64, 0u64, 0u64);
    for year in 1600..=2100 {
        let mut d = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
        let mut day = 0;
        while d.year_is(year) {
            if day % 7 == 0 || is_near_transition(d) {
                for h in 0..24 {
                    for m in [0u32, 30] {
                        let (a, g) = check(d.and_hms_opt(h, m, 0).unwrap());
                        n += 1;
                        amb += a as u64;
                        gap += g as u64;
                    }
                }
            }
            d = d.succ_opt().unwrap();
            day += 1;
        }
    }
    // Far-out years, including those outside the 1601..=30827 API range.
    for year in
        [-262143, -1000, -1, 0, 1, 1599, 1600, 1601, 30826, 30827, 30828, 31000, 100000, 262142]
    {
        for (m, dd) in [(1, 1), (3, 15), (7, 1), (11, 5), (12, 31)] {
            if let Some(d) = NaiveDate::from_ymd_opt(year, m, dd) {
                for h in 0..24 {
                    check(d.and_hms_opt(h, 0, 0).unwrap());
                    n += 1;
                }
            }
        }
    }
    for base in [NaiveDateTime::MIN, NaiveDateTime::MAX] {
        let _ = Local.from_local_datetime(&base);
        let _ = Local.from_utc_datetime(&base);
    }
    for secs in [i64::MIN, i64::MAX, -62_135_596_800, 253_402_300_799, 0] {
        if let Some(utc) = DateTime::from_timestamp(secs, 0) {
            assert_eq!(utc.with_timezone(&Local), utc);
        }
        let _ = Local.timestamp_opt(secs, 0);
    }
    eprintln!("checked {n} local times ({amb} ambiguous, {gap} in gaps) in {:?}", start.elapsed());
}

// Transitions in practice fall in Mar/Apr and Oct/Nov (and Sep/Oct south); sweep every day then.
fn is_near_transition(d: NaiveDate) -> bool {
    use chrono::Datelike;
    matches!(d.month(), 3 | 4 | 9 | 10 | 11)
}

trait YearIs {
    fn year_is(&self, y: i32) -> bool;
}
impl YearIs for NaiveDate {
    fn year_is(&self, y: i32) -> bool {
        chrono::Datelike::year(self) == y
    }
}
