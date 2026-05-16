# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — Initial release

### Added

- Generic constraint engine over a user-defined piece kind.
- `Constraint<P>` enum with five primitives — `Count`, `CountOnColor`,
  `At`, `NotAt`, `Order` — and three combinators — `And`, `Or`, `Not`.
- `Problem<P>` struct with the public surface:
  - `count()` — number of arrangements satisfying the constraint.
  - `iter()` — streams distinct arrangements in canonical lexicographic order.
  - `at(index)` — deterministic indexed lookup.
  - `sample(seed)` — uniformly random arrangement, deterministic in the seed.
  - `with_constraint(c)` — builder for AND-narrowing.
- Hand-rolled solver using the standard next-permutation algorithm over
  the sorted piece multiset.
- `chess` module with `Piece` enum, `STANDARD_BACK_RANK` const,
  `back_rank_board()` helper, and four named presets:
  - `standard()` — count `1`.
  - `shuffle()` — count `5040`.
  - `chess_2880()` — count `2880` (bishops on opposite-colour squares).
  - `chess960()` — count `960` (bishops opposite + king between rooks).
- `examples/quickstart.rs` showing the four presets, indexed lookup,
  random sampling, and constraint narrowing.
- MIT licence, README, contributor guide, GitHub issue/PR templates,
  CI workflow (fmt + clippy + test + doc on stable, library check on
  MSRV 1.80), dependabot configuration.
