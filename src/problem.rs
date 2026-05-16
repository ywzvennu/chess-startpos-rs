//! [`Problem`] — the constraint-satisfaction specification and its
//! enumerate / count entry points.

use crate::{ColorKind, Constraint, PieceKind, SquareColor};

/// A constraint-satisfaction problem: a fixed board (size + per-square
/// colour from a user-defined colour set), an alphabet of available
/// piece kinds, and a constraint to satisfy.
///
/// `pieces` is the **alphabet** — the set of distinct kinds available.
/// Duplicate entries are silently deduplicated; first-appearance order
/// is preserved. The multiset to permute is derived from
/// [`Constraint::Count`] entries with `op = CountOp::Eq` keyed by
/// alphabet members (see the crate-level docs).
///
/// `C` is the colour kind. The default is [`SquareColor`] — the
/// binary light/dark partition used by chess. For N-way partitions
/// define your own enum and use it as the `C` type parameter.
///
/// Solve by calling [`Problem::count`] for the population size or
/// [`Problem::iter`] to stream all satisfying arrangements.
#[derive(Clone, Debug)]
pub struct Problem<P: PieceKind, C: ColorKind = SquareColor> {
    /// Number of squares (e.g. `8` for a chess back rank).
    pub num_squares: usize,
    /// Colour of each square. `square_colors.len() == num_squares`.
    pub square_colors: Vec<C>,
    /// Alphabet of available piece kinds. Duplicates are silently
    /// deduped by the solver; size doesn't have to equal
    /// `num_squares` — the multiset to permute is derived from
    /// `Constraint::Count{Eq}` constraints in `constraint`.
    pub pieces: Vec<P>,
    /// Root constraint. Use [`Constraint::And`] for conjunctions.
    pub constraint: Constraint<P, C>,
}

impl<P: PieceKind, C: ColorKind> Problem<P, C> {
    /// Number of distinct arrangements satisfying the constraint.
    #[must_use]
    pub fn count(&self) -> u64 {
        self.iter().count() as u64
    }

    /// Streams all distinct arrangements satisfying the constraint,
    /// in canonical lexicographic order over the sorted derived
    /// multiset.
    pub fn iter(&self) -> impl Iterator<Item = Vec<P>> + '_ {
        ProblemIter::new(self)
    }

    /// Returns the `index`-th satisfying arrangement in canonical
    /// lexicographic order, or `None` if `index >= self.count()`.
    ///
    /// Equivalent to `self.iter().nth(index)` with `u64` indexing.
    /// O(index) — for repeated indexed lookup prefer iterating once.
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
    pub fn with_constraint(mut self, c: Constraint<P, C>) -> Self {
        self.constraint = match self.constraint {
            Constraint::And(mut cs) => {
                cs.push(c);
                Constraint::And(cs)
            }
            existing => Constraint::And(vec![existing, c]),
        };
        self
    }

    /// Returns the alphabet with duplicates removed, preserving
    /// first-appearance order.
    fn dedup_alphabet(&self) -> Vec<P> {
        let mut seen: Vec<P> = Vec::with_capacity(self.pieces.len());
        for p in &self.pieces {
            if !seen.contains(p) {
                seen.push(*p);
            }
        }
        seen
    }

    /// Builds the starting sequence the enumerator iterates over.
    ///
    /// If every alphabet member has a `Constraint::Count { piece, op:
    /// Eq, value }` and the values sum to `num_squares`, return the
    /// explicit sorted multiset. Otherwise return all length-N
    /// candidates implicitly by returning `None` — the iterator
    /// falls back to enumerating Cartesian-product sequences from
    /// the alphabet under partial / no count constraints.
    fn fully_constrained_multiset(&self) -> Option<Vec<P>> {
        let alphabet = self.dedup_alphabet();
        if alphabet.is_empty() {
            return None;
        }

        let counts = self.constraint.collect_eq_counts();
        if counts.is_empty() {
            return None;
        }

        let mut multiset: Vec<P> = Vec::with_capacity(self.num_squares);
        let mut total = 0usize;
        for kind in &alphabet {
            // First Eq-count wins if the user accidentally specified
            // two conflicting counts; the iterator filter will then
            // catch the contradiction by emitting zero arrangements.
            let count = counts.iter().find(|(p, _)| p == kind).map(|(_, n)| *n)?;
            for _ in 0..count {
                multiset.push(*kind);
            }
            total += count;
        }

        if total != self.num_squares {
            return None;
        }
        multiset.sort();
        Some(multiset)
    }
}

