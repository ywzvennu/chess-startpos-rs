//! [`Problem`] — the constraint-satisfaction specification and its
//! enumerate / count entry points.

use crate::{Constraint, PieceKind, SquareColor};

/// A constraint-satisfaction problem: a fixed board (size + per-square
/// colour), a multiset of pieces, and a constraint to satisfy.
///
/// Solve it by calling [`Problem::count`] for the population size or
/// [`Problem::iter`] to stream all satisfying arrangements.
#[derive(Clone, Debug)]
pub struct Problem<P: PieceKind> {
    /// Number of squares (e.g. `8` for a chess back rank).
    pub num_squares: usize,
    /// Colour of each square. `square_colors.len() == num_squares`.
    pub square_colors: Vec<SquareColor>,
    /// Multiset of pieces to arrange. `pieces.len() == num_squares`.
    pub pieces: Vec<P>,
    /// Root constraint. Use [`Constraint::And`] for conjunctions.
    pub constraint: Constraint<P>,
}

impl<P: PieceKind> Problem<P> {
    /// Number of distinct arrangements satisfying the constraint.
    #[must_use]
    pub fn count(&self) -> u64 {
        self.iter().count() as u64
    }

    /// Streams all distinct arrangements satisfying the constraint, in
    /// canonical lexicographic order over the sorted piece multiset.
    pub fn iter(&self) -> ProblemIter<'_, P> {
        let mut start = self.pieces.clone();
        start.sort();
        ProblemIter {
            problem: self,
            current: Some(start),
        }
    }

    /// Returns the `index`-th satisfying arrangement in canonical
    /// lexicographic order, or `None` if `index >= self.count()`.
    ///
    /// Equivalent to `self.iter().nth(index)` with `u64` indexing.
    #[must_use]
    pub fn at(&self, index: u64) -> Option<Vec<P>> {
        let idx = usize::try_from(index).ok()?;
        self.iter().nth(idx)
    }

    /// Returns a uniformly-random arrangement satisfying the
    /// constraint, deterministic in `seed`. `None` if the constraint
    /// is unsatisfiable.
    #[must_use]
    pub fn sample(&self, seed: u64) -> Option<Vec<P>> {
        let total = self.count();
        if total == 0 {
            return None;
        }
        let mut rng = fastrand::Rng::with_seed(seed);
        let idx = rng.u64(..total);
        self.at(idx)
    }

    /// Returns a copy of `self` with `c` added via AND-composition.
    #[must_use]
    pub fn with_constraint(mut self, c: Constraint<P>) -> Self {
        self.constraint = match self.constraint {
            Constraint::And(mut cs) => {
                cs.push(c);
                Constraint::And(cs)
            }
            existing => Constraint::And(vec![existing, c]),
        };
        self
    }
}

/// Iterator over satisfying arrangements; see [`Problem::iter`].
pub struct ProblemIter<'a, P: PieceKind> {
    problem: &'a Problem<P>,
    current: Option<Vec<P>>,
}

impl<P: PieceKind> Iterator for ProblemIter<'_, P> {
    type Item = Vec<P>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let candidate = self.current.take()?;
            let mut next = candidate.clone();
            if !next_permutation(&mut next) {
                // candidate is the last permutation; emit (if it
                // satisfies the constraint) then terminate.
                self.current = None;
            } else {
                self.current = Some(next);
            }
            if candidate.len() == self.problem.num_squares
                && self
                    .problem
                    .constraint
                    .evaluate(&candidate, &self.problem.square_colors)
            {
                return Some(candidate);
            }
        }
    }
}

