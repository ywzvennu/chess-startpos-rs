# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-05-16

Initial public release.

### Added

#### Core API

- Generic constraint engine parameterised over a user-defined piece
  kind (`P: PieceKind`) and a user-defined colour kind
  (`C: ColorKind`, default [`SquareColor`]).
- `Constraint<P, C>` with primitives `Count`, `CountOnColor`, `At`,
  `NotAt`, `Order`, `Relative`, and combinators `And`, `Or`, `Not`.
  `#[non_exhaustive]` so new variants can land in 0.2 without a
  semver break. Empty-vec semantics: `And(vec![])` is true
  (always-satisfied), `Or(vec![])` is false.
- `Problem<P, C>` with the public surface:
  - `count()` — number of arrangements satisfying the constraint.
  - `iter()` — streams distinct arrangements in canonical
    lexicographic order. Returns `impl Iterator<Item = Vec<P>> + '_`
    rather than a leaked private type.
  - `at(index)` — deterministic indexed lookup.
  - `sample(seed)` — uniformly random arrangement via single-pass
    Knuth reservoir sampling. Returns `Option` (None for
    unsatisfiable problems).
  - `with_constraint(c)` — AND-narrowing, takes `&self`.
  - `validate()` — checks structural consistency, returns
    `Result<(), ValidationError>`.
- `ProblemBuilder<P, C>` — fluent construction. Methods:
  `squares(n)`, `colors(vec)`, `colors_fn(|i| …)`,
  `alternating_colors(a, b)`, `uniform_colors(c)`, `pieces(iter)`,
  `piece(p)`, `constraint(c)` (chainable, AND-composed at build),
  `build()`, `try_build()` (build + validate). Struct-literal
  construction of `Problem` remains a fully supported alternative.
- `ValidationError` enum with `ColorLengthMismatch`, `UnknownPiece`,
  `UnknownColor`, `SquareOutOfRange`, `InstanceOutOfRange`.
  `#[non_exhaustive]`, implements `Error` + `Display`.

#### Mental model

- `pieces` is the **alphabet** — a set of distinct kinds available.
  Duplicate entries are silently deduplicated (first-appearance
  order preserved). Per-piece counts come from `Count` constraints,
  not from repetition in the field.
- `square_colors` is generic: `Vec<C>` where `C: ColorKind` defaults
  to the binary [`SquareColor`] (Light / Dark). User-defined N-way
  colour partitions are supported. Empty `square_colors` is valid
  ("no colour partition declared"); colour-keyed constraints are
  rejected by `validate()` in that case.
- All square indices in the public API are 0-based.

#### Solver

- Fast path: when every alphabet member has a top-level
  `Constraint::Count { Eq, n }` and the values sum to `num_squares`,
  the enumerator iterates the implied multiset's distinct
  permutations via next-permutation.
- Fallback: enumerates length-`num_squares` Cartesian product
  sequences over the alphabet and filters via the full constraint
  tree.

#### Chess module

- `Piece` enum: King, Queen, Rook, Bishop, Knight. (No Pawn — pawns
  never appear on a back rank.) `Display` impl yields single-letter
  algebraic notation (K, Q, R, B, N).
- `STANDARD_BACK_RANK` const and `back_rank_colors()` helper for the
  default a1=Dark alternating layout.
- `file::A`..`file::H` constants and `file::of(char)` letter-to-index
  helper.
- Four named presets:
  - `standard()` — `count() == 1`.
  - `shuffle()` — `count() == 5040`.
  - `chess_2880()` — `count() == 2880` (bishops on opposite colours).
  - `chess_960()` — `count() == 960` (bishops opposite + king between
    rooks). Returns a `Chess960Problem` wrapper carrying the
    canonical Chess960 SP-ID bijection: `sp_id(id) -> Option<Vec<Piece>>`,
    `sp_id_of(&arr) -> Option<u32>`, `Chess960Problem::COUNT == 960`.
    SP-ID 518 is the standard FIDE starting position.
    `Chess960Problem::sample(seed)` is infallible (preset is
    statically non-empty).

#### Helpers

- Crate-level `alternating(n, first, second)` and `uniform(n, c)`
  for assembling `square_colors`.

#### Optional features

- `serde` feature derives `Serialize` / `Deserialize` on
  `Constraint`, `CountOp`, `SquareColor`, `Problem`,
  `ValidationError`, and `chess::Piece`.

#### Examples + tests + benches

- `examples/quickstart.rs` walks the four chess presets.
- `examples/custom.rs` demonstrates a custom piece kind and a
  three-zone user-defined colour partition.
- `tests/custom.rs` exercises the generic API on a non-chess board.
- `benches/presets.rs` (criterion) covers preset `count`, `sample`,
  SP-ID forward/reverse, and a 4⁶ Cartesian-fallback bench.

#### Hygiene

- MIT licence.
- README with badges, install + quickstart, constraint vocabulary
  table, custom-piece + N-way-colour example, builder snippet,
  validation snippet, solver notes.
- `CONTRIBUTING.md` covering scope, dev environment, PR guidelines.
- GitHub issue + PR templates and dependabot config.
- CI workflow with three jobs: fmt + clippy + test + doc on stable
  (`-Dwarnings`), library check on MSRV 1.80, `cargo audit`.
- `[package.metadata.docs.rs] all-features = true` so docs.rs
  builds the `serde` feature.
- MSRV 1.80.