/// Iterator over satisfying arrangements; see [`Problem::iter`].
struct ProblemIter<'a, P: PieceKind, C: ColorKind> {
    problem: &'a Problem<P, C>,
    state: IterState<P>,
}

enum IterState<P: PieceKind> {
    /// Fully constrained — iterate distinct multiset permutations
    /// via next-permutation. `current` is the next candidate to emit
    /// (after filtering).
    Permutation { current: Option<Vec<P>> },
    /// Partial / unconstrained — iterate Cartesian-product
    /// length-N sequences from the alphabet. `current` is the next
    /// candidate to emit.
    Cartesian {
        alphabet: Vec<P>,
        current: Option<Vec<P>>,
    },
}

impl<'a, P: PieceKind, C: ColorKind> ProblemIter<'a, P, C> {
    fn new(problem: &'a Problem<P, C>) -> Self {
        let state = match problem.fully_constrained_multiset() {
            Some(start) => IterState::Permutation {
                current: Some(start),
            },
            None => {
                let alphabet = problem.dedup_alphabet();
                let current = if alphabet.is_empty() || problem.num_squares == 0 {
                    None
                } else {
                    Some(vec![alphabet[0]; problem.num_squares])
                };
                IterState::Cartesian { alphabet, current }
            }
        };
        Self { problem, state }
    }
}

impl<P: PieceKind, C: ColorKind> Iterator for ProblemIter<'_, P, C> {
    type Item = Vec<P>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let candidate = match &mut self.state {
                IterState::Permutation { current } => {
                    let candidate = current.take()?;
                    let mut next = candidate.clone();
                    if next_permutation(&mut next) {
                        *current = Some(next);
                    }
                    candidate
                }
                IterState::Cartesian { alphabet, current } => {
                    let candidate = current.take()?;
                    let mut next = candidate.clone();
                    if next_cartesian(&mut next, alphabet) {
                        *current = Some(next);
                    }
                    candidate
                }
            };

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

