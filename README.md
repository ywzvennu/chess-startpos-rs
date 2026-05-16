# chess-startpos-rs

[![crates.io](https://img.shields.io/crates/v/chess-startpos-rs.svg)](https://crates.io/crates/chess-startpos-rs)
[![docs.rs](https://img.shields.io/docsrs/chess-startpos-rs)](https://docs.rs/chess-startpos-rs)
[![CI](https://github.com/ywzvennu/chess-startpos-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ywzvennu/chess-startpos-rs/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.80-blue.svg)](Cargo.toml)

Generate, count, and sample chess back-rank arrangements under composable
constraints (Chess960, Chess2880, custom presets).

The crate provides a generic constraint engine. You describe a problem by
saying *which pieces, on how many squares, with which colours, satisfying
which constraints*; the crate enumerates, counts, indexes, or samples the
satisfying arrangements deterministically. An opinionated `chess` module
ships ready-to-use presets for the canonical shuffle variants.

## Install

```toml
[dependencies]
chess-startpos-rs = "0.1"
```

Minimum supported Rust version: **1.80**.

## Quick start

Chess users — the four named presets:

```rust
use chess_startpos_rs::chess;

assert_eq!(chess::standard().count(),    1);     // FIDE starting back rank
assert_eq!(chess::shuffle().count(),     5040);  // any permutation
assert_eq!(chess::chess_2880().count(),  2880);  // bishops opposite colours
assert_eq!(chess::chess_960().count(),    960);   // bishops opposite + king between rooks

// Deterministic indexed lookup, in canonical lexicographic order.
let pos = chess::chess_960().at(0).unwrap();

// Uniform random sampling, deterministic in the seed.
let pos = chess::chess_960().sample(42).unwrap();

// Narrow any preset with extra constraints. File letters
// (`chess::file::A..chess::file::H`) and `chess::file::of('a')` resolve
// to back-rank square indices.
use chess_startpos_rs::Constraint;
let with_queen_on_d1 = chess::chess_960().with_constraint(Constraint::At {
    piece: chess::Piece::Queen,
    square: chess::file::D,
});
assert!(with_queen_on_d1.count() < chess::chess_960().count());
```

For a longer worked example, see [`examples/quickstart.rs`](examples/quickstart.rs)
or run `cargo run --example quickstart`.

## Constraint primitives

Bring your own piece kind (any `Copy + Eq + Ord + Hash + Debug` type) and
your own board. The constraint vocabulary:

| Primitive | Meaning |
|---|---|
| `Count { piece, op, value }` | Number of `piece` on the board satisfies `op value`. |
| `CountOnColor { piece, color, op, value }` | Number of `piece` on squares of `color` satisfies `op value`. |
| `At { piece, square }` | `piece` must occupy `square`. |
| `NotAt { piece, square }` | `piece` must not occupy `square`. |
| `Order(vec)` | The indexed instances listed must appear in strictly increasing square order. `[(R, 0), (K, 0), (R, 1)]` reads as `rook[0] < king[0] < rook[1]`. |

And three combinators:

| Combinator | Meaning |
|---|---|
| `And(children)` | All children must hold. |
| `Or(children)` | At least one child must hold. |
| `Not(inner)` | The inner constraint must not hold. |

`op` is one of `Eq`, `NotEq`, `Le`, `Lt`, `Ge`, `Gt`.

## Solver

For v0.1 the solver is hand-rolled: it iterates distinct multiset
permutations via the standard next-permutation algorithm and filters by
the constraint. For chess back-rank problems (up to 5040 candidates) this
is microseconds, zero extra dependencies. A general-purpose CSP backend
(behind a feature flag) is tracked in [#7](https://github.com/ywzvennu/chess-startpos-rs/issues/7)
for whenever a larger problem size makes it worth the extra surface.

## Status

Initial development. Public API will stabilise at v0.1.0.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps --all-features
```

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the contribution workflow,
and [`CHANGELOG.md`](CHANGELOG.md) for release history.

## License

[MIT](LICENSE).
