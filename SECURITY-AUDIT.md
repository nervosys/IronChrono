# Security audit: IronChrono 0.4.46

Date: 2026-11-24. Base: commit `e3224f1` (package `ironchrono` 0.4.46, library `chrono`, the
`ironchrono` CLI and MCP server). Fixes are in the commit that adds this file.

Frameworks: CVE / RustSec / OSV (known vulnerabilities and new findings, scored with CVSS 3.1 and
CWE), MITRE ATT&CK (Enterprise; techniques an adversary could use through or against IronChrono,
and the mitigations), NIST FIPS 140-3 and FIPS 199/200, and CMMC 2.0 Level 2 (NIST SP 800-171 r2).

CMMC and SP 800-171 assess an organization's systems, not a library. The CMMC section says which
practices IronChrono helps a system meet, and which stay with the system that uses it.

## Summary

| Area | Result |
|---|---|
| Known vulnerabilities in IronChrono's code | None. Both chrono advisories checked: RUSTSEC-2020-0159 is fixed in this code; RUSTSEC-2026-0543 is chrono's "unmaintained" notice, which IronChrono answers. |
| Known vulnerabilities in dependencies | Default features: none (2 dependencies). One in the opt-in `rkyv` features: RUSTSEC-2026-0235 in rkyv 0.7, which has no fixed 0.7 release (DEP-1, mitigated: use `rkyv-08`). |
| New findings | 17: 1 High, 6 Medium, 6 Low, 4 Info. 14 fixed with regression tests; 1 mitigated (DEP-1: rkyv 0.7 has no fixed release, use `rkyv-08`); 2 accepted with reasons (LIB-2, CLI-10). Upstream's documented panics are listed as LIB-5. |
| Memory safety | 13 `unsafe` sites, all reviewed sound (8 in production code, OS calls and two checked invariants). The CLI forbids `unsafe`. |
| Fuzzing | 6 targets, about 64 million executions under AddressSanitizer with debug assertions: 0 crashes. The kept targets were run again after the fixes: 0 crashes. |
| Cryptography (FIPS 140-3) | None in the library or the CLI. Nothing to validate; IronChrono cannot weaken a FIPS configuration. |

## Method

- **Library** (`src/`): read every `unsafe` block, every file read, and the agent layer; checked
  both chrono advisories against the code; probed the panic surface with `catch_unwind`; fuzzed on
  Linux (WSL Debian, `cargo +nightly fuzz`, ASan, `-Cdebug-assertions`).
- **CLI and MCP server** (`cli/`): threat model is an AI agent passing untrusted strings (from web
  pages, mail, other tools) to `ironchrono`, and an MCP client that is buggy or compromised.
  Hostile probes over argv and stdio, with peak memory measured.
- **Supply chain**: `cargo audit` 0.23 and `cargo deny` 0.19 over the full lockfile (102
  packages); an OSV batch query of all 100 registry packages; dependency trees per feature set.

## Findings

Scores are CVSS 3.1 base scores for the library or tool in its own context. A consuming system
re-scores for its deployment.

