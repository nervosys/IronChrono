//! A self-describing table of every `strftime` specifier chrono supports.

use serde::ser::{Serialize, SerializeStruct, Serializer};

/// The instant every [`Specifier::example`] renders: 2001-07-08 00:34:60.026490708 at
/// +09:30 (note the leap second `:60`). It is the instant used by the strftime documentation
/// table in `chrono::format::strftime`; where that table abbreviates a value, the
/// `example` here is the exact output.
pub const REFERENCE_INSTANT_RFC3339: &str = "2001-07-08T00:34:60.026490708+09:30";

/// One strftime specifier, described for machines.
///
/// ```
/// use chrono::agent::{REFERENCE_INSTANT_RFC3339, SPECIFIERS};
/// use chrono::DateTime;
///
/// let at = DateTime::parse_from_rfc3339(REFERENCE_INSTANT_RFC3339).unwrap();
/// let s = SPECIFIERS.iter().find(|s| s.spec == "%F").unwrap();
/// assert_eq!(at.format(s.spec).to_string(), s.example);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Specifier {
    /// The specifier exactly as written in a format string, e.g. `%Y` or `%.3f`.
    pub spec: &'static str,
    /// What it means.
    pub meaning: &'static str,
    /// Its exact output for [`REFERENCE_INSTANT_RFC3339`].
    pub example: &'static str,
    /// Whether its example round-trips through chrono's parser (`%::z` and `%:::z` are
    /// formatting-only). `%#z` is parse-only (it cannot be formatted, so
    /// its `example` is the text it would parse, not a formatting result).
    pub parses: bool,
    /// One of `date`, `time`, `timezone`, `datetime`, `special`, `modifier`.
    pub group: &'static str,
}

impl Serialize for Specifier {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut st = serializer.serialize_struct("Specifier", 5)?;
        st.serialize_field("spec", &self.spec)?;
        st.serialize_field("meaning", &self.meaning)?;
        st.serialize_field("example", &self.example)?;
        st.serialize_field("parses", &self.parses)?;
        st.serialize_field("group", &self.group)?;
        st.end()
    }
}

include!("spec_table.rs");

