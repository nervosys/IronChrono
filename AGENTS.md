# AGENTS.md

Guidance for AI agents. Part 1 is for agents that use IronChrono. Part 2 is for agents that change it.

IronChrono is a maintained fork of Rust's chrono (drop-in replacement, API frozen at 0.4) with an
opt-in agent layer (`chrono::agent`, feature `agent`) and a CLI/MCP server, `ironchrono`.

## Part 1: using IronChrono

### Calling it

Two interfaces, same output.

CLI: one JSON object per call on stdout. Add `--text` for a single human-readable line.

```
$ ironchrono convert 2024-03-10T12:00:00Z --to +09:00 --text
2024-03-10T21:00:00+09:00
```

MCP: `ironchrono mcp` speaks the Model Context Protocol on stdio. Every CLI command is a tool.
Results come back as `structuredContent` with `isError` set on failure.

```json
{"mcpServers": {"ironchrono": {"command": "ironchrono", "args": ["mcp"]}}}
```

Commands: `now`, `parse`, `format`, `convert`, `add`, `sub`, `diff`, `round`, `trunc`, `specifiers`,
`errors`, `schema`, `mcp`. Discover rather than guess:

- `ironchrono <command> --help` lists the flags.
- `ironchrono schema` is the JSON Schema (draft 2020-12) of every envelope and output.
- `ironchrono specifiers` is the strftime table, with each specifier's output for a reference
  instant.
- `ironchrono errors` is the error vocabulary below.

### Envelopes and exit codes

Success: `{"ok":true,"command":...}`. Failure: `{"ok":false,"error":{"code","message","hint"}}`,
still on stdout. Branch on `error.code`, never on `message`. Read `hint` for the remedy.

| Exit | Meaning |
|------|---------|
| 0 | ok |
| 1 | internal error (a bug; report it) |
| 2 | usage error: unknown command or flag, missing or invalid argument |
| 3 | parse error: input, format, offset or duration could not be understood |
| 4 | out of range: the result is not representable, or rounding failed |
| 5 | local time is ambiguous or nonexistent (see `--policy`) |

### Error vocabulary

Generated from `ironchrono errors`. Codes are stable snake_case ids; they never change meaning.

| Code | Exit | Remedy |
|------|------|--------|
| `parse_out_of_range` | 3 | A field is outside its valid range (for example month 13 or hour 25); check each numeric field against the documented range of its specifier. |
| `parse_impossible` | 3 | The fields are individually valid but name no real date or contradict each other (for example 2023-02-30, or a weekday that does not match the date); correct the date or drop the redundant field. |
| `parse_not_enough` | 3 | The input lacks a field the target type needs (e.g. a time zone offset for DateTime<FixedOffset>); parse into a Naive type, or add %z to the format. |
| `parse_invalid` | 3 | The input does not match the format at some character; compare it to the specifier table (chrono::agent::SPECIFIERS) and check separators, padding and field order. |
| `parse_too_short` | 3 | The input ended before the format was satisfied; supply the missing trailing fields or shorten the format. |
| `parse_too_long` | 3 | The format matched but input remains; remove the trailing text, or extend the format to cover it. |
| `parse_bad_format` | 3 | The format string is malformed or uses an unsupported specifier or modifier; check it against chrono::agent::SPECIFIERS (the modifiers -, _ and 0 apply to numeric specifiers only). |
| `out_of_range` | 4 | The value is outside the range this type can represent; use a smaller magnitude or a wider type. |
| `rounding_duration_exceeds_timestamp` | 4 | The rounding duration is larger than the timestamp it applies to; use a smaller duration. |
| `rounding_duration_exceeds_limit` | 4 | The rounding duration must be positive and fit in 64-bit nanoseconds (about 292 years); use a shorter, positive duration. |
| `rounding_timestamp_exceeds_limit` | 4 | The timestamp does not fit in 64-bit nanoseconds (only years 1677 to 2262 can be rounded); round a value inside that range. |
| `invalid_month` | 3 | Use an English month name or its three-letter abbreviation (January or Jan), or construct Month from a number 1 to 12. |
| `invalid_weekday` | 3 | Use an English weekday name or its three-letter abbreviation (Monday or Mon). |
| `local_time_nonexistent` | 5 | That wall-clock time was skipped by a DST transition; choose a time outside the gap or work in UTC. |
| `local_time_ambiguous` | 5 | That wall-clock time occurs twice at a DST fold; pick LocalPolicy::Earliest or LocalPolicy::Latest, or supply an explicit offset. |
| `missing_offset` | 3 | A naive time cannot name an instant; supply an offset (Z or +HH:MM) or resolve it in a local zone under an explicit policy. |
| `unsupported_time_zone` | 3 | IANA time zone names are planned but not supported yet; use a fixed offset (+HH:MM), Z for UTC, or the local zone. |
| `usage` | 2 | Run `ironchrono <command> --help`. |
| `internal` | 1 | A bug in ironchrono; please report it. |

If this table and `ironchrono errors` disagree, the command is right.

### Rules: never guess

1. **Naive versus aware.** `2024-03-10T12:00:00` has no offset and names no instant. The CLI
   reports `"naive":true`, `unix_seconds:null`. Do not assume UTC. Pass `--offset +HH:MM`, `Z` or
   `local`, or ask for the offset. `"offset_assumed"` in a `parse` result says whether one was
   applied.
2. **DST.** A time skipped by a transition is an error (`local_time_nonexistent`, exit 5) under
   every policy. A time that occurs twice is an error (`local_time_ambiguous`) unless you pass
   `--policy earliest` or `--policy latest`. The default is `reject`. Choose a policy because the
   task says which instant is meant, not to make the error go away.