/// Lexicographically advances `v` to the next length-`v.len()`
/// sequence drawn from `alphabet` (Cartesian product, repetition
/// allowed). Returns `false` if `v` was already the last sequence.
fn next_cartesian<P: PieceKind>(v: &mut [P], alphabet: &[P]) -> bool {
    if alphabet.is_empty() {
        return false;
    }
    let last = alphabet[alphabet.len() - 1];
    for i in (0..v.len()).rev() {
        if v[i] != last {
            let pos = alphabet
                .iter()
                .position(|p| p == &v[i])
                .expect("in alphabet");
            v[i] = alphabet[pos + 1];
            for slot in v.iter_mut().skip(i + 1) {
                *slot = alphabet[0];
            }
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{alternating, CountOp};

    /// A tiny piece kind for unit tests.
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
    enum Tile {
        A,
        B,
        C,
    }

    fn light_dark(n: usize) -> Vec<SquareColor> {
        alternating(n, SquareColor::Light, SquareColor::Dark)
    }

    fn fixed_counts(items: &[(Tile, usize)]) -> Constraint<Tile> {
        Constraint::And(
            items
                .iter()
                .map(|(p, n)| Constraint::Count {
                    piece: *p,
                    op: CountOp::Eq,
                    value: *n,
                })
                .collect(),
        )
    }

    #[test]
    fn count_constraint_drives_multiset() {
        // Alphabet {A, B}; counts {A: 2, B: 1}; expect 3 perms over the
        // multiset [A, A, B].
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B],
            constraint: fixed_counts(&[(Tile::A, 2), (Tile::B, 1)]),
        };
        assert_eq!(problem.count(), 3);
    }

    #[test]
    fn duplicates_in_pieces_are_deduped() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::A, Tile::B], // duplicate Tile::A
            constraint: fixed_counts(&[(Tile::A, 2), (Tile::B, 1)]),
        };
        assert_eq!(problem.count(), 3);
    }

    #[test]
    fn at_constraint() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::And(vec![
                Constraint::Count {
                    piece: Tile::A,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::B,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::C,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::At {
                    piece: Tile::A,
                    square: 0,
                },
            ]),
        };
        assert_eq!(problem.count(), 2);
    }

    #[test]
    fn unconstrained_cartesian_fallback() {
        // No Count-Eq constraints; alphabet of 2 on 3 squares = 2^3 = 8.
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B],
            constraint: Constraint::And(vec![]),
        };
        assert_eq!(problem.count(), 8);
    }

    #[test]
    fn unconstrained_with_at_filter() {
        // Cartesian fallback + At filter.
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B],
            constraint: Constraint::At {
                piece: Tile::A,
                square: 0,
            },
        };
        // Tile::A at square 0 + any of 2^2 sequences on squares 1, 2.
        assert_eq!(problem.count(), 4);
    }

    #[test]
    fn order_constraint_king_between_rooks() {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        enum Piece {
            R,
            K,
        }
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Piece::R, Piece::K],
            constraint: Constraint::And(vec![
                Constraint::Count {
                    piece: Piece::R,
                    op: CountOp::Eq,
                    value: 2,
                },
                Constraint::Count {
                    piece: Piece::K,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Order(vec![(Piece::R, 0), (Piece::K, 0), (Piece::R, 1)]),
            ]),
        };
        assert_eq!(problem.count(), 1);
    }

    #[test]
    fn order_with_out_of_range_instance_is_unsatisfied() {
        let problem = Problem {
            num_squares: 4,
            square_colors: light_dark(4),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::And(vec![
                Constraint::Count {
                    piece: Tile::A,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::B,
                    op: CountOp::Eq,
                    value: 2,
                },
                Constraint::Count {
                    piece: Tile::C,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Order(vec![
                    (Tile::B, 0),
                    (Tile::B, 1),
                    (Tile::B, 2), // doesn't exist
                ]),
            ]),
        };
        assert_eq!(problem.count(), 0);
    }

    #[test]
    fn relative_constraint_exact_offset() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::And(vec![
                Constraint::Count {
                    piece: Tile::A,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::B,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::C,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Relative {
                    lhs: (Tile::A, 0),
                    rhs: (Tile::B, 0),
                    op: CountOp::Eq,
                    offset: 1,
                },
            ]),
        };
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
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::And(vec![
                Constraint::Count {
                    piece: Tile::A,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::B,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::C,
                    op: CountOp::Eq,
                    value: 1,
                },
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
        assert_eq!(problem.count(), 4);
    }

    #[test]
    fn and_or_not_combinators() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::And(vec![
                Constraint::Count {
                    piece: Tile::A,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::B,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::C,
                    op: CountOp::Eq,
                    value: 1,
                },
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
        assert_eq!(problem.count(), 4);
    }

    #[test]
    fn count_on_color() {
        // Two A's that must both be on light squares (positions 0, 2).
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3), // light, dark, light
            pieces: vec![Tile::A, Tile::B],
            constraint: Constraint::And(vec![
                Constraint::Count {
                    piece: Tile::A,
                    op: CountOp::Eq,
                    value: 2,
                },
                Constraint::Count {
                    piece: Tile::B,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::CountOnColor {
                    piece: Tile::A,
                    color: SquareColor::Light,
                    op: CountOp::Eq,
                    value: 2,
                },
            ]),
        };
        assert_eq!(problem.count(), 1);
    }

    #[test]
    fn at_returns_lexicographic_arrangements() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B],
            constraint: fixed_counts(&[(Tile::A, 2), (Tile::B, 1)]),
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
            constraint: Constraint::And(vec![
                Constraint::Count {
                    piece: Tile::A,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::B,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::C,
                    op: CountOp::Eq,
                    value: 1,
                },
            ]),
        };
        let arrangements: Vec<Vec<Tile>> = problem.iter().collect();
        let first = problem.sample(42).expect("non-empty");
        let again = problem.sample(42).expect("non-empty");
        assert_eq!(first, again);
        assert!(arrangements.contains(&first));
    }

    #[test]
    fn sample_returns_none_for_unsatisfiable() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B, Tile::C],
            constraint: Constraint::And(vec![
                Constraint::Count {
                    piece: Tile::A,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::B,
                    op: CountOp::Eq,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::C,
                    op: CountOp::Eq,
                    value: 1,
                },
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
            constraint: fixed_counts(&[(Tile::A, 1), (Tile::B, 1), (Tile::C, 1)]),
        };
        assert_eq!(problem.count(), 6);
        let narrowed = problem.with_constraint(Constraint::At {
            piece: Tile::A,
            square: 0,
        });
        assert_eq!(narrowed.count(), 2);
    }
}
