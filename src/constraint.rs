//! Constraint primitives and combinators.

use crate::{ColorKind, PieceKind};

/// Colour of a square — the default binary partition used by chess
/// (and the default type parameter `C` for [`Constraint`] /
/// [`crate::Problem`]).
///
/// For N-way colour partitions, define your own enum and use it as
/// the `C` type parameter. Any `Copy + Eq + Hash + Debug` type
/// satisfies [`ColorKind`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum SquareColor {
    /// Light-coloured square.
    Light,
    /// Dark-coloured square.
    Dark,
}

/// Comparison operator for count constraints.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum CountOp {
    /// Equal.
    Eq,
    /// Not equal.
    NotEq,
    /// Less than or equal.
    Le,
    /// Less than.
    Lt,
    /// Greater than or equal.
    Ge,
    /// Greater than.
    Gt,
}

impl CountOp {
    /// Returns whether `lhs` and `rhs` satisfy this comparison.
    ///
    /// Generic over any [`Ord`] type, so the same operator works for
    /// counts (`usize`) and signed positional offsets (`i32`).
    #[must_use]
    pub fn check<T: Ord>(self, lhs: T, rhs: T) -> bool {
        match self {
            Self::Eq => lhs == rhs,
            Self::NotEq => lhs != rhs,
            Self::Le => lhs <= rhs,
            Self::Lt => lhs < rhs,
            Self::Ge => lhs >= rhs,
            Self::Gt => lhs > rhs,
        }
    }
}

/// A single constraint over an arrangement of pieces.
///
/// Primitive constraints test a property of the arrangement;
/// combinator constraints (`And` / `Or` / `Not`) compose them.
///
/// `P` is the piece kind. `C` is the colour kind (defaults to
/// [`SquareColor`] — the binary light/dark partition).
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "P: serde::Serialize, C: serde::Serialize",
        deserialize = "P: serde::Deserialize<'de>, C: serde::Deserialize<'de>"
    ))
)]
#[non_exhaustive]
pub enum Constraint<P, C = SquareColor> {
    /// Number of occurrences of `piece` across the arrangement
    /// satisfies `(op, value)`.
    Count {
        /// Piece kind to count.
        piece: P,
        /// Comparison operator.
        op: CountOp,
        /// Right-hand value.
        value: usize,
    },
    /// Number of occurrences of `piece` on squares of the given colour
    /// satisfies `(op, value)`. Used for e.g. "bishops on opposite
    /// colours" by requiring one bishop on each colour.
    CountOnColor {
        /// Piece kind to count.
        piece: P,
        /// Square colour to count on.
        color: C,
        /// Comparison operator.
        op: CountOp,
        /// Right-hand value.
        value: usize,
    },
    /// The `piece` kind must occupy the given square index.
    /// Satisfied if any occurrence of `piece` is at `square`.
    At {
        /// Piece kind.
        piece: P,
        /// Square index.
        square: usize,
    },
    /// The `piece` kind must NOT occupy the given square index.
    NotAt {
        /// Piece kind.
        piece: P,
        /// Square index.
        square: usize,
    },
    /// Strict positional ordering: the indexed instances of the listed
    /// pieces must appear in strictly increasing square order.
    ///
    /// `Order(vec![(Rook, 0), (King, 0), (Rook, 1)])` is read as
    /// `rook[0] < king[0] < rook[1]`, i.e. the king lies strictly
    /// between the two rooks.
    ///
    /// If any `(piece, instance_idx)` in the chain references an
    /// instance that does not exist in the arrangement (e.g.
    /// `(Bishop, 2)` when only two bishops were declared via
    /// `Constraint::Count { Eq, 2 }`), the constraint is
    /// **unsatisfied** for that arrangement. The chain silently
    /// fails — it does not panic. For stricter upfront checking,
    /// call [`crate::Problem::validate`] (or use
    /// [`crate::ProblemBuilder::try_build`]).
    Order(Vec<(P, usize)>),
    /// Relative positional constraint between two specific piece
    /// instances:
    ///
    /// ```text
    /// (lhs.0[lhs.1].square as i32 - rhs.0[rhs.1].square as i32) op offset
    /// ```
    ///
    /// `Relative { lhs: (King, 0), rhs: (Queen, 0), op: CountOp::Eq, offset: 2 }`
    /// reads as "the king is exactly 2 squares to the right of the
    /// queen". Absolute distance `<= k` between two instances can be
    /// expressed as `And([Relative { op: Le, offset: k }, Relative { op:
    /// Ge, offset: -k }])` with matching lhs / rhs.
    ///
    /// If either `lhs.1` or `rhs.1` references an instance that
    /// doesn't exist in the arrangement, the constraint is
    /// **unsatisfied** for that arrangement (same convention as
    /// [`Constraint::Order`]).
    ///
    /// `offset` is `i32` because the difference of two `usize`
    /// squares is signed. Square indices are cast to `i32` before
    /// the subtraction, so boards with `num_squares > i32::MAX` are
    /// not supported by this constraint (chess uses 8).
    ///
    /// ```
    /// use chess_startpos_rs::{chess, Constraint, CountOp};
    ///
    /// // King exactly two squares to the right of the queen.
    /// let _ = Constraint::<chess::Piece>::Relative {
    ///     lhs: (chess::Piece::King, 0),
    ///     rhs: (chess::Piece::Queen, 0),
    ///     op: CountOp::Eq,
    ///     offset: 2,
    /// };
    /// ```
    Relative {
        /// Left-hand piece instance.
        lhs: (P, usize),
        /// Right-hand piece instance.
        rhs: (P, usize),
        /// Comparison operator applied to `(lhs - rhs) op offset`.
        op: CountOp,
        /// Signed offset on the right-hand side.
        offset: i32,
    },
    /// Logical AND: all child constraints must hold.
    And(Vec<Constraint<P, C>>),
    /// Logical OR: at least one child constraint must hold.
    Or(Vec<Constraint<P, C>>),
    /// Logical NOT: child constraint must not hold.
    Not(Box<Constraint<P, C>>),
}

