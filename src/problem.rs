//! [`Problem`] — the constraint-satisfaction specification and its
//! enumerate / count entry points.

use std::collections::HashMap;

use crate::{ColorKind, Constraint, PieceKind, SquareColor};

/// A constraint-satisfaction problem: a fixed board (size + per-square
/// colour from a user-defined colour set), an alphabet of available
/// piece kinds, and a constraint to satisfy.
///
/// `pieces` is the **alphabet** — the set of distinct kinds available.
/// Duplicate entries are silently deduplicated; first-appearance order
/// is preserved.
///
/// `C` is the colour kind. The default is [`SquareColor`] — the
/// binary light/dark partition used by chess. For N-way partitions
/// define your own enum and use it as the `C` type parameter.
///
/// Solve by calling [`Problem::count`] for the population size or
/// [`Problem::iter`] to stream all satisfying arrangements.
///
/// If `pieces` is empty or `num_squares == 0`, the problem has no
/// arrangements to enumerate and `count()` returns `0`.
#[derive(Clone, Debug)]
pub struct Problem<P: PieceKind, C: ColorKind = SquareColor> {
    /// Number of squares (e.g. `8` for a chess back rank).
    pub num_squares: usize,
    /// Colour of each square. `square_colors.len() == num_squares`.
    pub square_colors: Vec<C>,
    /// Alphabet of available piece kinds. Duplicates are silently
    /// deduped by the solver; size doesn't have to equal
    /// `num_squares`.
    pub pieces: Vec<P>,
    /// Root constraint. Use [`Constraint::And`] for conjunctions.
    pub constraint: Constraint<P, C>,
}

impl<P: PieceKind, C: ColorKind> Problem<P, C> {
    /// Returns a fresh empty [`ProblemBuilder`] for fluent
    /// construction.
    ///
    /// ```
    /// use chess_startpos_rs::{chess, Constraint, CountOp, Problem, SquareColor};
    ///
    /// let problem: Problem<chess::Piece> = Problem::builder()
    ///     .squares(8)
    ///     .alternating_colors(SquareColor::Dark, SquareColor::Light)
    ///     .pieces([chess::Piece::King, chess::Piece::Queen])
    ///     .constraint(Constraint::Count {
    ///         piece: chess::Piece::King,
    ///         op: CountOp::Eq,
    ///         value: 1,
    ///     })
    ///     .constraint(Constraint::Count {
    ///         piece: chess::Piece::Queen,
    ///         op: CountOp::Eq,
    ///         value: 7,
    ///     })
    ///     .build();
    /// assert_eq!(problem.count(), 8); // 1 king × 8 placements over 7 queens
    /// ```
    ///
    /// The struct-literal API ([`Problem`] is a regular public-fields
    /// struct) remains a fully supported alternative.
    #[must_use]
    pub fn builder() -> ProblemBuilder<P, C> {
        ProblemBuilder::new()
    }

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
    ///
    /// Single pass over `self.iter()` using reservoir-of-size-1
    /// sampling: each satisfying arrangement is selected with
    /// probability `1 / k` where `k` is its 1-based index in the
    /// iterator. The result is uniformly random over the satisfying
    /// arrangements without materialising them all.
    #[must_use]
    pub fn sample(&self, seed: u64) -> Option<Vec<P>> {
        let mut rng = fastrand::Rng::with_seed(seed);
        let mut chosen: Option<Vec<P>> = None;
        for (i, arrangement) in self.iter().enumerate() {
            // 1-based position used as the reservoir denominator.
            let seen = (i as u64).saturating_add(1);
            if rng.u64(0..seen) == 0 {
                chosen = Some(arrangement);
            }
        }
        chosen
    }