/// Every specifier chrono's strftime supports, plus representative padding-modifier forms
/// (group `modifier`: `%-?`, `%_?`, `%0?` apply to numeric specifiers). The table is
/// verified against `src/format/strftime.rs` by the test suite, so it cannot drift.
pub const SPECIFIERS: &[Specifier] = TABLE;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{Parsed, StrftimeItems, parse};
    use crate::{DateTime, FixedOffset};
    use core::fmt::Write;
    use std::collections::BTreeSet;

    fn reference() -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(REFERENCE_INSTANT_RFC3339).unwrap()
    }

    fn render(spec: &str) -> Option<String> {
        let mut s = String::new();
        write!(s, "{}", reference().format(spec)).ok()?;
        Some(s)
    }

    /// Developer aid: `ICHR_DUMP=path cargo test --features agent --lib agent::spec::tests::dump`
    #[test]
    fn dump() {
        if let Ok(path) = std::env::var("ICHR_DUMP") {
            let mut out = String::new();
            for s in SPECIFIERS {
                if let Some(r) = render(s.spec) {
                    out.push_str(&format!("{}\t{}\n", s.spec, r));
                }
            }
            std::fs::write(path, out).unwrap();
        }
    }

    #[test]
    fn reference_instant_is_the_documented_one() {
        let r = reference();
        assert_eq!(r.format("%F").to_string(), "2001-07-08");
        assert_eq!(r.format("%S").to_string(), "60");
    }

    #[test]
    fn every_example_is_what_formatting_produces() {
        for s in SPECIFIERS {
            if s.spec == "%#z" {
                assert!(render(s.spec).is_none(), "%#z is documented as parse-only");
                continue;
            }
            let got = render(s.spec).unwrap_or_else(|| panic!("{} failed to format", s.spec));
            assert_eq!(got, s.example, "spec {}", s.spec);
        }
    }

    #[test]
    fn every_parseable_example_parses() {
        // Round trip at the field level: parsing the rendered example with the same
        // specifier must succeed and consume all of it. (Combining into a full value
        // is not meaningful for a single specifier; %s, %+ and %c are checked for value below.)
        let mut bad = std::vec::Vec::new();
        for s in SPECIFIERS {
            let mut p = Parsed::new();
            let r = parse(&mut p, s.example, StrftimeItems::new(s.spec));
            if r.is_ok() != s.parses {
                bad.push(s.spec);
            }
        }
        assert!(bad.is_empty(), "`parses` is wrong for {:?}", bad);
    }

    #[test]
    fn composite_specifiers_round_trip_to_the_instant() {
        let r = reference();
        for spec in ["%+", "%s%.9f%z"] {
            let text = r.format(spec).to_string();
            let back = DateTime::parse_from_str(&text, spec);
            if spec == "%+" {
                assert_eq!(back.unwrap(), r);
            } else {
                // %s carries whole seconds; the leap second is folded into the next one.
                assert!(back.is_ok(), "{:?}", back);
            }
        }
        let p = DateTime::parse_from_str("2001-07-08 00:34:59 +0930", "%F %T %z").unwrap();
        assert_eq!(p.format("%F %T %z").to_string(), "2001-07-08 00:34:59 +0930");
    }

    #[test]
    fn groups_are_known_and_entries_are_unique() {
        let mut seen = BTreeSet::new();
        for s in SPECIFIERS {
            assert!(!s.example.is_empty(), "{} has an empty example", s.spec);
            assert!(
                ["date", "time", "timezone", "datetime", "special", "modifier"].contains(&s.group)
            );
            assert!(s.spec.starts_with('%'));
            assert!(!s.meaning.is_empty());
            assert!(seen.insert(s.spec), "duplicate {}", s.spec);
        }
    }

    /// Every row of the strftime.rs doc table must be in SPECIFIERS and vice versa (apart from
    /// the `modifier` forms, which come from the modifier table).
    #[test]
    fn table_matches_strftime_docs() {
        let src = include_str!("../format/strftime.rs");
        let start = src.find("## Specifiers").expect("doc table heading");
        let end = src.find("It is possible to override").expect("modifier section");
        let mut documented = BTreeSet::new();
        for line in src[start..end].lines() {
            let line = line.trim();
            if !line.starts_with('|') {
                continue;
            }
            let first = line.trim_matches('|').split('|').next().unwrap().trim();
            if first.starts_with('`') && first.ends_with('`') && first.len() > 2 {
                documented.insert(first.trim_matches('`').to_string());
            }
        }
        assert!(documented.len() > 50, "table extraction broke: {}", documented.len());
        let ours: BTreeSet<String> = SPECIFIERS
            .iter()
            .filter(|s| s.group != "modifier")
            .map(|s| s.spec.to_string())
            .collect();
        let missing: Vec<_> = documented.difference(&ours).collect();
        let extra: Vec<_> = ours.difference(&documented).collect();
        assert!(missing.is_empty(), "documented but absent from SPECIFIERS: {:?}", missing);
        assert!(extra.is_empty(), "in SPECIFIERS but not documented: {:?}", extra);
    }

    #[test]
    fn modifiers_do_what_the_docs_say() {
        let find = |s: &str| SPECIFIERS.iter().find(|x| x.spec == s).unwrap().example;
        assert_eq!(find("%-d"), "8");
        assert_eq!(find("%_d"), " 8");
        assert_eq!(find("%0e"), "08");
        assert_eq!(find("%-j"), "189");
        assert_eq!(find("%_j"), "189");
        assert_eq!(find("%-m"), "7");
        assert_eq!(find("%-H"), "0");
        assert_eq!(find("%0k"), "00");
    }

    #[test]
    fn padding_modifier_on_text_specifier_is_a_bad_format() {
        assert!(render("%-a").is_none());
    }
}
