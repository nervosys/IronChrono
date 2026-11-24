# Notice

IronChrono is a fork of [chrono](https://github.com/chronotope/chrono), the
date and time library for Rust.

- Forked from `chronotope/chrono` at commit `6adaa52` (2026-08-02): release
  0.4.45 plus two unreleased fixes. The full upstream history is kept in this
  repository, so `git log` and `git blame` show who wrote each line.
- chrono is Copyright (c) 2014--2026, Kang Seonghoon and contributors, and is
  dual-licensed under the MIT License and the Apache License 2.0. Its licence
  text is reproduced unchanged in [LICENSE.txt](LICENSE.txt).
- IronChrono's own changes are Copyright (c) 2026 Nervosys and are released
  under the same terms: MIT OR Apache-2.0, at your option.

IronChrono is not affiliated with or endorsed by the chrono project or its
authors. The name `chrono` appears in this repository only where compatibility
requires it: the library name (so `use chrono::...` keeps working) and the
`compat/chrono` package (so `[patch.crates-io]` can replace chrono in a whole
dependency graph). Neither is published to crates.io under the name `chrono`.

Why the fork exists: chrono's maintainer announced in January 2026 that
maintenance would wind down
([chronotope/chrono#1768](https://github.com/chronotope/chrono/issues/1768)),
and RUSTSEC-2026-0543 (2026-09-14) marks every chrono version unmaintained.
IronChrono keeps the API and maintains it.