    /// Returns a copy of `self` with `c` added via AND-composition.
    #[must_use]
    pub fn with_constraint(&self, c: Constraint<P, C>) -> Self
    where
        C: Clone,
    {
        let mut next = self.clone();
        next.constraint = match next.constraint {
            Constraint::And(mut cs) => {
                cs.push(c);
                Constraint::And(cs)
            }
            existing => Constraint::And(vec![existing, c]),
        };
        next
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

    /// Internal optimisation: when every alphabet member has a
    /// top-level (root or And-nested) `Constraint::Count { Eq, n }`
    /// fixing its multiplicity and the values sum to `num_squares`,
    /// the enumerator can permute that exact multiset instead of
    /// running the full Cartesian product. Returns `None` (falls
    /// back to Cartesian enumeration) whenever the fast path can't
    /// apply — including when `Count{Eq}` constraints sit inside
    /// `Or` / `Not` (we don't infer counts across disjunction).
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

/// Fluent builder for [`Problem`].
///
/// Construct via [`Problem::builder`]. Every method consumes `self`
/// and returns the updated builder, so calls chain. Finalise with
/// [`ProblemBuilder::build`].
///
/// The builder is an alternative to direct struct-literal
/// construction of `Problem`; both produce the same value.
///
/// ```
/// use chess_startpos_rs::{Constraint, CountOp, Problem, SquareColor};
///
/// #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
/// enum Card { Ace, King, Queen }
///
/// let problem: Problem<Card> = Problem::builder()
///     .squares(6)
///     .alternating_colors(SquareColor::Light, SquareColor::Dark)
///     .pieces([Card::Ace, Card::King, Card::Queen])
///     .constraint(Constraint::Count { piece: Card::Ace,   op: CountOp::Eq, value: 2 })
///     .constraint(Constraint::Count { piece: Card::King,  op: CountOp::Eq, value: 2 })
///     .constraint(Constraint::Count { piece: Card::Queen, op: CountOp::Eq, value: 2 })
///     .build();
///
/// assert_eq!(problem.count(), 90); // 6! / (2! · 2! · 2!)
/// ```
#[derive(Clone, Debug)]
pub struct ProblemBuilder<P: PieceKind, C: ColorKind> {
    num_squares: usize,
    square_colors: Vec<C>,
    pieces: Vec<P>,
    constraints: Vec<Constraint<P, C>>,
}

impl<P: PieceKind, C: ColorKind> Default for ProblemBuilder<P, C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: PieceKind, C: ColorKind> ProblemBuilder<P, C> {
    /// Returns a fresh empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            num_squares: 0,
            square_colors: Vec::new(),
            pieces: Vec::new(),
            constraints: Vec::new(),
        }
    }

    /// Sets the board size.
    #[must_use]
    pub fn squares(mut self, n: usize) -> Self {
        self.num_squares = n;
        self
    }

    /// Sets `square_colors` to `colors`. The slice's length should
    /// match `num_squares`; mismatch isn't a build-time error but
    /// will produce empty results from the solver.
    #[must_use]
    pub fn colors(mut self, colors: Vec<C>) -> Self {
        self.square_colors = colors;
        self
    }

    /// Sets `square_colors` to a length-`num_squares` sequence
    /// alternating between `first` (even indices) and `second` (odd
    /// indices). Call after `.squares(n)`; otherwise produces an
    /// empty colour vector.
    #[must_use]
    pub fn alternating_colors(mut self, first: C, second: C) -> Self
    where
        C: Copy,
    {
        self.square_colors = crate::alternating(self.num_squares, first, second);
        self
    }

    /// Sets `square_colors` to a length-`num_squares` sequence with
    /// every square coloured `c`. Call after `.squares(n)`.
    #[must_use]
    pub fn uniform_colors(mut self, c: C) -> Self
    where
        C: Copy,
    {
        self.square_colors = crate::uniform(self.num_squares, c);
        self
    }

    /// Replaces the alphabet with `alphabet`. Duplicates are silently
    /// deduplicated by the solver.
    #[must_use]
    pub fn pieces<I: IntoIterator<Item = P>>(mut self, alphabet: I) -> Self {
        self.pieces = alphabet.into_iter().collect();
        self
    }

