#![no_main]
use chrono::TimeDelta;
use chrono::format::{Item, StrftimeItems};
use chrono::prelude::*;
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: (String, String, i64, i64)| {
    let (a, f, x, y) = data;
    let _ = NaiveDate::parse_from_str(&a, &f);
    let _ = NaiveTime::parse_from_str(&a, &f);
    let _ = NaiveDateTime::parse_from_str(&a, &f);
    let _ = a.parse::<NaiveDate>();
    let _ = a.parse::<NaiveTime>();
    let _ = a.parse::<NaiveDateTime>();
    let _ = a.parse::<DateTime<Utc>>();
    let _ = a.parse::<DateTime<FixedOffset>>();
    let _ = a.parse::<DateTime<Local>>();
    let _ = a.parse::<Weekday>();
    let _ = a.parse::<Month>();
    let _ = a.parse::<FixedOffset>();
    let _ = DateTime::parse_from_rfc2822(&a);
    let _ = DateTime::parse_from_rfc3339(&a);
    let _ = serde_json::from_str::<DateTime<Utc>>(&a);
    let _ = serde_json::from_str::<NaiveDateTime>(&a);
    let _ = serde_json::from_str::<NaiveDate>(&a);
    let _ = serde_json::from_str::<NaiveTime>(&a);
    // format with attacker format
    if StrftimeItems::new(&f).all(|i| i != Item::Error) {
        let dt = DateTime::from_timestamp(x, 0);
        if let Some(dt) = dt {
            use std::fmt::Write;
            let mut o = String::new();
            let _ = write!(o, "{}", dt.format(&f));
        }
    }
    // constructors that must not panic
    let _ = DateTime::from_timestamp(x, y as u32);
    let _ = DateTime::from_timestamp_millis(x);
    let _ = DateTime::from_timestamp_micros(x);
    let _ = TimeDelta::try_seconds(x);
    let _ = TimeDelta::try_milliseconds(x);
    let _ = TimeDelta::try_days(x);
    let _ = TimeDelta::try_weeks(x);
    let _ = TimeDelta::new(x, y as u32);
    let _ = NaiveDate::from_num_days_from_ce_opt(x as i32);
    let _ = NaiveDate::from_yo_opt(x as i32, y as u32);
    let _ = NaiveDate::from_isoywd_opt(x as i32, y as u32, Weekday::Mon);
    if let (Some(d), Some(td)) = (NaiveDate::from_ymd_opt(2000, 1, 1), TimeDelta::try_seconds(x)) {
        let _ = d.checked_add_signed(td);
        let _ = d.checked_sub_signed(td);
    }
    if let Some(dt) = DateTime::from_timestamp(x, 0) {
        let _ = dt.checked_add_months(chrono::Months::new(y as u32));
        let _ = dt.checked_sub_days(chrono::Days::new(y as u64));
        if (0..=9999).contains(&dt.year()) {
            let _ = dt.to_rfc2822();
        }
        let _ = dt.to_rfc3339();
        let _ = dt.timestamp_nanos_opt();
    }
});