| ID | Title | Severity | CWE | Status |
|---|---|---|---|---|
| CLI-1 | Non-ASCII offset panics the CLI; one MCP call ends the server | High 7.5 `AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H` | 248, 135 | Fixed |
| CLI-3 | An MCP line that is not UTF-8 ends the server silently | Medium | 20, 755 | Fixed |
| CLI-4 | MCP lines, ids and arrays unbounded: 1 GB line → 1.6 GB, 40 MB array → 2.5 GB | Medium | 400, 770 | Fixed |
| CLI-5 | Output amplification (`%+` ×28) and full input echoed in errors | Medium | 405 | Fixed |
| LIB-1 | The file `TZ` names is read without limit: `/dev/zero` → 2.9 GB, a FIFO hangs `Local::now()` | Medium 4.7 `AV:L/AC:H/PR:L/UI:N/S:U/C:N/I:N/A:H` | 770 | Fixed |
| LIB-6 | A forged rkyv 0.7 archive of a `NaiveDate`, `NaiveTime`, `TimeDelta`, `FixedOffset` or `IsoWeek` passed validation with broken invariants, so later calls panicked or returned wrong values | Medium 4.8 `AV:N/AC:H/PR:N/UI:N/S:U/C:N/I:L/A:L` | 20, 1287 | Fixed |
| LIB-7 | On Unix `Local.from_local_datetime` returned every fold as (later, earlier) instead of the documented (earliest, latest), so `earliest()`, `latest()` and `LocalPolicy::Earliest`/`Latest` picked the wrong instant; times a fraction of a second past a fold or gap boundary were also misclassified | Medium 4.1 `AV:N/AC:H/PR:N/UI:N/S:U/C:N/I:L/A:N` | 682 | Fixed |
| CLI-2 | A non-UTF-8 argument panics the CLI | Low 3.3 `AV:L/AC:L/PR:N/UI:R/S:U/C:N/I:N/A:L` | 248, 176 | Fixed |
| CLI-6 | `--text` mode writes control and bidi characters raw to the terminal | Low 4.3 `AV:N/AC:L/PR:N/UI:R/S:U/C:N/I:L/A:N` | 150, 117 | Fixed |
| CLI-7 | `--text` / `--help` recognised anywhere, even after `--` or as a flag's value | Low 4.3 `AV:N/AC:L/PR:N/UI:R/S:U/C:N/I:L/A:N` | 88 | Fixed |
| LIB-3 | `check_format` accepts `%#z`, which then fails when formatting | Low | 703 | Fixed (new `check_output_format`) |
| LIB-4 | One diagnosis message echoed a raw control character | Low | 117, 150 | Fixed |
| DEP-1 | rkyv 0.7 (opt-in `rkyv-*` features): RUSTSEC-2026-0235, out-of-bounds read via forged `Rc`/`Arc` metadata | Low here (see below) | 125 | Mitigated: use `rkyv-08` |
| CLI-8 | `--help` panics on a closed pipe | Info | 248 | Fixed |
| CLI-9 | `--now` accepted and ignored by most commands; a hint named a Rust type; MCP accepted any `jsonrpc` | Info | 20 | Fixed |
| LIB-2 | A relative `TZ` may contain `..` and leave the zoneinfo directories | Info | 22 | Accepted |
| CLI-10 | Each MCP request is answered: 100k `tools/list` → 667 MB to the client | Info | 400 | Accepted |
| LIB-5 | Upstream's documented panics (operators, deprecated constructors, `to_rfc2822` outside 0..=9999, `format().to_string()` with a failing format) | Info | 248 | Documented |

### High

**CLI-1.** `parse_zone` split an offset with `split_at(2)` on a byte length, so `--to '+a€'`
panicked (exit 101, no JSON). Through MCP, one `tools/call` ended the server, and the client saw a
dead pipe. The panic text named the toolchain path. Fix: a non-ASCII offset is a `parse_invalid`
error (exit 3). Every command runs inside `catch_unwind`, a panic becomes the documented `internal`
error (exit 1, or MCP `isError: true`), and the panic hook prints one generic line. Tests:
`non_ascii_offset_is_a_parse_error_not_a_panic`, `mcp_survives_a_non_ascii_offset`,
`guard_maps_a_panic_to_internal`.

### Medium

**CLI-3.** `serve` returned on the first line that was not UTF-8. It now answers JSON-RPC -32700
and reads on. Test: `mcp_invalid_utf8_line_is_answered_and_the_server_lives`.

**CLI-4.** Fixed with:
- a 1 MiB line cap (`read_line_bounded`): the rest of an oversized line is discarded, then -32600;
- `id` limited to a string of at most 256 characters, an integer or null;
- `params` moved rather than cloned.

Peak memory before → after: 1 GB line 1,617 MB → 7 MB; 20M-element array 2,460 MB → 7 MB; 50 MB
`id` 216 MB → 7 MB. Tests: `mcp_oversized_line_is_refused_and_the_stream_resyncs`,
`mcp_restricts_ids_and_jsonrpc_version`, `bounded_reader_caps_and_resyncs`, `ids_are_restricted`.

**CLI-5.** Fixed with:
- arguments capped at 4,096 bytes (a `usage` error);
- formatted output capped at 65,536 bytes (`out_of_range`);
- echoed input cut to 200 characters.

A 50 MB `%+` request went from a 1.4 GB response (3 GB peak) to a 99-byte refusal. Tests:
`arguments_over_4096_bytes_are_a_usage_error`, `echoed_input_in_errors_is_cut_to_200_chars`,
`output_over_the_limit_is_out_of_range`, `clip_cuts_long_input`.