    /// Appends `p` to the alphabet.
    #[must_use]
    pub fn piece(mut self, p: P) -> Self {
        self.pieces.push(p);
        self
    }

    /// Appends a constraint. Multiple calls AND-compose at
    /// [`build`](Self::build) time.
    #[must_use]
    pub fn constraint(mut self, c: Constraint<P, C>) -> Self {
        self.constraints.push(c);
        self
    }

    /// Finalises into a [`Problem`]. The accumulated constraints are
    /// wrapped in [`Constraint::And`] (a single constraint is wrapped
    /// alone; zero constraints become `And(vec![])`, which is always
    /// satisfied).
    #[must_use]
    pub fn build(self) -> Problem<P, C> {
        let constraint = match self.constraints.len() {
            0 => Constraint::And(Vec::new()),
            1 => self.constraints.into_iter().next().expect("len == 1"),
            _ => Constraint::And(self.constraints),
        };
        Problem {
            num_squares: self.num_squares,
            square_colors: self.square_colors,
            pieces: self.pieces,
            constraint,
        }
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
    /// candidate to emit. `index` maps each alphabet member to its
    /// position so advancing is O(1) per slot.
    Cartesian {
        alphabet: Vec<P>,
        index: HashMap<P, usize>,
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
                let index = alphabet.iter().enumerate().map(|(i, p)| (*p, i)).collect();
                IterState::Cartesian {
                    alphabet,
                    index,
                    current,
                }
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
                IterState::Cartesian {
                    alphabet,
                    index,
                    current,
                } => {
                    let candidate = current.take()?;
                    let mut next = candidate.clone();
                    if next_cartesian(&mut next, alphabet, index) {
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
/// allowed). `index` maps each alphabet member to its position
/// (built once in [`ProblemIter::new`]). Returns `false` if `v`
/// was already the last sequence.
fn next_cartesian<P: PieceKind>(v: &mut [P], alphabet: &[P], index: &HashMap<P, usize>) -> bool {
    if alphabet.is_empty() {
        return false;
    }
    let last = alphabet[alphabet.len() - 1];
    for i in (0..v.len()).rev() {
        if v[i] != last {
            let pos = *index.get(&v[i]).expect("in alphabet");
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
    fn count_constraint_with_inequality_filters() {
        // Alphabet {A, B} on 3 squares (Cartesian regime — no Count-Eq).
        // Filter to arrangements with at least 1 A and at most 2 A.
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B],
            constraint: Constraint::And(vec![
                Constraint::Count {
                    piece: Tile::A,
                    op: CountOp::Ge,
                    value: 1,
                },
                Constraint::Count {
                    piece: Tile::A,
                    op: CountOp::Le,
                    value: 2,
                },
            ]),
        };
        // 2^3 = 8 total, minus the all-B (0 A's) and all-A (3 A's) extremes.
        assert_eq!(problem.count(), 6);
    }

    #[test]
    fn count_constraint_inside_or_not_treated_as_multiset_declaration() {
        // Or-wrapped Count-Eq must NOT be picked up by the multiset
        // fast path — the solver should fall back to Cartesian
        // enumeration and filter via the Or.
        let problem = Problem {
            num_squares: 2,
            square_colors: light_dark(2),
            pieces: vec![Tile::A, Tile::B],
            constraint: Constraint::Or(vec![
                Constraint::Count {
                    piece: Tile::A,
                    op: CountOp::Eq,
                    value: 2,
                },
                Constraint::Count {
                    piece: Tile::B,
                    op: CountOp::Eq,
                    value: 2,
                },
            ]),
        };
        // Cartesian regime: 2^2 = 4 candidates (AA, AB, BA, BB).
        // AA satisfies first arm, BB satisfies second. Count = 2.
        assert_eq!(problem.count(), 2);
    }

    #[test]
    fn empty_alphabet_yields_zero_arrangements() {
        let problem: Problem<Tile> = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![],
            constraint: Constraint::And(vec![]),
        };
        assert_eq!(problem.count(), 0);
        assert_eq!(problem.at(0), None);
        assert_eq!(problem.sample(0), None);
    }

    #[test]
    fn zero_squares_yields_zero_arrangements() {
        let problem: Problem<Tile> = Problem {
            num_squares: 0,
            square_colors: vec![],
            pieces: vec![Tile::A, Tile::B],
            constraint: Constraint::And(vec![]),
        };
        assert_eq!(problem.count(), 0);
        assert_eq!(problem.at(0), None);
        assert_eq!(problem.sample(0), None);
    }

    #[test]
    fn builder_matches_struct_literal_on_chess_960() {
        // Build the same problem two ways and assert they have the
        // same population.
        let from_preset = crate::chess::chess_960().into_problem();

        use crate::chess::{back_rank_colors, Piece};
        let from_builder: Problem<Piece> = Problem::builder()
            .squares(8)
            .colors(back_rank_colors())
            .pieces([
                Piece::King,
                Piece::Queen,
                Piece::Rook,
                Piece::Bishop,
                Piece::Knight,
            ])
            .constraint(Constraint::Count {
                piece: Piece::King,
                op: CountOp::Eq,
                value: 1,
            })
            .constraint(Constraint::Count {
                piece: Piece::Queen,
                op: CountOp::Eq,
                value: 1,
            })
            .constraint(Constraint::Count {
                piece: Piece::Rook,
                op: CountOp::Eq,
                value: 2,
            })
            .constraint(Constraint::Count {
                piece: Piece::Bishop,
                op: CountOp::Eq,
                value: 2,
            })
            .constraint(Constraint::Count {
                piece: Piece::Knight,
                op: CountOp::Eq,
                value: 2,
            })
            .constraint(Constraint::CountOnColor {
                piece: Piece::Bishop,
                color: SquareColor::Light,
                op: CountOp::Eq,
                value: 1,
            })
            .constraint(Constraint::CountOnColor {
                piece: Piece::Bishop,
                color: SquareColor::Dark,
                op: CountOp::Eq,
                value: 1,
            })
            .constraint(Constraint::Order(vec![
                (Piece::Rook, 0),
                (Piece::King, 0),
                (Piece::Rook, 1),
            ]))
            .build();

        assert_eq!(from_builder.count(), from_preset.count());
        assert_eq!(from_builder.count(), 960);
    }

    #[test]
    fn builder_with_zero_constraints_is_and_empty() {
        let problem: Problem<Tile> = Problem::builder()
            .squares(3)
            .alternating_colors(SquareColor::Light, SquareColor::Dark)
            .pieces([Tile::A, Tile::B])
            .build();
        // Unconstrained Cartesian regime: 2^3 = 8.
        assert_eq!(problem.count(), 8);
        assert!(matches!(problem.constraint, Constraint::And(ref v) if v.is_empty()));
    }

    #[test]
    fn builder_with_single_constraint_unwraps_and() {
        let problem: Problem<Tile> = Problem::builder()
            .squares(3)
            .uniform_colors(SquareColor::Light)
            .pieces([Tile::A, Tile::B])
            .constraint(Constraint::At {
                piece: Tile::A,
                square: 0,
            })
            .build();
        // A pinned at square 0, 2 choices on each remaining square → 4.
        assert_eq!(problem.count(), 4);
        // Single constraint is stored bare, not wrapped in And.
        assert!(matches!(problem.constraint, Constraint::At { .. }));
    }

    #[test]
    fn builder_piece_method_appends() {
        let problem: Problem<Tile> = Problem::builder()
            .squares(3)
            .uniform_colors(SquareColor::Light)
            .piece(Tile::A)
            .piece(Tile::B)
            .constraint(Constraint::Count {
                piece: Tile::A,
                op: CountOp::Eq,
                value: 2,
            })
            .constraint(Constraint::Count {
                piece: Tile::B,
                op: CountOp::Eq,
                value: 1,
            })
            .build();
        assert_eq!(problem.count(), 3);
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