/// Standard next-permutation algorithm (lexicographic). For multisets
/// this naturally skips duplicates because equal elements never
/// produce a new ordering. Returns `false` if `v` was already the
/// last permutation.
fn next_permutation<T: Ord>(v: &mut [T]) -> bool {
    let n = v.len();
    if n < 2 {
        return false;
    }
    // Step 1: find the largest i such that v[i-1] < v[i].
    let mut i = n - 1;
    while i > 0 && v[i - 1] >= v[i] {
        i -= 1;
    }
    if i == 0 {
        return false;
    }
    // Step 2: find the largest j >= i such that v[j] > v[i-1].
    let mut j = n - 1;
    while v[j] <= v[i - 1] {
        j -= 1;
    }
    // Step 3: swap v[i-1] and v[j].
    v.swap(i - 1, j);
    // Step 4: reverse v[i..].
    v[i..].reverse();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CountOp;

    /// A tiny piece kind for unit tests.
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
    enum Tile {
        A,
        B,
        C,
    }

    fn light_dark(n: usize) -> Vec<SquareColor> {
        (0..n)
            .map(|i| {
                if i % 2 == 0 {
                    SquareColor::Light
                } else {
                    SquareColor::Dark
                }
            })
            .collect()
    }

    #[test]
    fn empty_and_is_always_true() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::And(vec![]),
        };
        assert_eq!(problem.count(), 6); // 3! permutations, all distinct
    }

    #[test]
    fn count_constraint() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::A, Tile::B],
            constraint: Constraint::Count {
                piece: Tile::A,
                op: CountOp::Eq,
                value: 2,
            },
        };
        // 3!/2! = 3 distinct arrangements; all satisfy the count.
        assert_eq!(problem.count(), 3);
    }

    #[test]
    fn at_constraint() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::At {
                piece: Tile::A,
                square: 0,
            },
        };
        // A fixed at square 0 leaves 2! = 2 arrangements of {B, C}.
        assert_eq!(problem.count(), 2);
    }

    #[test]
    fn not_at_constraint() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::NotAt {
                piece: Tile::A,
                square: 0,
            },
        };
        // Of 6 perms, those without A at square 0 = 6 - 2 = 4.
        assert_eq!(problem.count(), 4);
    }

    #[test]
    fn order_constraint_king_between_rooks() {
        // 3 squares, pieces R K R. "K between rooks" means rook[0] <
        // king[0] < rook[1].
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        enum Piece {
            R,
            K,
        }
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Piece::R, Piece::K, Piece::R],
            constraint: Constraint::Order(vec![(Piece::R, 0), (Piece::K, 0), (Piece::R, 1)]),
        };
        // Only one arrangement: RKR. (Two rooks are indistinguishable,
        // so RKR and "swapped rooks RKR" are the same permutation.)
        assert_eq!(problem.count(), 1);
    }

    #[test]
    fn order_with_out_of_range_instance_is_unsatisfied() {
        // Reference (B, 2) when only two B's exist. No arrangement
        // satisfies the chain — count must be zero.
        let problem = Problem {
            num_squares: 4,
            square_colors: light_dark(4),
            pieces: vec![Tile::A, Tile::B, Tile::B, Tile::C],
            constraint: Constraint::Order(vec![
                (Tile::B, 0),
                (Tile::B, 1),
                (Tile::B, 2), // does not exist
            ]),
        };
        assert_eq!(problem.count(), 0);
    }

    #[test]
    fn relative_constraint_exact_offset() {
        // Tile::A exactly one square right of Tile::B.
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::Relative {
                lhs: (Tile::A, 0),
                rhs: (Tile::B, 0),
                op: CountOp::Eq,
                offset: 1,
            },
        };
        // BAC (B=0, A=1, C=2), CBA (C=0, B=1, A=2) — A is one right of
        // B in both. Other arrangements either fail or have A left of B.
        let arrangements: Vec<Vec<Tile>> = problem.iter().collect();
        assert_eq!(
            arrangements,
            vec![
                vec![Tile::B, Tile::A, Tile::C],
                vec![Tile::C, Tile::B, Tile::A],
            ],
        );
    }

    #[test]
    fn relative_constraint_absolute_distance_via_and() {
        // |A - B| <= 1 expressed as And([Le, Ge]).
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::And(vec![
                Constraint::Relative {
                    lhs: (Tile::A, 0),
                    rhs: (Tile::B, 0),
                    op: CountOp::Le,
                    offset: 1,
                },
                Constraint::Relative {
                    lhs: (Tile::A, 0),
                    rhs: (Tile::B, 0),
                    op: CountOp::Ge,
                    offset: -1,
                },
            ]),
        };
        // A and B must be adjacent. ABC, BAC, BCA, CAB are the 4
        // perms (out of 6) with |A-B| == 1.
        assert_eq!(problem.count(), 4);
    }

    #[test]
    fn relative_with_out_of_range_instance_is_unsatisfied() {
        // Reference (A, 1) when only one A exists.
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::Relative {
                lhs: (Tile::A, 1), // does not exist
                rhs: (Tile::B, 0),
                op: CountOp::Eq,
                offset: 0,
            },
        };
        assert_eq!(problem.count(), 0);
    }

    #[test]
    fn and_or_not_combinators() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::And(vec![
                Constraint::Not(Box::new(Constraint::At {
                    piece: Tile::A,
                    square: 0,
                })),
                Constraint::Or(vec![
                    Constraint::At {
                        piece: Tile::B,
                        square: 0,
                    },
                    Constraint::At {
                        piece: Tile::C,
                        square: 0,
                    },
                ]),
            ]),
        };
        // Of 6 perms, square 0 must be B or C (not A) — that's all 4
        // arrangements with A not at 0.
        assert_eq!(problem.count(), 4);
    }

    #[test]
    fn count_on_color() {
        // Two A's that must both be on light squares: light squares are
        // 0 and 2 → arrangement is A B A only (one arrangement) since
        // B is at square 1, A's at 0 and 2.
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3), // light, dark, light
            pieces: vec![Tile::A, Tile::A, Tile::B],
            constraint: Constraint::CountOnColor {
                piece: Tile::A,
                color: SquareColor::Light,
                op: CountOp::Eq,
                value: 2,
            },
        };
        assert_eq!(problem.count(), 1);
    }

    #[test]
    fn iterator_yields_distinct_arrangements() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::A, Tile::B],
            constraint: Constraint::And(vec![]),
        };
        let arrangements: Vec<Vec<Tile>> = problem.iter().collect();
        // 3!/2! = 3 distinct lexicographic orderings:
        // [A, A, B], [A, B, A], [B, A, A]
        assert_eq!(
            arrangements,
            vec![
                vec![Tile::A, Tile::A, Tile::B],
                vec![Tile::A, Tile::B, Tile::A],
                vec![Tile::B, Tile::A, Tile::A],
            ],
        );
    }

    #[test]
    fn at_returns_lexicographic_arrangements() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::A, Tile::B],
            constraint: Constraint::And(vec![]),
        };
        assert_eq!(problem.at(0), Some(vec![Tile::A, Tile::A, Tile::B]));
        assert_eq!(problem.at(1), Some(vec![Tile::A, Tile::B, Tile::A]));
        assert_eq!(problem.at(2), Some(vec![Tile::B, Tile::A, Tile::A]));
        assert_eq!(problem.at(3), None);
    }

    #[test]
    fn sample_is_deterministic_and_in_range() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::And(vec![]),
        };
        let arrangements: Vec<Vec<Tile>> = problem.iter().collect();

        let first = problem.sample(42).expect("non-empty");
        let again = problem.sample(42).expect("non-empty");
        assert_eq!(first, again, "sample must be deterministic in seed");
        assert!(
            arrangements.contains(&first),
            "sample must be a valid arrangement"
        );
    }

    #[test]
    fn sample_returns_none_for_unsatisfiable() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            // Impossible: piece A must be at square 0 AND not at square 0.
            constraint: Constraint::And(vec![
                Constraint::At {
                    piece: Tile::A,
                    square: 0,
                },
                Constraint::NotAt {
                    piece: Tile::A,
                    square: 0,
                },
            ]),
        };
        assert_eq!(problem.count(), 0);
        assert_eq!(problem.sample(0), None);
    }

    #[test]
    fn with_constraint_adds_via_and() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::And(vec![]),
        };
        assert_eq!(problem.count(), 6);
        let narrowed = problem.with_constraint(Constraint::At {
            piece: Tile::A,
            square: 0,
        });
        assert_eq!(narrowed.count(), 2);
    }
}
