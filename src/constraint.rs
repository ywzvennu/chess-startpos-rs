//! Constraint primitives and combinators.

use crate::PieceKind;

/// Colour of a square, used by colour-keyed count constraints.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SquareColor {
    /// Light-coloured square.
    Light,
    /// Dark-coloured square.
    Dark,
}

/// Comparison operator for count constraints.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Constraint<P> {
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
        color: SquareColor,
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
    /// `(Bishop, 2)` when the multiset has only two bishops), the
    /// constraint is **unsatisfied** for that arrangement. The chain
    /// silently fails — it does not panic. This means a count of
    /// zero is the visible result of such a mistake; for stricter
    /// upfront checking see issue #14.
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
    /// [`Order`]).
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
    And(Vec<Constraint<P>>),
    /// Logical OR: at least one child constraint must hold.
    Or(Vec<Constraint<P>>),
    /// Logical NOT: child constraint must not hold.
    Not(Box<Constraint<P>>),
}

impl<P: PieceKind> Constraint<P> {
    /// Returns whether `arrangement` satisfies this constraint.
    ///
    /// `arrangement.len()` and `colors.len()` must agree.
    #[must_use]
    pub fn evaluate(&self, arrangement: &[P], colors: &[SquareColor]) -> bool {
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
