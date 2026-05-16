//! Generate, count, and sample chess back-rank arrangements under
//! composable constraints (Chess960, Chess2880, custom presets).
//!
//! The crate is generic — parameterised over a user-defined piece
//! kind, a user-defined colour set, and board size — and built around
//! a small set of composable constraint primitives with `And` / `Or` /
//! `Not` combinators. An opinionated [`chess`] module ships
//! ready-to-use presets for the canonical chess shuffle variants
//! (Chess960, Chess-2880, full shuffle).
//!
//! # Mental model
//!
//! - `pieces` is the **alphabet** — the *set* of distinct kinds
//!   available. Duplicate entries are silently deduplicated.
//! - `num_squares` is the board size.
//! - `square_colors` labels each square with a colour from the
//!   user-defined colour set. Generic; defaults to [`SquareColor`].
//! - Constraints filter the arrangements you want. The solver
//!   enumerates length-`num_squares` sequences over the alphabet
//!   and keeps the ones satisfying the root constraint.
//!
//! The constraint vocabulary is `Count`, `CountOnColor`, `At`,
//! `NotAt`, `Order`, and `Relative`, composable with `And` / `Or` /
//! `Not`. See [`Constraint`] for each variant's semantics.
//!
//! # Quick start — chess presets
//!
//! ```
//! use chess_startpos_rs::chess;
//!
//! assert_eq!(chess::shuffle().count(), 5040);
//! assert_eq!(chess::chess_2880().count(), 2880);
//! assert_eq!(chess::chess_960().count(), 960);
//! ```
//!
//! # Custom piece kinds, boards, and colours
//!
//! ```
//! use chess_startpos_rs::{alternating, Constraint, CountOp, Problem, SquareColor};
//!
//! #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
//! enum Tile { A, B }
//!
//! let problem: Problem<Tile> = Problem {
//!     num_squares: 3,
//!     square_colors: alternating(3, SquareColor::Light, SquareColor::Dark),
//!     pieces: vec![Tile::A, Tile::B],                // alphabet
//!     constraint: Constraint::And(vec![
//!         Constraint::Count { piece: Tile::A, op: CountOp::Eq, value: 2 },
//!         Constraint::Count { piece: Tile::B, op: CountOp::Eq, value: 1 },
//!         Constraint::At { piece: Tile::B, square: 1 },
//!     ]),
//! };
//! assert_eq!(problem.count(), 1);
//! assert_eq!(problem.at(0), Some(vec![Tile::A, Tile::B, Tile::A]));
//! ```

#![warn(missing_docs)]

use std::fmt::Debug;
use std::hash::Hash;

mod constraint;
mod problem;

pub mod chess;

pub use constraint::{Constraint, CountOp, SquareColor};
pub use problem::{Problem, ProblemBuilder, ValidationError};

/// Marker trait for piece kinds usable as the type parameter of
/// [`Problem`] / [`Constraint`].
///
/// Has a blanket impl: any type satisfying the supertrait bounds is
/// automatically a `PieceKind`. The user's piece enum typically
/// derives `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`,
/// `PartialOrd`, `Ord`, `Hash`.
pub trait PieceKind: Copy + Eq + Ord + Hash + Debug {}

impl<T: Copy + Eq + Ord + Hash + Debug> PieceKind for T {}

/// Marker trait for colour kinds usable as the type parameter of
/// [`Problem`] / [`Constraint`].
///
/// Has a blanket impl: any type satisfying the supertrait bounds is
/// automatically a `ColorKind`. The user's colour enum typically
/// derives `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`.
///
/// The default colour type for `Problem` is [`SquareColor`] — the
/// binary light/dark partition standard chess uses. Define your own
/// for N-way partitions (halves, thirds, fairy zones, …).
pub trait ColorKind: Copy + Eq + Hash + Debug {}

impl<T: Copy + Eq + Hash + Debug> ColorKind for T {}

/// Returns a `Vec<C>` of length `n` alternating between `first` and
/// `second`, starting with `first`.
///
/// ```
/// use chess_startpos_rs::{alternating, SquareColor};
///
/// let cs = alternating(4, SquareColor::Dark, SquareColor::Light);
/// assert_eq!(
///     cs,
///     vec![
///         SquareColor::Dark,
///         SquareColor::Light,
///         SquareColor::Dark,
///         SquareColor::Light,
///     ],
/// );
/// ```
#[must_use]
pub fn alternating<C: Copy>(n: usize, first: C, second: C) -> Vec<C> {
    (0..n)
        .map(|i| if i % 2 == 0 { first } else { second })
        .collect()
}

/// Returns a `Vec<C>` of length `n` where every entry is `c`.
///
/// ```
/// use chess_startpos_rs::{uniform, SquareColor};
///
/// assert_eq!(
///     uniform(3, SquareColor::Light),
///     vec![SquareColor::Light, SquareColor::Light, SquareColor::Light],
/// );
/// ```
#[must_use]
pub fn uniform<C: Copy>(n: usize, c: C) -> Vec<C> {
    vec![c; n]
}