**LIB-1.** On Unix, `TZ=/path` and `TZ=:path` read the named file with `read_to_end`, with no size
or type check. `TZ=/dev/zero` grew to 2.86 GB in 4 s; `TZ=/tmp/fifo` blocked `Local::now()`
indefinitely. Exploiting it needs control of the process environment or of the file a `TZ` path
names. Fix: `read_tz_file` (`src/offset/local/tz_info/timezone.rs`) reads only a regular file, of
at most 1 MiB (real TZif files are a few kilobytes). It checks before opening, because opening a
FIFO blocks, and checks again on the handle. `/etc/localtime` goes through the same function.
After the fix, each of these falls back to the system zone in about 10 ms: `/dev/zero`,
`:/dev/zero`, a FIFO, and a 2 GB sparse file. Test: `tz_file_must_be_a_small_regular_file`.

- **LIB-6:** with `rkyv-validation` (rkyv 0.7, bytecheck 0.6) the derived `CheckBytes` checked
  bytes only. A `NaiveDate` archive with ordinal 511 passed `from_bytes` and `month()` then
  panicked; a `NaiveTime` with 90000 seconds passed and `hour()` returned 25; likewise a
  `TimeDelta` with 10^9 or more nanoseconds, a `FixedOffset` beyond one day and an `IsoWeek`
  that does not exist. It is memory safe (a zero `NaiveDate` was already rejected by the
  `NonZeroI32` check), but an application that reads archives from an untrusted source could be
  made to panic (A:L) or compute a wrong time (I:L); the attacker needs the application to
  accept such archives and use the value (AC:H), hence 4.8. Fix: the five archived types have a
  hand-written `CheckBytes` that checks the fields, then the invariant, with the same predicates
  that rkyv 0.8's `Verify` impls use (`NaiveDate::yof_is_valid` and so on), so the two cannot
  drift. Names, layouts and public items are unchanged; `NaiveDateTime` and `DateTime` inherit the
  checks through their fields. Tests: `rkyv07_forged_*` in `src/rkyv07_tests.rs`, nine of which
  fail on the previous code. The derived `Deserialize` is unchanged: rkyv 0.7 leaves unchecked
  access of untrusted bytes to the caller, and a generic deserializer error cannot be built.

- **LIB-7:** found by `fuzz_local` on Linux (its `assert!(a <= b)` on every `Ambiguous`). In a fold
  the larger offset is the earlier instant, but the tz_info lookup returned the pair ordered by
  offset ascending (TZif path) or `(std, dst)` (TZ-string path), so for every fold `a > b`:
  Los Angeles 2023-11-05 01:30 gave `(-08:00, -07:00)`; likewise Berlin, Sydney, Dublin (negative
  DST) and the 1883 LMT change. Code that takes `earliest()` (for example to schedule at the first
  occurrence) got the later instant, one hour wrong, once or twice a year per zone. A second
  defect floored the local time to whole seconds, so times just past a boundary were classified
  as on it. Impact is the integrity of a time stamp (I:L), with no confidentiality or availability
  effect; it needs an application that resolves attacker-influenced local times in a fold (AC:H),
  hence 4.1. Fix: the pair is always (larger, smaller) offset, boundaries compare seconds and
  fraction exactly (`WallTime`), and the result is the same as the Windows path. chrono 0.4.45 has
  both defects. Tests: `fold_and_gap_posix_tz_strings`, `fold_and_gap_constructed_transitions`,
  `fold_and_gap_tzif_zones` and the property test `local_lookup_is_consistent_with_utc_lookup`
  (every offset returned converts back to itself, for fractions around every transition), all of
  which fail on the previous code.

### Low and Info

- **CLI-2:** `args_os()` replaces `args()`; a non-UTF-8 argument is a usage error (exit 2).
- **CLI-6:** `render_text` escapes C0, DEL and C1 controls and the bidi controls (U+061C, U+200E/F,
  U+202A–202E, U+2066–2069) as `\u{..}`. JSON output already escaped them.
- **CLI-7:** global flags are parsed with the rest: they count only before `--` and never as a
  flag's value.
- **CLI-8:** help is written with `write!` and the error ignored.
- **CLI-9:** `--now` exists only on `now`; the ambiguity hint uses CLI words (`--policy earliest`);
  MCP requires `"jsonrpc": "2.0"`. The CLI has `#![forbid(unsafe_code)]`.