impl<P: PieceKind, C: ColorKind> Constraint<P, C> {
    /// Returns whether `arrangement` satisfies this constraint.
    ///
    /// `arrangement.len()` and `colors.len()` must agree.
    #[must_use]
    pub fn evaluate(&self, arrangement: &[P], colors: &[C]) -> bool {
        match self {
            Self::Count { piece, op, value } => {
                let n = arrangement.iter().filter(|p| *p == piece).count();
                op.check(n, *value)
            }
            Self::CountOnColor {
                piece,
                color,
                op,
                value,
            } => {
                let n = arrangement
                    .iter()
                    .zip(colors.iter())
                    .filter(|(p, c)| *p == piece && *c == color)
                    .count();
                op.check(n, *value)
            }
            Self::At { piece, square } => arrangement.get(*square) == Some(piece),
            Self::NotAt { piece, square } => arrangement.get(*square) != Some(piece),
            Self::Relative {
                lhs,
                rhs,
                op,
                offset,
            } => {
                let lhs_pos = nth_position(arrangement, &lhs.0, lhs.1);
                let rhs_pos = nth_position(arrangement, &rhs.0, rhs.1);
                match (lhs_pos, rhs_pos) {
                    (Some(l), Some(r)) => {
                        let diff = (l as i32) - (r as i32);
                        op.check(diff, *offset)
                    }
                    _ => false,
                }
            }
            Self::Order(chain) => {
                let mut positions: Vec<usize> = Vec::with_capacity(chain.len());
                for (piece_kind, instance_idx) in chain {
                    let occurrence = nth_position(arrangement, piece_kind, *instance_idx);
                    match occurrence {
                        Some(pos) => positions.push(pos),
                        None => return false,
                    }
                }
                positions.windows(2).all(|w| w[0] < w[1])
            }
            Self::And(children) => children.iter().all(|c| c.evaluate(arrangement, colors)),
            Self::Or(children) => children.iter().any(|c| c.evaluate(arrangement, colors)),
            Self::Not(inner) => !inner.evaluate(arrangement, colors),
        }
    }

    /// Collects every `Constraint::Count { piece, op: Eq, value }`
    /// keyed by `piece` from `self` and its top-level `And`-nested
    /// children. Used by the solver to derive the per-piece counts
    /// that define the multiset for the fast-path enumeration.
    pub(crate) fn collect_eq_counts(&self) -> Vec<(P, usize)> {
        let mut out = Vec::new();
        self.collect_eq_counts_into(&mut out);
        out
    }

    fn collect_eq_counts_into(&self, out: &mut Vec<(P, usize)>) {
        match self {
            Self::Count {
                piece,
                op: CountOp::Eq,
                value,
            } => out.push((*piece, *value)),
            Self::And(children) => {
                for c in children {
                    c.collect_eq_counts_into(out);
                }
            }
            _ => {}
        }
    }

    /// Walks the constraint tree, invoking `visitor` on every node.
    /// Used by validation to check that all `piece` / `color` /
    /// `square` references are consistent with the problem
    /// declarations.
    pub(crate) fn walk(&self, visitor: &mut impl FnMut(&Self)) {
        visitor(self);
        match self {
            Self::And(children) | Self::Or(children) => {
                for c in children {
                    c.walk(visitor);
                }
            }
            Self::Not(inner) => inner.walk(visitor),
            _ => {}
        }
    }
}

/// Returns the square index of the `instance_idx`-th occurrence of
/// `piece` in `arrangement`, or `None` if fewer than `instance_idx + 1`
/// occurrences exist.
fn nth_position<P: PieceKind>(arrangement: &[P], piece: &P, instance_idx: usize) -> Option<usize> {
    arrangement
        .iter()
        .enumerate()
        .filter_map(|(i, p)| (p == piece).then_some(i))
        .nth(instance_idx)
}
