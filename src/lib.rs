//! Generate, count, and sample chess back-rank arrangements under
//! composable constraints (Chess960, Chess2880, custom presets).
//!
//! The crate is generic — parameterised over a user-defined piece
//! kind, generic over board size — and built around a small set of
//! composable constraint primitives with `And` / `Or` / `Not`
//! combinators. An opinionated [`chess`] module ships ready-to-use
//! presets for the canonical chess shuffle variants (Chess960,
//! Chess-2880, full shuffle).
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
//! # Custom piece kinds and boards
//!
//! The chess module is a convenience layer; the core
//! ([`Constraint<P>`] / [`Problem<P>`]) is generic over your own piece
//! kind and board size.
//!
//! ```
//! use chess_startpos_rs::{Constraint, Problem, SquareColor};
//!
//! #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
//! enum Tile { A, B }
//!
//! let problem = Problem {
//!     num_squares: 3,
//!     square_colors: vec![SquareColor::Light; 3],
//!     pieces: vec![Tile::A, Tile::A, Tile::B],
//!     constraint: Constraint::At { piece: Tile::B, square: 1 },
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
pub use problem::Problem;

/// Marker trait for piece kinds usable as the type parameter of
/// [`Problem`] / [`Constraint`].
///
/// Has a blanket impl: any type satisfying the supertrait bounds is
/// automatically a `PieceKind`. The user's piece enum typically
/// derives `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`,
/// `PartialOrd`, `Ord`, `Hash`.
pub trait PieceKind: Copy + Eq + Ord + Hash + Debug {}

impl<T: Copy + Eq + Ord + Hash + Debug> PieceKind for T {}