- **LIB-3:** `check_format` checks syntax, which is all parsing needs. The new
  `check_output_format` also rejects parse-only specifiers (`%#z`). Formatting any `DateTime` with
  a format it accepts cannot fail. A test checks it against every specifier in `SPECIFIERS`, and
  the `fuzz_agent` target asserts it.
- **LIB-4:** the named specifier is printed with `escape_debug`. Test: `bad_specifier_is_escaped`.
- **DEP-1:** the advisory is in rkyv's own validation of shared pointers. chrono's archived types
  contain no `Rc` or `Arc`, so archives of chrono types alone are not affected. An application
  that validates untrusted archives mixing chrono types with `Rc`/`Arc` is affected through rkyv
  itself. rkyv 0.7 has no fixed release, and 0.8 is a different API: moving chrono's `rkyv-*`
  features to it would break every user of them, which the compatibility promise rules out.
  Remedy (done): the additive `rkyv-08` feature uses rkyv >= 0.8.17, which is fixed. It can be
  enabled next to the 0.7 features, so applications migrate at their own pace; the 0.7 features
  are unchanged. Don't validate untrusted archives with rkyv 0.7. The 0.8 integration also checks
  chrono's own invariants, as the 0.7 one now does too (LIB-6). Tests: `rkyv08_forged_*` in
  `src/rkyv08_tests.rs`. `cargo deny` fails on this advisory, and it is deliberately not added to
  `deny.toml`'s ignore list.
- **LIB-2:** this matches glibc for programs that are not setuid. Changing it would break existing
  `TZ` values. Only the zone rules are taken from the file: a non-TZif target (tested with
  `/etc/passwd`) is rejected, and nothing from it leaks.
- **CLI-10:** the output is proportional to what the client sends over its own stdio pipe, so it
  harms no one but the sender.
- **LIB-5:** these panics are upstream's documented 0.4 behaviour and cannot change under the
  compatibility promise. The `checked_*` and `try_*` forms, and `check_output_format`, are the
  safe paths for untrusted input. AGENTS.md says so.

## `unsafe` inventory

| Location | What | Verdict |
|---|---|---|
| `src/date.rs:468` | `unsafe impl Send for Date<Tz>` bounded on the offset type | Sound |
| `src/naive/date/mod.rs:1520` | `NonZeroI32::new_unchecked` in `from_yof` | Sound; invariant held over ~45M fuzz runs with debug assertions |
| `src/offset/local/tz_data.rs:160` | `CStr::from_bytes_with_nul_unchecked` up to the first NUL | Sound (Android/OHOS only) |
| `src/offset/local/tz_info/timezone.rs` (`TimeZoneName::as_str`) | `from_utf8_unchecked` | Sound; the only constructor admits `[0-9A-Za-z+_-]` |
| `src/offset/local/windows.rs:136` | `GetTimeZoneInformationForYear`, then `assume_init` | Sound; return checked, year clamped; exercised by `fuzz_local` under ASan on Windows |
| `src/offset/local/windows.rs:263–277` | two Win32 calls, return ignored, then `assume_init` | Test-only code |
| `src/offset/local/win_bindings.rs:23, 57` | `mem::zeroed` of all-integer `repr(C)` structs | Sound |

## Known vulnerabilities (CVE / RustSec / OSV)

| Advisory | Alias | Affected | IronChrono |
|---|---|---|---|
| RUSTSEC-2020-0159 (`localtime_r` segfault, `setenv` race) | related CVE-2020-26235 | chrono < 0.4.20 | Not affected. No `localtime_r`, `mktime`, `setenv` or `libc::` calls remain; the TZ database is parsed in Rust (`tz_info/parser.rs`). |
| RUSTSEC-2026-0543 (chrono unmaintained) | none | every `chrono` version, by name | Answered by this fork. `cargo audit` does not report `ironchrono`. The `compat/chrono` shim carries the name `chrono` and still matches SBOM scans that ignore a package's source (README, "Advisory scanners"). |
| RUSTSEC-2026-0235 (rkyv) | none | rkyv 0.7.0-pre.2 to before 0.8.17 | DEP-1 |
| RUSTSEC-2025-0141 (bincode unmaintained) | none | dev-dependency only | Not shipped; already ignored upstream in `deny.toml` |