3. **Zones.** Only `+HH:MM`, `-HH:MM`, `Z` and `local` work. IANA names such as `Europe/Paris` fail
   with `unsupported_time_zone`. Do not substitute a fixed offset for one silently: a fixed offset
   is wrong across DST.
4. **Always pass `--now` in tests** (and `now` in MCP calls). Without it the CLI reads the system
   clock and results are not reproducible. In Rust, take `&dyn chrono::agent::Clock` and use
   `FixedClock` in tests.
5. **Months are not durations.** `1mo` clamps to month end (Jan 31 + 1mo = Feb 29 in 2024). Months
   apply before exact units. Use `d` or `h` when you mean a fixed span.
6. **Check formats before use.** `chrono::agent::check_format` (or the `parse_bad_format` error)
   names the bad specifier and its position. Look specifiers up in `ironchrono specifiers`; do not
   recall them from another language's strftime.
7. **Rounding** works only inside the years 1677 to 2262 (64-bit nanoseconds).
8. **Read `error.code`, not the message.** Messages are for people and may be reworded.

### Using the Rust library

```toml
chrono = { package = "ironchrono", version = "0.4", features = ["agent"] }
```

Not published to crates.io yet; use `git = "https://github.com/nervosys/IronChrono"` until it is.
Code is written as `use chrono::...`, exactly as for chrono. The agent layer is in `chrono::agent`:
`ErrorCode`, `Diagnose`, `Diagnosis`, `check_format`, `LocalPolicy`, `ResolveLocal`,
`LocalTimeError`, `Clock`, `SystemClock`, `FixedClock`, `SteppingClock`, `SPECIFIERS`,
`Specifier`, `REFERENCE_INSTANT_RFC3339`, `Facts`, `facts`, `naive_facts`. See README.md for
examples.

To move a whole dependency graph (sqlx and friends), patch `chrono` to `compat/chrono` and run
`cargo update -p chrono`. Scanners match on the package name, so prefer depending on `ironchrono`
directly in your own crates. README.md explains why.

## Part 2: developing IronChrono

### Layout

| Path | What |
|------|------|
| `src/` | The library. Package `ironchrono`, library name `chrono`, upstream layout. |
| `src/agent/` | The agent layer (`error`, `local`, `clock`, `facts`, `spec`, `spec_table`), behind feature `agent`. |
| `cli/` | The `ironchrono` binary (package `ironchrono-cli`): `commands`, `duration`, `mcp`, `output`, `schema`, `spec`. Tests in `cli/tests/cli.rs`. |
| `compat/chrono/` | The shim: a package named `chrono` that re-exports `ironchrono`, for `[patch.crates-io]`. Its own workspace. Surface test in `compat/chrono/tests/surface.rs`. |
| `compat/verify/` | An ecosystem graph (sqlx chrono feature, serde_json, tokio) with chrono patched to the shim. Test in `compat/verify/tests/roundtrip.rs`. |
| `tests/`, `bench/`, `fuzz/`, `ci/core-test/` | Upstream's integration tests and tooling. |
| `PLAN.md`, `NOTICE.md` | Principles and roadmap; provenance. |

### Compatibility rules

These come from the principles in PLAN.md. They are not negotiable.

1. Compatibility is the product. chrono 0.4's public API is frozen: do not remove, rename or make
   anything stricter. Additions are additive.
2. A behaviour change is allowed only to fix a bug. List it in CHANGELOG.md with the upstream issue
   it answers.
3. Agent features live in `chrono::agent` behind the `agent` feature, and in the CLI. Never change
   an existing type to serve them.
4. Keep the library name `chrono` and the upstream layout, so upstream fixes cherry-pick cleanly.
5. MIT OR Apache-2.0. Keep upstream history; NOTICE.md says what was forked and when.
6. Library MSRV stays 1.62. The CLI may use current Rust.
7. **Never add a dependency to the library.** The agent feature uses `serde`, which the library
   already optionally depends on. The CLI may add dependencies sparingly.
8. Do not edit `compat/chrono` features by hand without checking that every `ironchrono` feature
   (including implicit ones from optional dependencies) is forwarded under the same name.

### Running the suites

Set a short private target directory on Windows if paths get long, for example
`export CARGO_TARGET_DIR=C:/Users/<you>/.ichr`.

Upstream suite, with the CI feature set:

```
cargo test --features "default unstable-locales rkyv-32 rkyv-validation serde arbitrary"
```

Agent layer (unit tests and doctests):

```
cargo test --features agent
cargo test --doc --features agent
```

CLI:

```
cargo test -p ironchrono-cli
```

Shim surface test (every public chrono path reachable, same types as IronChrono). It is its own
workspace:

```
cargo test --manifest-path compat/chrono/Cargo.toml
```

Ecosystem proof (sqlx through the patch). Afterwards `cargo tree -i chrono` in that directory must
show only the shim:

```
cargo test --manifest-path compat/verify/Cargo.toml
cargo tree --manifest-path compat/verify/Cargo.toml -i chrono
```

MSRV: check the library on Linux with Rust 1.62 (CI does this; see the `rust_msrv` job in
`.github/workflows/test.yml`). Windows is not the place to verify it.

```
cargo +1.62.0 check --lib --features unstable-locales,wasmbind,oldtime,clock,winapi,serde
```

Add `agent` to that feature list when you change `src/agent`.

### Gotchas

- **CRLF.** On Windows, `tests/win_bindings.rs` rewrites the line endings of
  `src/offset/local/win_bindings.rs`. After a test run, `git checkout src/offset/local/win_bindings.rs`
  so the diff stays clean.
- Do not paste invented output into docs. Run the binary (with `--now`) and paste what it prints.
- Do not publish, tag or push without the owner's say-so.
