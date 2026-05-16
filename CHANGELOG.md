# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Round-5 final polish

- Documented the empty-vec semantics on `Constraint::And` and
  `Constraint::Or`: `And(vec![])` is vacuously **true** (the natural
  always-true constraint); `Or(vec![])` is vacuously **false**.
  Matches Rust's `iter().all() / iter().any()` and standard
  identity-element semantics.
- `Problem::dedup_alphabet` and `Problem::validate` use a `HashSet<P>`
  for membership checks instead of `Vec::contains`. O(n) instead of
  O(n²) — invisible for chess (5 kinds) but matters for larger
  alphabets.
- New unit test exercising all six `CountOp` variants
  (`Eq` / `NotEq` / `Lt` / `Le` / `Gt` / `Ge`) end-to-end against the
  same alphabet, asserting the expected populations.

### Round-4 polish

- `Cargo.toml` keywords: dropped weak `shuffle` and `starting-position`;
  added `csp` and `combinatorics` for better crates.io discoverability.
- Added `[package.metadata.docs.rs] all-features = true` so docs.rs
  builds with the `serde` feature enabled.
- Removed the redundant `where C: Clone` bound on
  `Problem::with_constraint`; `ColorKind: Copy` already implies it.
- Documented 0-based square indexing in README + chess module.
- README quickstart now mentions `Problem::at` / `sample` are
  `Option`-returning for unsatisfiable problems.
- New doctests: `try_build` with a `ColorLengthMismatch` error,
  `Constraint::Relative` shape, `chess::back_rank_colors()` layout.

### Validation + ergonomics + serde

- New `Problem::validate() -> Result<(), ValidationError>` checks
  `square_colors` length, that every constraint reference uses
  declared pieces / colours / squares, and that empty
  `square_colors` is only paired with constraints that don't
  reference colours. `count` / `iter` / `sample` don't auto-validate.
- New `ProblemBuilder::try_build()` — `build()` followed by
  `validate()`.
- New `ProblemBuilder::colors_fn(|i| …)` — assigns colours via a
  per-index closure, expanded eagerly at builder time.
- New `ValidationError` enum (re-exported at the crate root) with
  variants `ColorLengthMismatch`, `UnknownPiece`, `UnknownColor`,
  `SquareOutOfRange`. `#[non_exhaustive]`; implements `Error` +
  `Display`.
- `Display` impl on `chess::Piece` — single-letter algebraic
  (`K`, `Q`, `R`, `B`, `N`).
- New optional `serde` feature gating `Serialize` / `Deserialize`
  on `Constraint`, `CountOp`, `SquareColor`, `Problem`,
  `ValidationError`, and `chess::Piece`.
- Doc notes: `Constraint::Relative::offset` is `i32` (boards
  larger than `i32::MAX` not supported); `square_colors` may be
  empty (treated as "no colour partition declared").

### Builder API

- New `Problem::builder()` returns a `ProblemBuilder<P, C>` for
  fluent construction. Methods: `squares(n)`, `colors(vec)`,
  `alternating_colors(a, b)`, `uniform_colors(c)`, `pieces(iter)`,
  `piece(p)`, `constraint(c)` (chainable, AND-composed),
  `build()`. The struct-literal API stays a fully supported
  alternative.

### Round-2 pre-tag cleanup

- `Problem::with_constraint` and `Chess960Problem::with_constraint`
  take `&self` instead of `self`, so callers no longer need to
  pre-clone the problem to keep using it.
- `Problem::sample` now does a single-pass reservoir sample
  (Knuth-style) instead of iterating twice (once for `count`, once
  for `nth`). Same uniform distribution, half the work.
- `next_cartesian` in the unconstrained-regime hot path caches the
  alphabet's position lookup in a `HashMap` instead of doing
  `O(|alphabet|)` `.position()` each advance.
- `chess::back_rank_colors()` and crate helpers `alternating` /
  `uniform` were already added in the earlier reshape; this round
  drops the "multiset declaration" framing from `Constraint::Count`
  and the crate-level docs — `Count` is just one constraint among
  many, and the `Count{Eq}` optimisation is an internal solver
  detail.
- `Cargo.toml`: added `homepage`.
- README: constraint primitives table now includes `Relative`.
- CI: new `cargo-audit` job runs on every push.
- New `benches/presets.rs` (criterion) covers the four chess
  presets' `count`, `sample`, SP-ID forward/reverse, and a 4⁶
  Cartesian-fallback bench.
