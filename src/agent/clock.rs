//! Injectable time sources.

use std::boxed::Box;
use std::sync::{Arc, Mutex};

use crate::{DateTime, TimeDelta, Utc};

/// A source of the current instant. Take `&dyn Clock` or `impl Clock` instead of calling
/// `Utc::now()` so tests can be deterministic.
pub trait Clock {
    /// The current instant.
    fn now(&self) -> DateTime<Utc>;
}

/// The real clock: `Utc::now()`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        crate::Utc::now()
    }
}

/// A clock frozen at one instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedClock(pub DateTime<Utc>);

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

/// A clock that returns `start` on the first call and advances by `step` after each call.
/// Safe to share between threads. If advancing would overflow the representable range the
/// clock stays where it is.
///
/// ```
/// use chrono::agent::{Clock, SteppingClock};
/// use chrono::{TimeDelta, TimeZone, Utc};
///
/// let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
/// let step = TimeDelta::try_seconds(10).unwrap();
/// let c = SteppingClock::new(start, step);
/// assert_eq!(c.now(), start);
/// assert_eq!(c.now(), start + step);
/// ```
#[derive(Debug)]
pub struct SteppingClock {
    next: Mutex<DateTime<Utc>>,
    step: TimeDelta,
}

impl SteppingClock {
    /// A clock whose first reading is `start`, advancing `step` per reading.
    pub fn new(start: DateTime<Utc>, step: TimeDelta) -> Self {
        SteppingClock { next: Mutex::new(start), step }
    }
}

impl Clock for SteppingClock {
    fn now(&self) -> DateTime<Utc> {
        let mut guard = self.next.lock().unwrap_or_else(|e| e.into_inner());
        let current = *guard;
        if let Some(n) = current.checked_add_signed(self.step) {
            *guard = n;
        }
        current
    }
}

impl<C: Clock + ?Sized> Clock for &C {
    fn now(&self) -> DateTime<Utc> {
        (**self).now()
    }
}

impl<C: Clock + ?Sized> Clock for Box<C> {
    fn now(&self) -> DateTime<Utc> {
        (**self).now()
    }
}

impl<C: Clock + ?Sized> Clock for Arc<C> {
    fn now(&self) -> DateTime<Utc> {
        (**self).now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TimeZone;

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()
    }

    #[test]
    fn fixed_is_constant() {
        let c = FixedClock(t0());
        for _ in 0..5 {
            assert_eq!(c.now(), t0());
        }
    }

    #[test]
    fn system_is_after_2024() {
        assert!(SystemClock.now() > t0());
    }

    #[test]
    fn stepping_advances_and_forwards() {
        let step = TimeDelta::try_milliseconds(250).unwrap();
        let c = SteppingClock::new(t0(), step);
        assert_eq!(c.now(), t0());
        assert_eq!(c.now(), t0() + step);
        let r: &dyn Clock = &c;
        assert_eq!(r.now(), t0() + step + step);
        let b: Box<dyn Clock> = Box::new(FixedClock(t0()));
        assert_eq!(b.now(), t0());
        let a = Arc::new(FixedClock(t0()));
        assert_eq!(a.now(), t0());
        // The `&C` impl, named so the call cannot resolve to the Arc impl.
        assert_eq!(<&Arc<FixedClock> as Clock>::now(&&a), t0());
    }

    #[test]
    fn stepping_is_sync_and_exact_across_threads() {
        fn assert_sync<T: Sync + Send>() {}
        assert_sync::<SteppingClock>();
        let step = TimeDelta::try_seconds(1).unwrap();
        let c = Arc::new(SteppingClock::new(t0(), step));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let c = Arc::clone(&c);
                std::thread::spawn(move || (0..100).map(|_| c.now()).collect::<Vec<_>>())
            })
            .collect();
        let mut all: Vec<_> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();
        all.sort();
        assert_eq!(all.len(), 800);
        for (i, t) in all.iter().enumerate() {
            assert_eq!(*t, t0() + TimeDelta::try_seconds(i as i64).unwrap());
        }
        assert_eq!(c.now(), t0() + TimeDelta::try_seconds(800).unwrap());
    }

    #[test]
    fn stepping_saturates_at_the_end_of_time() {
        let c = SteppingClock::new(DateTime::<Utc>::MAX_UTC, TimeDelta::try_days(1).unwrap());
        assert_eq!(c.now(), DateTime::<Utc>::MAX_UTC);
        assert_eq!(c.now(), DateTime::<Utc>::MAX_UTC);
    }
}
