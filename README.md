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

### Optional features

| Feature | Effect |
|---|---|
| `serde` | Derives `Serialize` / `Deserialize` on `Constraint`, `CountOp`, `SquareColor`, `Problem`, `ValidationError`, and `chess::Piece`. |

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

// Look up by canonical Chess960 SP-ID (0..=959). SP-ID 518 is the
// standard FIDE starting position. Round-trips with `sp_id_of`.
let preset = chess::chess_960();
let standard = preset.sp_id(518).unwrap();
assert_eq!(preset.sp_id_of(&standard), Some(518));

// Uniform random sampling, deterministic in the seed.
let pos = chess::chess_960().sample(42);

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

All square indices in the public API are **0-based**. Chess users:
square 0 is `a1`, square 7 is `h1`. The `chess::file::A`..`chess::file::H`
constants and `chess::file::of('a')` resolve to those indices.

`Problem::at(idx)` and `Problem::sample(seed)` return `Option<Vec<P>>`
on the generic type — `None` when the constraint set is unsatisfiable
(`count() == 0`) or when `idx >= count()`. The chess presets are
statically non-empty, so `Chess960Problem::sample` returns
`Vec<Piece>` directly.

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
| `Relative { lhs, rhs, op, offset }` | Signed positional offset between two specific piece instances: `(lhs_square − rhs_square) op offset`. `lhs = (King, 0)`, `rhs = (Queen, 0)`, `op = Eq`, `offset = 2` reads as "king is exactly 2 squares right of queen". |

And three combinators:

| Combinator | Meaning |
|---|---|
| `And(children)` | All children must hold. |
| `Or(children)` | At least one child must hold. |
| `Not(inner)` | The inner constraint must not hold. |

`op` is one of `Eq`, `NotEq`, `Le`, `Lt`, `Ge`, `Gt`.

## Custom piece kinds, boards, and colours

The `chess` module is a convenience layer; the core
(`Constraint<P, C>` / `Problem<P, C>`) is generic over **both** the
piece kind and the colour kind. `pieces` is the *alphabet* — a set of
distinct kinds. Constraints filter the arrangements; how many of each
kind you want is just another constraint
(`Constraint::Count { piece, Eq, value }`).

```rust
use chess_startpos_rs::{Constraint, CountOp, Problem, SquareColor};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Card { Ace, King, Queen }

let problem: Problem<Card> = Problem {
    num_squares: 6,
    square_colors: vec![
        SquareColor::Light, SquareColor::Light, SquareColor::Light,
        SquareColor::Dark,  SquareColor::Dark,  SquareColor::Dark,
    ],
    pieces: vec![Card::Ace, Card::King, Card::Queen],   // alphabet
    constraint: Constraint::And(vec![
        // Pin exactly 2 of each kind (6 squares total).
        Constraint::Count { piece: Card::Ace,   op: CountOp::Eq, value: 2 },
        Constraint::Count { piece: Card::King,  op: CountOp::Eq, value: 2 },
        Constraint::Count { piece: Card::Queen, op: CountOp::Eq, value: 2 },
        // Aces split across the two halves.
        Constraint::CountOnColor {
            piece: Card::Ace, color: SquareColor::Light,
            op: CountOp::Eq, value: 1,
        },
        Constraint::CountOnColor {
            piece: Card::Ace, color: SquareColor::Dark,
            op: CountOp::Eq, value: 1,
        },
        // First King precedes first Queen.
        Constraint::Order(vec![(Card::King, 0), (Card::Queen, 0)]),
    ]),
};

assert_eq!(problem.count(), 27);
```

For N-way colour partitions (halves, thirds, fairy zones …) define
your own colour enum and pass it as the `C` type parameter:

```rust
use chess_startpos_rs::{Constraint, CountOp, Problem};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
enum Zone { Red, Green, Blue }

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Bead { White, Black }

let problem: Problem<Bead, Zone> = Problem {
    num_squares: 6,
    square_colors: vec![Zone::Red, Zone::Red, Zone::Green, Zone::Green, Zone::Blue, Zone::Blue],
    pieces: vec![Bead::White, Bead::Black],
    constraint: Constraint::And(vec![
        Constraint::Count { piece: Bead::White, op: CountOp::Eq, value: 3 },
        Constraint::Count { piece: Bead::Black, op: CountOp::Eq, value: 3 },
        Constraint::CountOnColor {
            piece: Bead::White, color: Zone::Red,
            op: CountOp::Eq, value: 1,
        },
        // ...
    ]),
};
```

The solver enumerates length-`num_squares` sequences from the alphabet
and filters by the constraint tree. When every alphabet member has a
`Constraint::Count { Eq, n }` and the values sum to `num_squares`, it
takes a fast path that walks distinct piece arrangements directly
instead of the full Cartesian product.

For a longer worked example, see [`examples/custom.rs`](examples/custom.rs)
or run `cargo run --example custom`.

### Validation

`Problem::validate()` returns `Ok(())` if the problem is internally
consistent — colour vector length matches `num_squares` (or is empty),
every constraint references declared pieces, colours, and squares —
or a `ValidationError` otherwise. Useful before solving large
problems. The builder has a matching `try_build()` that runs
`validate()` and returns the error if it fails:

```rust
use chess_startpos_rs::{chess, Constraint, CountOp, Problem, SquareColor, ValidationError};

let result: Result<Problem<chess::Piece>, _> = Problem::builder()
    .squares(8)
    .colors(vec![SquareColor::Light]) // mismatched: 1 ≠ 8
    .pieces([chess::Piece::King])
    .try_build();
assert!(matches!(result, Err(ValidationError::ColorLengthMismatch { .. })));
```

`count()` / `iter()` / `sample()` do not auto-validate; call
`validate()` (or use `try_build`) up front if you want errors over
silent zero-result enumeration.

### Builder alternative

`Problem` has both struct-literal and fluent-builder construction
paths; pick whichever reads better. The builder accumulates
constraints and AND-composes them at `build()` time:

```rust
use chess_startpos_rs::{Constraint, CountOp, Problem, SquareColor};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Card { Ace, King, Queen }

let problem: Problem<Card> = Problem::builder()
    .squares(6)
    .alternating_colors(SquareColor::Light, SquareColor::Dark)
    .pieces([Card::Ace, Card::King, Card::Queen])
    .constraint(Constraint::Count { piece: Card::Ace,   op: CountOp::Eq, value: 2 })
    .constraint(Constraint::Count { piece: Card::King,  op: CountOp::Eq, value: 2 })
    .constraint(Constraint::Count { piece: Card::Queen, op: CountOp::Eq, value: 2 })
    .build();

assert_eq!(problem.count(), 90);
```

## Solver

For v0.1 the solver is hand-rolled: it enumerates piece arrangements
over the declared alphabet via the standard next-permutation algorithm
(when piece counts are fully fixed via `Count{Eq}` constraints) or via
the Cartesian product over the alphabet (otherwise), then filters by
the constraint tree. For chess back-rank problems (up to 5040
candidates) this is microseconds, zero extra dependencies. A
general-purpose CSP backend (behind a feature flag) is tracked in
[#7](https://github.com/ywzvennu/chess-startpos-rs/issues/7) for
whenever a larger problem size makes it worth the extra surface.

## Status

v0.1.0 released. The public API is stable within the 0.x series;
breaking changes require a minor version bump.

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
