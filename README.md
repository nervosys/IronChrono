# IronChrono

The maintained, agentic-first successor to Rust's [chrono](https://github.com/chronotope/chrono),
and a drop-in replacement for it. By Nervosys. MIT OR Apache-2.0.

IronChrono is not published to crates.io yet. Until it is, use the git or path forms below.
Where this file shows a crates.io form, it is what will work once published.

## What and why

chrono is the date and time library most Rust code depends on. Its maintainer announced in January
2026 that maintenance would wind down
([chronotope/chrono#1768](https://github.com/chronotope/chrono/issues/1768)), and RUSTSEC-2026-0543
(2026-09-14) marks every chrono version unmaintained. IronChrono is a fork with full history
(from `chronotope/chrono` at `6adaa52`: 0.4.45 plus two unreleased fixes) that keeps the 0.4 API
and keeps fixing it.

"Drop-in" is tested, not claimed. The `compat/chrono` shim re-exports all 167 public paths of
chrono (the list is derived from rustdoc JSON and checked by a test). sqlx's `chrono` feature builds
through the shim. The full suite of IronMarketingIntelligence (2,135 tests) passes on IronChrono
through the patch, as it does on chrono 0.4.43. On top of the frozen API, IronChrono adds an opt-in
agent layer and a CLI, so programs that cannot eyeball a failure get stable error codes, explicit
DST handling, injectable clocks and JSON.

## Migrating in one line

There are three ways. Pick the first unless you cannot.

**(a) Rename the package, change no code (recommended).** The library inside `ironchrono` is named
`chrono`, so `use chrono::...` keeps working.

```toml
chrono = { package = "ironchrono", version = "0.4" }
```

**(b) Depend on `ironchrono` directly.** Same result; the dependency key is `ironchrono`, the
crate name in code is still `chrono`.

```toml
ironchrono = "0.4"
```

Until it is on crates.io, point either form at the repository:

```toml
chrono = { package = "ironchrono", git = "https://github.com/nervosys/IronChrono" }
```

**(c) Patch a whole graph, for transitive dependents such as sqlx.** The `compat/chrono` package is
named `chrono`, so Cargo accepts it as a patch and every crate in the graph builds against
IronChrono. With a local clone:

```toml
[patch.crates-io]
chrono = { path = "../IronChrono/compat/chrono" }
```

The shim lives in a subdirectory of the repository, so a git patch must resolve to it. Use the
`path` form unless you have checked that your Cargo version finds `compat/chrono` in the git
checkout.

```toml
[patch.crates-io]
chrono = { git = "https://github.com/nervosys/IronChrono" }
```

After either, run `cargo update -p chrono`. A lockfile pins the old chrono version and keeps using
it until you do. We learned this the hard way.

### Scanner caveat (read before using the patch)

Advisory scanners key on the package name. A graph patched with the shim still contains a package
named `chrono`, from a path source. `cargo audit` skips non-registry sources and will not report it.
SBOM-based OSV scans that ignore the source will still match RUSTSEC-2026-0543.

So for your own crates, use (a) or (b) and depend on `ironchrono` directly. Use the patch only to
move transitive dependents, and tell your scanner that the `chrono` package in your graph is a path
package, not the registry crate.

## The compatibility promise

- chrono 0.4's public API is frozen: nothing is removed, renamed or made stricter.
- Additions are additive. New things live in new items, behind the `agent` feature, or in the CLI.
- Behaviour changes only to fix a bug. Each fix is listed in [CHANGELOG.md](CHANGELOG.md) with the
  upstream issue it answers.
- The library keeps the name `chrono` and upstream's layout, so upstream fixes cherry-pick cleanly.

Fixed so far: three doctests used the deprecated `core::i32::MAX`; they now use `i32::MAX`.

## The agent layer

Enable the `agent` feature to get `chrono::agent`. It changes no existing type. It wraps what is
there so that failures are machine-readable and nothing is guessed. Everything in it implements
`serde::Serialize`.

```toml
chrono = { package = "ironchrono", version = "0.4", features = ["agent"] }
```

Stable error codes, with a remediation hint for each:

```rust
use chrono::agent::{check_format, Diagnose, ErrorCode};
use chrono::{DateTime, FixedOffset};

let err = DateTime::<FixedOffset>::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap_err();
assert_eq!(err.code(), ErrorCode::ParseNotEnough);
assert_eq!(err.code().as_str(), "parse_not_enough");
assert!(err.hint().contains("%z"));

// Check a format string without using it.
let d = check_format("%Y-%Q").unwrap_err();
assert_eq!(d.code, ErrorCode::ParseBadFormat);
```

No silent DST guesses. A gap is always an error. A fold is an error unless you name `Earliest` or
`Latest`:

```rust
use chrono::agent::{LocalPolicy, LocalTimeError, ResolveLocal};
use chrono::{MappedLocalTime, NaiveDate, TimeDelta};

let a = NaiveDate::from_ymd_opt(2024, 11, 3).unwrap().and_hms_opt(1, 30, 0).unwrap();
let b = a + TimeDelta::try_hours(1).unwrap();
let fold = MappedLocalTime::Ambiguous(a, b);
assert_eq!(fold.resolve(LocalPolicy::Earliest), Ok(a));
assert_eq!(
    fold.resolve(LocalPolicy::Reject),
    Err(LocalTimeError::Ambiguous { earliest: a, latest: b })
);
let gap = MappedLocalTime::<i32>::None;
assert_eq!(gap.resolve(LocalPolicy::Latest), Err(LocalTimeError::Nonexistent));
```

Injectable clocks. Take `&dyn Clock` instead of calling `Utc::now()`, and tests become deterministic:

```rust
use chrono::agent::{Clock, FixedClock};
use chrono::{DateTime, TimeDelta, TimeZone, Utc};

fn expires_at(clock: &dyn Clock) -> DateTime<Utc> {
    clock.now() + TimeDelta::try_hours(24).unwrap()
}

let clock = FixedClock(Utc.with_ymd_and_hms(2024, 3, 10, 12, 0, 0).unwrap());
assert_eq!(expires_at(&clock), Utc.with_ymd_and_hms(2024, 3, 11, 12, 0, 0).unwrap());
```

`SteppingClock` returns a start instant and then advances by a fixed step per reading.
`SystemClock` is the real one.

Flat facts about an instant, and a self-describing table of every strftime specifier:

```rust
use chrono::agent::{facts, SPECIFIERS};
use chrono::{FixedOffset, TimeZone};

let dt = FixedOffset::east_opt(9 * 3600).unwrap().with_ymd_and_hms(2024, 3, 10, 21, 0, 0).unwrap();
let f = facts(&dt);
assert_eq!((f.iso_week, f.weekday, f.is_leap_year), (10, "Sunday", true));
assert_eq!(f.offset.as_deref(), Some("+09:00"));

// Each entry has the spec, its meaning, and its output for a reference instant.
let j = SPECIFIERS.iter().find(|s| s.spec == "%j").unwrap();
assert_eq!(j.example, "189");
```

`naive_facts` does the same for a `NaiveDateTime`. The examples above were compiled and run against
this repository.

## The `ironchrono` CLI

`ironchrono` (crate `ironchrono-cli`, in `cli/`) exposes the same layer on the command line.
It prints exactly one JSON object per call, uses stable exit codes, and never reads the system
clock except in `now`, which takes `--now` to fix it. Add `--text` for a human-readable line.
Each argument may be at most 4096 bytes, output at most 65536 bytes, and an MCP request line at most 1 MiB.
Build it with:

```
cargo build -p ironchrono-cli
```

Commands: `now`, `parse`, `format`, `convert`, `add`, `sub`, `diff`, `round`, `trunc`, `specifiers`,
`errors`, `schema`, `mcp`. Run `ironchrono <command> --help` for flags. Output below is real.

```
$ ironchrono parse 2024-03-10T12:00:00+01:00
{"ok":true,"command":"parse","naive":false,"instant":{"rfc3339":"2024-03-10T12:00:00+01:00","unix_seconds":1710068400,"unix_nanos":1710068400000000000,"year":2024,"month":3,"month_name":"March","day":10,"ordinal":70,"iso_year":2024,"iso_week":10,"weekday":"Sunday","weekday_from_monday":6,"hour":12,"minute":0,"second":0,"nanosecond":0,"offset_seconds":3600,"offset":"+01:00","is_leap_year":true,"days_in_month":31,"leap_second":false,"naive":false},"matched":"rfc3339","offset_assumed":false}

$ ironchrono convert 2024-03-10T12:00:00Z --to +09:00 --text
2024-03-10T21:00:00+09:00

$ ironchrono add 2024-01-31T00:00:00Z 1mo --text
2024-02-29T00:00:00+00:00

$ ironchrono diff 2024-01-01T00:00:00Z 2024-03-10T12:00:00Z
{"ok":true,"command":"diff","seconds":6004800,"nanos":0,"total_nanos":6004800000000000,"negative":false,"iso8601":"P69DT12H","human":"69 days, 12 hours"}

$ ironchrono now --now 2024-03-10T12:00:00Z --text
2024-03-10T12:00:00+00:00

$ ironchrono parse 2024-13-45
{"ok":false,"error":{"code":"parse_invalid","message":"\"2024-13-45\" matches none of the supported formats (rfc3339, rfc2822, unix_seconds (@N), iso_offset_space, iso_offset_minutes, iso_naive_t, iso_naive_space, iso_naive_minutes_t, iso_naive_minutes_space, date, date_slash)","hint":"Use RFC 3339 (2024-03-01T12:00:00Z), or pass --format <strftime>."}}
```

The last command exits 3. Failures print a JSON error envelope on stdout; the exit code says which
class.

| Exit | Meaning |
|------|---------|
| 0 | ok |
| 1 | internal error |
| 2 | usage error: unknown command or flag, missing or invalid argument |
| 3 | parse error: input, format, offset or duration could not be understood |
| 4 | out of range: the result is not representable, or rounding failed |
| 5 | local time is ambiguous or nonexistent (see `--policy`) |

Zones are `+HH:MM`, `Z` or `local`. IANA names are not supported yet. Durations are ISO 8601
(`P1DT2H30M`) or compact (`90s`, `1h30m`, `1mo`); months clamp to month end.
`ironchrono schema` prints the JSON Schema of every output, and `ironchrono errors` lists every
error code with its hint and exit code.

### MCP

`ironchrono mcp` serves the same commands as tools over the Model Context Protocol on stdio. Add it
to any MCP client's configuration:

```json
{"mcpServers": {"ironchrono": {"command": "ironchrono", "args": ["mcp"]}}}
```

Each tool returns the same JSON as the CLI, as `structuredContent`, with `isError` set on failure.

## Crate features

Default features:

* `alloc`: features that depend on allocation (primarily string formatting).
* `std`: functionality that depends on the standard library. A superset of `alloc`.
* `clock`: reading the local timezone (`Local`). A superset of `now`.
* `now`: reading the system time (`now`).
* `wasmbind`: the JS Date API for the `wasm32` target.
* `oldtime`: no effect; kept for compatibility.

| Feature | What it does |
|---------|--------------|
| `agent` | IronChrono's agent layer, `chrono::agent` (enables `std` and `serde`). New in IronChrono. |
| `serde` | Serialization via [serde](https://github.com/serde-rs/serde). |
| `rkyv`, `rkyv-16`, `rkyv-32`, `rkyv-64` | Serialization via [rkyv](https://github.com/rkyv/rkyv) with that integer width. `rkyv` is deprecated; the four are mutually exclusive. |
| `rkyv-validation` | rkyv validation through `bytecheck`. |
| `arbitrary` | Arbitrary instances of each type. |
| `unstable-locales` | Localization: methods with a `_localized` suffix. May change in a patch release. |
| `defmt` | `defmt` formatting. |
| `libc`, `winapi`, `core-error` | Platform and `core::error` support, as upstream. |

## Limitations

These are chrono's, unchanged.

* Only the proleptic Gregorian calendar is supported.
* Date types are limited to about +/- 262,000 years from the common epoch.
* Time types are limited to nanosecond accuracy.
* Leap seconds can be represented, but are not fully supported. See
  [Leap Second Handling](https://docs.rs/chrono/latest/chrono/naive/struct.NaiveTime.html#leap-second-handling).
* Timezone data is not shipped. `Local` follows the OS; fixed offsets and UTC are built in.
  IANA zone support is planned, see [PLAN.md](PLAN.md).

## Rust version

MSRV 1.62 for the library. CI checks it on Linux. The CLI may use current Rust.

## Upstream chrono

IronChrono is a fork of [chrono](https://github.com/chronotope/chrono), Copyright (c) 2014-2026
Kang Seonghoon and contributors. The upstream history is kept in this repository. It is not
affiliated with or endorsed by the chrono project. See [NOTICE.md](NOTICE.md) for what was forked
and when, and [PLAN.md](PLAN.md) for the roadmap. Upstream documentation at
[docs.rs/chrono](https://docs.rs/chrono) describes the same API.

## Documentation

* API reference: <https://docs.rs/ironchrono> (once published).
* For AI agents: [AGENTS.md](AGENTS.md) and [llms.txt](llms.txt).
* Security: [SECURITY.md](SECURITY.md).

## Licence

Licensed under either of [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
or [MIT License](https://opensource.org/licenses/MIT), at your option. See
[LICENSE.txt](LICENSE.txt).
