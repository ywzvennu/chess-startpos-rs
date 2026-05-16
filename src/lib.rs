//! Generate, count, and sample chess back-rank arrangements under
//! composable constraints.
//!
//! The crate is generic — parameterised over a user-defined piece
//! kind, generic over board size — and built around a small set of
//! composable constraint primitives with `And` / `Or` / `Not`
//! combinators. An opinionated [`chess`] module ships ready-to-use
//! presets for the canonical chess shuffle variants (Chess960,
//! Chess-2880, full shuffle).
//!
//! # Quick start
//!
//! ```
//! use chess_startpos_rs::chess;
//!
//! assert_eq!(chess::shuffle().count(), 5040);
//! assert_eq!(chess::chess_2880().count(), 2880);
//! assert_eq!(chess::chess_960().count(), 960);
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