- New tests: non-Eq `Count` as a filter, `Count{Eq}` inside `Or` is
  not picked up by the fast path, empty alphabet and `num_squares == 0`
  yield zero arrangements.

### Pre-publish API reshape

The crate has not yet been published to crates.io. The [0.1.0] entry
below describes the final shape that will ship.

- `Problem<P, C = SquareColor>` and `Constraint<P, C = SquareColor>`
  gain a generic colour parameter. Define your own colour enum for
  N-way partitions (halves, thirds, fairy zones); chess users keep
  the binary [`SquareColor`] default and write no extra type
  parameters.
- `pieces` is now the **alphabet** — a set of distinct kinds —
  rather than a flat multiset with repetition. Duplicate entries
  are silently deduplicated. Multiset multiplicities come from
  `Constraint::Count { piece, Eq, value }` entries.
- The solver derives the multiset from `Count{Eq}` constraints in
  the root / top-level `And`. Falls back to enumerating Cartesian
  product sequences from the alphabet when some kinds are
  unconstrained.
- `#[non_exhaustive]` on `Constraint`, `CountOp`, and `SquareColor`.
- `Chess960Problem` derives `Clone, Debug`.
- `Chess960Problem::sample(seed) -> Vec<Piece>` is infallible (the
  preset is statically non-empty). Generic `Problem::sample` keeps
  returning `Option`.
- `Problem::iter()` and `Chess960Problem::iter()` return
  `impl Iterator<Item = Vec<P>> + '_` rather than the previously
  unnameable `ProblemIter`.
- `Piece::Pawn` removed — pawns never appear on the back rank.
- `chess::back_rank_colors()` and crate-level `alternating(n, a, b)`
  / `uniform(n, c)` helpers for assembling `square_colors`.

## [0.1.0] — Initial release

### Added

- Generic constraint engine parameterised over a user-defined piece
  kind (`P: PieceKind`) and a user-defined colour kind
  (`C: ColorKind`, default [`SquareColor`]).
- `Constraint<P, C>` with primitives `Count`, `CountOnColor`, `At`,
  `NotAt`, `Order`, `Relative`, and combinators `And`, `Or`, `Not`.
- `Problem<P, C>` with the public surface:
  - `count()` — number of arrangements satisfying the constraint.
  - `iter()` — streams distinct arrangements in canonical
    lexicographic order.
  - `at(index)` — deterministic indexed lookup.
  - `sample(seed)` — uniformly random arrangement, deterministic
    in the seed (returns `Option` for unsatisfiable problems).
  - `with_constraint(c)` — AND-narrowing builder.
- `pieces` is the alphabet (set of distinct kinds), not a multiset
  with repetition. Multiset multiplicities come from
  `Constraint::Count{Eq}` entries.
- Solver:
  - Fast path: when all alphabet members have a Count-Eq fixing the
    multiset, runs the standard next-permutation algorithm.
  - Fallback: enumerates length-N Cartesian product sequences from
    the alphabet and filters via the constraint tree.
- `chess` module with:
  - `Piece` enum (King, Queen, Rook, Bishop, Knight).
  - `STANDARD_BACK_RANK` const and `back_rank_colors()` helper.
  - `file::A`..`file::H` constants and `file::of('a')` for letter
    → index conversion.
  - Four named presets:
    - `standard()` — count `1`.
    - `shuffle()` — count `5040`.
    - `chess_2880()` — count `2880` (bishops on opposite colours).
    - `chess_960()` — count `960` (bishops opposite + king between
      rooks). Returns `Chess960Problem` carrying the canonical
      Chess960 SP-ID bijection (`sp_id(id)`, `sp_id_of(&arr)`,
      `Chess960Problem::COUNT == 960`).
- Crate-level `alternating(n, first, second)` and `uniform(n, c)`
  helpers for assembling `square_colors`.
- `#[non_exhaustive]` on `Constraint`, `CountOp`, and `SquareColor`.
- Examples: `examples/quickstart.rs` (chess presets),
  `examples/custom.rs` (custom piece + colour kinds with a
  three-zone partition).
- Integration tests under `tests/custom.rs` covering the generic
  API on a non-chess board.
- MIT licence, README, contributor guide, GitHub issue/PR templates,
  CI workflow (fmt + clippy + test + doc on stable, library check on
  MSRV 1.80), dependabot configuration.