The OSV query for `ironchrono` returns nothing. Default features depend on `num-traits` and, on
Windows, `windows-link`. `cargo deny`: bans, licences and sources pass; advisories fail on DEP-1
only.

## Fuzzing

All runs on Linux, `cargo +nightly fuzz`, ASan, debug assertions, `max_len` 8192.

| Target | Covers | Runs | Crashes |
|---|---|---|---|
| `fuzz_reader` (upstream) | RFC 2822 / 3339 parsing | 17,055,594 | 0 |
| `fuzz_format` (upstream) | `parse_from_str` with a fuzzed format | 11,420,336 | 0 |
| `fuzz_tzif` (audit harness, not kept) | TZif parser plus lookups at extreme instants | 10,485,313 | 0 |
| `fuzz_tzstr` (audit harness, not kept) | POSIX TZ strings plus lookups | 20,645,287 | 0 |
| `fuzz_agent` (kept) | `check_format`, `facts`, `ResolveLocal`, serde, formatting | 1,982,562 | 0 |
| `fuzz_parse` (kept) | every `FromStr`, `parse_from_str`, serde, constructors, `checked_*` | 2,590,858 | 0 |

The two TZ harnesses need a `cfg(fuzzing)` hook into private parsers, so they were not added to
the tree. Re-run after the fixes, with `fuzz_agent` now asserting that every format `check_output_format`
accepts also formats: `fuzz_agent` 2,125,744 runs in 301 s and `fuzz_parse` 627,074 runs in
121 s, with 0 crashes.

After the LIB-7 fix, `fuzz_local` (Linux, ASan) replayed its crash artifact in five zones with no failure
and ran with `TZ` set to America/Los_Angeles (317,481 runs, 301 s), Europe/Berlin (104,011),
Europe/Dublin (113,462), Australia/Sydney (137,786) and Pacific/Apia (527,432), 120 s each: 0
crashes, `assert!(a <= b)` holds.

## MITRE ATT&CK

IronChrono is a library and a local tool, so it is an avenue for an adversary rather than a target
of its own. These are the techniques it bears on.

