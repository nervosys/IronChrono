# IronChrono plan

IronChrono is the maintained, agentic-first successor to chrono and a drop-in
replacement for it. "Drop-in" is a promise with a test behind it: code written
against chrono 0.4 compiles and behaves the same against IronChrono, and a
whole dependency graph (sqlx and friends included) can be moved with one
`[patch.crates-io]` line.

## Principles

1. **Compatibility is the product.** chrono 0.4's public API is frozen here:
   nothing is removed, renamed or made stricter. Additions are additive.
   Behaviour changes only to fix a bug, each one listed in the CHANGELOG with
   the upstream issue it answers.
2. **Agentic-first, additively.** Everything an agent needs (stable error
   codes with remediation hints, explicit local-time resolution, injectable
   clocks, self-describing tables, JSON and MCP access) lives in
   `chrono::agent` behind the `agent` feature and in the `ironchrono` CLI,
   never as a change to existing types.
3. **Upstream stays mergeable.** The library keeps the name `chrono` and the
   upstream layout, so an upstream fix cherry-picks cleanly.
4. **Same licence, honest provenance.** MIT OR Apache-2.0, upstream history
   kept, NOTICE.md says what was forked and when.
5. **MSRV stays 1.62** for the library (the CLI may use current Rust).

## Phase 1 — Foundation

- [x] Fork with history at 6adaa52; package `ironchrono` 0.4.46, library
      `chrono`; `compat/chrono` shim (package `chrono`, own workspace).
- [x] Upstream suite green on current Rust: three doctests used
      `core::i32::MAX`, now rejected as deprecated.
- [x] NOTICE.md, this plan.
- [x] README, AGENTS.md, llms.txt, CHANGELOG, SECURITY.md (maintenance
      policy: who answers an advisory, and how fast), CITATION.cff.

## Phase 2 — Agent layer (`chrono::agent`, feature `agent`)

- [x] `ErrorCode` (stable snake_case ids, hint per code), `Diagnose` for every
      error type chrono returns. 17 codes, incl. `missing_offset` and
      `unsupported_time_zone`; `check_format` for strftime strings.
- [x] `ResolveLocal` with `LocalPolicy` (reject, earliest, latest): a DST fold
      is never resolved silently, and a gap is always an error.
- [x] `Clock` with `SystemClock`, `FixedClock`, `SteppingClock`.
- [x] `SPECIFIERS`: every strftime specifier with meaning and a rendered
      example, self-verified against the formatter and checked against the
      upstream strftime doc table so it cannot drift.
- [x] `facts()`: everything an agent asks about an instant, Serializable;
      a naive time reports no instant (offset and Unix fields are null).
- [x] MSRV check of the layer on 1.62: `cargo +1.62.0 check --lib --target
      x86_64-unknown-linux-gnu` with `agent` and upstream's MSRV feature set,
      after replacing serde derives (syn needs 1.71) with hand-written impls,
      as upstream does; `serde_snapshot` proves the JSON is unchanged.

## Phase 3 — `ironchrono` CLI and MCP server

- [x] JSON envelopes, stable exit codes, `--now` for determinism.
- [x] now, parse, format, convert, add, sub, diff, round, trunc, specifiers,
      errors, schema (24 tests, schema checked against real output).
- [x] `ironchrono mcp`: stdio MCP server exposing the same commands as tools
      (11 tools, protocol 2025-06-18).

## Phase 4 — Drop-in proof

- [x] Surface test: every public path of chrono 0.4.45 reachable through the
      shim, and the shim's types are IronChrono's types (167 paths from
      rustdoc JSON; `compat/chrono/tests/surface.rs`).
- [x] Ecosystem test: sqlx (chrono feature) + serde_json with the patch;
      `cargo tree -i chrono` shows only the shim (`compat/verify`, 3 tests).
- [x] IronMarketingIntelligence's full suite on IronChrono via the patch,
      against its unpatched baseline: 2,135 of 2,135, same as chrono 0.4.43
      (2026-11-24; needs `cargo update -p chrono`, since lockfiles pin the old
      version).
- [ ] Advisory scanners: what an OSV / cargo-audit scan reports for a patched
      graph, and what IronChrono must do to be scanned as itself.
      Found: an SBOM-based OSV scan that ignores the source still queries the
      shim as `chrono` and matches RUSTSEC-2026-0543 (all versions). Direct
      users should name `ironchrono`; the shim needs the scanner to skip path
      packages, or an allowance naming it.

## Security audit (2026-11-24)

- [x] CVE/RustSec/OSV, MITRE ATT&CK, NIST FIPS, CMMC 2.0 Level 2: SECURITY-AUDIT.md.
      17 findings, 14 fixed with regression tests (1 High, 6 Medium fixed); LIB-6 closed the
      forged rkyv 0.7 archives, LIB-7 the reversed fold order (and sub-second boundaries) of the
      Unix `Local` lookup.
- [x] DEP-1: an additive `rkyv-08` feature (rkyv 0.7 has RUSTSEC-2026-0235 and no fix).
- [x] Windows-native fuzzing of the `Local` FFI path (`fuzz_local` under ASan,
      `tests/windows_local.rs` sweep; Pacific zone only).

## Phase 5 — Release

- [x] CI written (GitHub Actions, upstream's matrix plus the MSRV job, the
      proofs, `cargo audit` / `cargo deny`, nightly fuzzing, signed releases);
      actionlint-clean.
- [ ] CI running: the organization's Actions are on a billing hold (owner).
- [ ] Publish `ironchrono` and `ironchrono-cli` to crates.io (owner).
- [ ] Track upstream: a weekly check of chronotope/chrono for commits to
      cherry-pick, while it still receives any.

## Later

- IANA time zones (a maintained tz database: decide between a successor to
  chrono-tz and a jiff-tz bridge).
- `ironchrono` interop with `jiff` and `time` (From/Into behind features).
