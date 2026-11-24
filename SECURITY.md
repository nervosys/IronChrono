# Security policy

## Reporting a vulnerability

Use GitHub private vulnerability reporting on
[nervosys/IronChrono](https://github.com/nervosys/IronChrono) (the Security tab, "Report a
vulnerability"). Please do not open a public issue for a suspected vulnerability.

Include the version or commit, the feature flags, a minimal reproduction, and what you expect to
happen.

## What to expect

- We acknowledge a report within 3 business days.
- For a confirmed issue, we ship a fix or a documented mitigation within 30 days.
- For a confirmed vulnerability, we file a RUSTSEC advisory so `cargo audit` and OSV users see it.
- We credit the reporter unless asked not to.

If we decide a report is not a vulnerability, we say why.

## Supported versions

Only the latest 0.4.x release is supported. Fixes are not backported to older 0.4.x releases.
IronChrono has not been published to crates.io yet; until it is, the supported version is the
current `master` of the repository.

## Upstream chrono advisories

IronChrono keeps chrono 0.4's API, so an advisory against chrono may also apply here.

- We watch the RustSec advisory database and the `chronotope/chrono` repository for new advisories
  and fixes, and check a report against our code the day we see it.
- If IronChrono is affected, we fix it, record the fix in CHANGELOG.md with the upstream
  reference, and file our own advisory against `ironchrono`.
- If it is not affected (for example the fault is in code IronChrono has already fixed), we say so
  in the changelog.
- Upstream fixes are cherry-picked while upstream still receives them. The fork point is
  `6adaa52` (see NOTICE.md).

RUSTSEC-2026-0543 (2026-09-14) marks every chrono version unmaintained. That is a maintenance
notice, not a vulnerability, and it is the reason this fork exists. Scanners match on the package
name, so a graph patched with the `compat/chrono` shim still contains a package named `chrono`
(from a path source). See the scanner caveat in [README.md](README.md). To be scanned as
IronChrono, depend on `ironchrono` directly.
