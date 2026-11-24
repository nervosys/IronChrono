//! Explicit resolution of DST gaps and folds.

use core::fmt;

use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, SerializeStructVariant, Serializer};

use crate::MappedLocalTime;

/// What to do when a wall-clock time is ambiguous (a DST fold).
///
/// A nonexistent time (a DST gap) is an error under every policy: there is no honest answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LocalPolicy {
    /// Fail on a fold.
    Reject,
    /// Take the earlier of the two instants.
    Earliest,
    /// Take the later of the two instants.
    Latest,
}

/// Why a local time could not be resolved to one instant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalTimeError<T> {
    /// The time was skipped by a DST transition.
    Nonexistent,
    /// The time occurs twice; both candidates are reported.
    Ambiguous {
        /// The earlier instant.
        earliest: T,
        /// The later instant.
        latest: T,
    },
}

const POLICY_NAMES: &[&str] = &["reject", "earliest", "latest"];

impl Serialize for LocalPolicy {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (idx, name) = match self {
            LocalPolicy::Reject => (0, "reject"),
            LocalPolicy::Earliest => (1, "earliest"),
            LocalPolicy::Latest => (2, "latest"),
        };
        serializer.serialize_unit_variant("LocalPolicy", idx, name)
    }
}

impl<'de> Deserialize<'de> for LocalPolicy {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PolicyVisitor;

        impl<'de> Visitor<'de> for PolicyVisitor {
            type Value = LocalPolicy;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("one of \"reject\", \"earliest\", \"latest\"")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<LocalPolicy, E> {
                match v {
                    "reject" => Ok(LocalPolicy::Reject),
                    "earliest" => Ok(LocalPolicy::Earliest),
                    "latest" => Ok(LocalPolicy::Latest),
                    _ => Err(E::unknown_variant(v, POLICY_NAMES)),
                }
            }
        }

        deserializer.deserialize_str(PolicyVisitor)
    }
}

impl<T: Serialize> Serialize for LocalTimeError<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            LocalTimeError::Nonexistent => {
                serializer.serialize_unit_variant("LocalTimeError", 0, "nonexistent")
            }
            LocalTimeError::Ambiguous { earliest, latest } => {
                let mut sv =
                    serializer.serialize_struct_variant("LocalTimeError", 1, "ambiguous", 2)?;
                sv.serialize_field("earliest", earliest)?;
                sv.serialize_field("latest", latest)?;
                sv.end()
            }
        }
    }
}

impl<T: fmt::Display> fmt::Display for LocalTimeError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocalTimeError::Nonexistent => {
                f.write_str("local time does not exist (skipped by a DST transition)")
            }
            LocalTimeError::Ambiguous { earliest, latest } => {
                write!(f, "local time is ambiguous (DST fold): could be {} or {}", earliest, latest)
            }
        }
    }
}

impl<T: fmt::Debug + fmt::Display> std::error::Error for LocalTimeError<T> {}

/// Turns chrono's [`MappedLocalTime`] into a `Result` under an explicit [`LocalPolicy`].
///
/// ```
/// use chrono::agent::{LocalPolicy, LocalTimeError, ResolveLocal};
/// use chrono::{MappedLocalTime, NaiveDate, TimeDelta};
///
/// let a = NaiveDate::from_ymd_opt(2024, 11, 3).unwrap().and_hms_opt(1, 30, 0).unwrap();
/// let b = a + TimeDelta::try_hours(1).unwrap();
/// let fold = MappedLocalTime::Ambiguous(a, b);
/// assert_eq!(fold.resolve(LocalPolicy::Earliest), Ok(a));
/// assert_eq!(fold.resolve(LocalPolicy::Latest), Ok(b));
/// assert_eq!(
///     fold.resolve(LocalPolicy::Reject),
///     Err(LocalTimeError::Ambiguous { earliest: a, latest: b })
/// );
/// let gap = MappedLocalTime::<i32>::None;
/// assert_eq!(gap.resolve(LocalPolicy::Latest), Err(LocalTimeError::Nonexistent));
/// ```
pub trait ResolveLocal<T> {
    /// Resolve to one value, or say exactly why that is impossible.
    fn resolve(self, policy: LocalPolicy) -> Result<T, LocalTimeError<T>>;
}

impl<T> ResolveLocal<T> for MappedLocalTime<T> {
    fn resolve(self, policy: LocalPolicy) -> Result<T, LocalTimeError<T>> {
        match self {
            MappedLocalTime::Single(t) => Ok(t),
            MappedLocalTime::None => Err(LocalTimeError::Nonexistent),
            MappedLocalTime::Ambiguous(earliest, latest) => match policy {
                LocalPolicy::Reject => Err(LocalTimeError::Ambiguous { earliest, latest }),
                LocalPolicy::Earliest => Ok(earliest),
                LocalPolicy::Latest => Ok(latest),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FixedOffset, TimeZone};

    const POLICIES: [LocalPolicy; 3] =
        [LocalPolicy::Reject, LocalPolicy::Earliest, LocalPolicy::Latest];

    #[test]
    fn single_resolves_under_every_policy() {
        let tz = FixedOffset::east_opt(3600).unwrap();
        for p in POLICIES {
            let r = tz.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).resolve(p).unwrap();
            assert_eq!(r.to_rfc3339(), "2024-06-01T12:00:00+01:00");
        }
    }

    #[test]
    fn gap_is_always_an_error() {
        for p in POLICIES {
            assert_eq!(MappedLocalTime::<u8>::None.resolve(p), Err(LocalTimeError::Nonexistent));
        }
    }

    #[test]
    fn fold_follows_policy() {
        let fold = || MappedLocalTime::Ambiguous(1u8, 2u8);
        assert_eq!(fold().resolve(LocalPolicy::Earliest), Ok(1));
        assert_eq!(fold().resolve(LocalPolicy::Latest), Ok(2));
        assert_eq!(
            fold().resolve(LocalPolicy::Reject),
            Err(LocalTimeError::Ambiguous { earliest: 1, latest: 2 })
        );
        assert_eq!(MappedLocalTime::Single(7u8).resolve(LocalPolicy::Reject), Ok(7));
    }

    #[test]
    fn display_and_error_trait() {
        fn is_error<E: std::error::Error>(_: &E) {}
        let e = LocalTimeError::Ambiguous { earliest: 1, latest: 2 };
        is_error(&e);
        assert!(e.to_string().contains("ambiguous"));
        assert!(LocalTimeError::<u8>::Nonexistent.to_string().contains("does not exist"));
    }
}
