#![no_main]
use chrono::agent::*;
use chrono::prelude::*;
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: (String, i64, u32, i32, u8)| {
    let (s, secs, nanos, off, pol) = data;
    if let Err(d) = check_format(&s) {
        let _ = serde_json::to_string(&d);
    }
    if let Some(dt) = DateTime::from_timestamp(secs, nanos % 2_000_000_000) {
        let f = facts(&dt);
        let _ = serde_json::to_string(&f);
        let _ = naive_facts(&dt.naive_utc());
        if let Some(o) = FixedOffset::east_opt(off) {
            let l = dt.with_timezone(&o);
            let _ = facts(&l);
            let p =
                [LocalPolicy::Reject, LocalPolicy::Earliest, LocalPolicy::Latest][pol as usize % 3];
            let r = o.from_local_datetime(&dt.naive_utc()).resolve(p);
            let _ = r.map(|x| facts(&x));
        }
        // format with attacker string, only if valid
        // A format check_output_format accepts must format every DateTime without error.
        if check_output_format(&s).is_ok() {
            use std::fmt::Write;
            let mut out = String::new();
            write!(out, "{}", dt.with_timezone(&FixedOffset::east_opt(0).unwrap()).format(&s))
                .expect("check_output_format accepted a format that fails");
        }
    }
    let _ = DateTime::parse_from_str(&s, &s);
    if let Err(e) = s.parse::<DateTime<Utc>>() {
        let _ = e.diagnosis();
    }
    if let Err(e) = s.parse::<NaiveDate>() {
        let _ = e.diagnosis();
    }
    let _ = serde_json::from_str::<LocalPolicy>(&s);
});