| Technique | How it applies | Status |
|---|---|---|
| T1499.004 Endpoint Denial of Service: Application or System Exploitation | A crafted string (CLI-1, CLI-3), an oversized request (CLI-4, CLI-5) or a hostile `TZ` (LIB-1) stops or exhausts the process that parses time. In an agent, the string can arrive by prompt injection. | Mitigated: no panic reachable from input, bounded reads and responses, fuzzed. |
| T1195.001 Supply Chain Compromise: Compromise Software Dependencies and Development Tools | A time library sits in most dependency graphs, and the patch mechanism swaps it for every crate. | Two default dependencies; `cargo deny` checks sources and licences; `Cargo.lock` reviewed; the shim is `publish = false`. Releases are not yet signed (see gaps). |
| T1036 Masquerading | A third party could publish a package claiming to be IronChrono or "chrono-maintained". | The canonical source is `github.com/nervosys/IronChrono`; NOTICE.md and the README name it. Publishing reserves `ironchrono` on crates.io. |
| T1070.006 Indicator Removal: Timestomp, T1562 Impair Defenses | A program whose time can be steered writes misleading log times. | `Utc` ignores `TZ` (tested); `Utc::now()` is the system clock and does not drift on its own. Clock integrity is the host's job (NTP). `FixedClock` and `SteppingClock` exist for tests: production code should take `SystemClock`. |
| T1059 Command and Scripting Interpreter (via an agent's tool use) | An agent passes injected text to `ironchrono`; terminal escapes (CLI-6) or a flag inside the data (CLI-7) could alter what an operator sees or how the call is read. | Mitigated: `--text` output escaped; flags count only before `--`; JSON on stdout by default. |

Mitigations in place: M1013 Application Developer Guidance (AGENTS.md, SECURITY.md, this file),
M1016 Vulnerability Scanning (`cargo audit`, `cargo deny`, OSV), M1051 Update Software (SECURITY.md
response targets), M1048 Application Isolation and Sandboxing (no network access; the only files
read are time zone files; MCP over stdio only). Not yet in place: M1045 Code Signing.

## NIST FIPS

- **FIPS 140-3 (cryptographic modules).** Out of scope, because IronChrono performs no
  cryptography. With default features it depends on `num-traits` and `windows-link` alone. The
  opt-in `rkyv-*` features bring non-cryptographic hashes (`ahash`, `seahash`) and `getrandom`,
  which seeds hash maps against flooding. None of these protects data, so none needs a validated
  module. A system running in FIPS mode is unaffected by IronChrono, and IronChrono cannot be the
  reason it falls out of FIPS mode.
- **FIPS 199 / FIPS 200.** These categorize information systems and set their minimum
  requirements (through SP 800-53). A system that uses IronChrono categorizes itself. The time
  stamp control below is where IronChrono contributes.
- **SP 800-53 AU-8 (Time Stamps).** `Utc::now()` returns UTC from the system clock, `to_rfc3339()`
  writes an unambiguous time stamp, and local-time ambiguity is never resolved silently
  (`ResolveLocal`). An authoritative time source (AU-8(1)) is the host's responsibility.

## CMMC 2.0 Level 2 (NIST SP 800-171 r2)

Applicable practices for a component of an assessed system:

| Practice | What IronChrono provides | Status for the component |
|---|---|---|
| AU.L2-3.3.7 Authoritative time source / time stamps | UTC time stamps from the system clock, `Utc` not affected by `TZ`, ambiguous local times rejected unless a policy is named | Supports; host clock sync stays with the system |
| AU.L2-3.3.1 / 3.3.2 Audit records | Correct, parseable time stamps (RFC 3339) for audit records | Supports |
| CM.L2-3.4.1 / 3.4.2 Baseline configuration | Pinned versions in `Cargo.lock`; MSRV 1.62 checked; features documented | Met for the component |
| CM.L2-3.4.6 / 3.4.7 Least functionality | Default features minimal; agent layer and CLI are opt-in; no network code | Met for the component |
| RA.L2-3.11.2 Vulnerability scanning | `cargo audit`, `cargo deny` and OSV run in this audit | Partly: run by hand; not yet in CI (Phase 5) |
| RA.L2-3.11.3 Remediate vulnerabilities | 13 findings fixed with regression tests; DEP-1 mitigated by `rkyv-08` | Met for the component; rkyv 0.7 stays in the lockfile for the legacy features |
| SI.L1-3.14.1 Flaw remediation | SECURITY.md: acknowledgement in 3 business days, fix or mitigation in 30 days, RUSTSEC advisory for confirmed issues | Met for the component |
| SI.L2-3.14.6 / 3.14.7 Monitor for attacks / unauthorized use | Refusals are structured (`code`, `message`, `hint`), so a caller can log them | Supports |
| SC.L2-3.13.2 Security engineering principles | Compatibility, input caps, no `unsafe` in the CLI, fuzzing, explicit DST policy | Met for the component |
| AC.L2-3.1.1 / IA.L2-3.5.x Access control and identification | The MCP server runs as the user who started it, over stdio, with no network listener, and adds no access of its own | N/A to the component; inherits the host's |

## Gaps and next steps

1. **CI** (Phase 5). Workflows written and `actionlint`-clean: `test.yml` (upstream matrix plus
   MSRV with `agent`, the agent, CLI and compat jobs), `security.yml` (`cargo audit`, `cargo deny`,
   weekly), `fuzz.yml` (nightly, five targets) and `release.yml`. They have not run yet: the
   organization's Actions are on a billing hold. Until they run, the scans are manual.
2. **DEP-1.** Mitigated: the additive `rkyv-08` feature (rkyv 0.8.18) exists, and the 0.7 path
   now validates invariants too (LIB-6). `cargo audit` and `cargo deny` still report
   RUSTSEC-2026-0235, because the lockfile keeps rkyv 0.7 for the legacy features.
3. **Signed releases** (M1045). `release.yml` publishes SHA256SUMS, GitHub artifact attestations
   (Sigstore, keyless) and a cosign signature, and publishes to crates.io by trusted publishing
   behind the `crates-io` environment. The owner still has to set up that environment (required
   reviewers) and the trusted-publisher entry on crates.io.
4. **Windows `Local` path.** Done for one zone. `fuzz_local` ran natively on Windows with
   AddressSanitizer for 900 s (243,000 runs); `tests/windows_local.rs` checks 4.4 million local
   times from 1600 to 2100 (1,503 folds, 501 gaps) and the extremes. No fault found. Both ran in
   the machine's zone (Pacific) only: the system zone was not changed to cover others.
5. **Run the audit again** after each upstream cherry-pick that touches `src/offset/local` or
   `src/format`.
