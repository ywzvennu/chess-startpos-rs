//! [`Problem`] — the constraint-satisfaction specification and its
//! enumerate / count entry points.

use std::collections::HashMap;
use std::fmt;

use crate::{ColorKind, Constraint, PieceKind, SquareColor};

/// Reasons a [`Problem`] can fail [`validate`](Problem::validate).
///
/// Returned by [`Problem::validate`] and
/// [`ProblemBuilder::try_build`]. Variants only carry the
/// problem-relative indices needed to locate the offending item; they
/// don't carry `P` or `C` values so that the error type can be
/// `Eq + Hash + Clone` without dragging those bounds onto the user's
/// piece / colour kinds.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ValidationError {
    /// `square_colors` is non-empty but its length doesn't match
    /// `num_squares`. An empty `square_colors` is treated as
    /// "no colour partition declared" and is **not** an error —
    /// colour-keyed constraints will see zero matches.
    ColorLengthMismatch {
        /// `num_squares`.
        expected: usize,
        /// `square_colors.len()`.
        actual: usize,
    },
    /// A constraint references a piece kind that is not in the
    /// problem's alphabet (`pieces`).
    UnknownPiece,
    /// A `CountOnColor` constraint references a colour that doesn't
    /// appear in `square_colors`.
    UnknownColor,
    /// An `At` / `NotAt` constraint references a square index
    /// `>= num_squares`.
    SquareOutOfRange {
        /// The offending square index.
        square: usize,
        /// `num_squares`.
        num_squares: usize,
    },
    /// An `Order` or `Relative` constraint references a piece
    /// instance index that exceeds the count declared for that piece
    /// via `Constraint::Count { Eq, n }`. Only checked when the
    /// piece has a declared `Eq`-count.
    InstanceOutOfRange {
        /// The offending instance index.
        instance: usize,
        /// The declared count of that piece.
        declared: usize,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ColorLengthMismatch { expected, actual } => write!(
                f,
                "square_colors has length {actual}, expected {expected} (= num_squares)",
            ),
            Self::UnknownPiece => f.write_str("constraint references a piece not in the alphabet"),
            Self::UnknownColor => {
                f.write_str("CountOnColor references a colour not in square_colors")
            }
            Self::SquareOutOfRange {
                square,
                num_squares,
            } => write!(
                f,
                "constraint references square {square} but num_squares is {num_squares}",
            ),
            Self::InstanceOutOfRange {
                instance,
                declared,
            } => write!(
                f,
                "Order / Relative references instance {instance} of a piece declared with count {declared}",
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

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
/// `square_colors` must either be empty (no colour partition
/// declared; colour-keyed constraints always see zero matches) or
/// have length exactly `num_squares`. Call [`validate`](Self::validate)
/// to check this and that every constraint references only declared
/// pieces, colours, and squares.
///
/// Solve by calling [`Problem::count`] for the population size or
/// [`Problem::iter`] to stream all satisfying arrangements.
///
/// If `pieces` is empty or `num_squares == 0`, the problem has no
/// arrangements to enumerate and `count()` returns `0`.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "P: serde::Serialize, C: serde::Serialize",
        deserialize = "P: serde::Deserialize<'de>, C: serde::Deserialize<'de>"
    ))
)]
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
    /// Checks `self` is internally consistent and returns a
    /// `ValidationError` if not.
    ///
    /// Specifically:
    /// - `square_colors` is either empty or `num_squares` long.
    /// - Every piece referenced by a constraint is in `self.pieces`.
    /// - Every colour referenced by a `CountOnColor` is in
    ///   `self.square_colors`. An empty `square_colors` is valid
    ///   only for problems that have **no** `CountOnColor`
    ///   constraints; otherwise the colour reference is treated as
    ///   unknown.
    /// - Every `At` / `NotAt` square index is `< num_squares`.
    /// - Every `Order` / `Relative` instance index is `< declared
    ///   count` for that piece (only checked when the piece has a
    ///   `Constraint::Count { Eq, n }` constraint declaring its
    ///   count).
    ///
    /// `count()` / `iter()` / `sample()` do **not** auto-validate; if
    /// you need correctness up front, call this first or use
    /// [`ProblemBuilder::try_build`].
    pub fn validate(&self) -> Result<(), ValidationError> {
        if !self.square_colors.is_empty() && self.square_colors.len() != self.num_squares {
            return Err(ValidationError::ColorLengthMismatch {
                expected: self.num_squares,
                actual: self.square_colors.len(),
            });
        }

        let alphabet = self.dedup_alphabet();
        let counts: HashMap<P, usize> = self.constraint.collect_eq_counts().into_iter().collect();
        let check_instance = |p: &P, idx: usize| -> Option<ValidationError> {
            counts
                .get(p)
                .filter(|&&declared| idx >= declared)
                .map(|&declared| ValidationError::InstanceOutOfRange {
                    instance: idx,
                    declared,
                })
        };

        let mut error: Option<ValidationError> = None;
        self.constraint.walk(&mut |c| {
            if error.is_some() {
                return;
            }
            match c {
                Constraint::Count { piece, .. } if !alphabet.contains(piece) => {
                    error = Some(ValidationError::UnknownPiece);
                }
                Constraint::CountOnColor { piece, color, .. } => {
                    if !alphabet.contains(piece) {
                        error = Some(ValidationError::UnknownPiece);
                    } else if !self.square_colors.contains(color) {
                        // Includes the case where `square_colors` is
                        // empty: declaring "no colour partition" and
                        // then referencing a colour is a contradiction.
                        error = Some(ValidationError::UnknownColor);
                    }
                }
                Constraint::At { piece, square } | Constraint::NotAt { piece, square } => {
                    if !alphabet.contains(piece) {
                        error = Some(ValidationError::UnknownPiece);
                    } else if *square >= self.num_squares {
                        error = Some(ValidationError::SquareOutOfRange {
                            square: *square,
                            num_squares: self.num_squares,
                        });
                    }
                }
                Constraint::Order(chain) => {
                    for (p, idx) in chain {
                        if !alphabet.contains(p) {
                            error = Some(ValidationError::UnknownPiece);
                            return;
                        }
                        if let Some(e) = check_instance(p, *idx) {
                            error = Some(e);
                            return;
                        }
                    }
                }
                Constraint::Relative { lhs, rhs, .. }
                    if !alphabet.contains(&lhs.0) || !alphabet.contains(&rhs.0) =>
                {
                    error = Some(ValidationError::UnknownPiece);
                }
                Constraint::Relative { lhs, rhs, .. } => {
                    if let Some(e) = check_instance(&lhs.0, lhs.1) {
                        error = Some(e);
                    } else if let Some(e) = check_instance(&rhs.0, rhs.1) {
                        error = Some(e);
                    }
                }
                _ => {}
            }
        });
        match error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

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

    /// Streams all distinct arrangements satisfying the constraint
    /// in canonical lexicographic order.
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
    /// fixing its count and the values sum to `num_squares`, the
    /// declared counts define a single multiset. The enumerator can
    /// then iterate that multiset's distinct permutations via
    /// next-permutation instead of running the full Cartesian
    /// product. Returns `None` (falls back to Cartesian enumeration)
    /// whenever the fast path can't apply — including when
    /// `Count{Eq}` constraints sit inside `Or` / `Not` (we don't
    /// infer counts across disjunction).
    fn fixed_count_arrangement(&self) -> Option<Vec<P>> {
        let alphabet = self.dedup_alphabet();
        if alphabet.is_empty() {
            return None;
        }

        let counts = self.constraint.collect_eq_counts();
        if counts.is_empty() {
            return None;
        }

        let mut arrangement: Vec<P> = Vec::with_capacity(self.num_squares);
        let mut total = 0usize;
        for kind in &alphabet {
            // First Eq-count wins if the user accidentally specified
            // two conflicting counts; the iterator filter will then
            // catch the contradiction by emitting zero arrangements.
            let count = counts.iter().find(|(p, _)| p == kind).map(|(_, n)| *n)?;
            for _ in 0..count {
                arrangement.push(*kind);
            }
            total += count;
        }

        if total != self.num_squares {
            return None;
        }
        arrangement.sort();
        Some(arrangement)
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

    /// Sets `square_colors` to `(0..num_squares).map(f).collect()`.
    /// The closure receives each 0-based square index and returns
    /// its colour. Call after `.squares(n)`.
    ///
    /// ```
    /// use chess_startpos_rs::{Problem, SquareColor};
    ///
    /// // 6-square board split into halves: first 3 Light, last 3 Dark.
    /// let problem: Problem<u8> = Problem::builder()
    ///     .squares(6)
    ///     .colors_fn(|i| if i < 3 { SquareColor::Light } else { SquareColor::Dark })
    ///     .build();
    /// assert_eq!(problem.square_colors.len(), 6);
    /// ```
    #[must_use]
    pub fn colors_fn<F>(mut self, f: F) -> Self
    where
        F: FnMut(usize) -> C,
    {
        self.square_colors = (0..self.num_squares).map(f).collect();
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

    /// Finalises into a [`Problem`] without validating it. The
    /// accumulated constraints are wrapped in [`Constraint::And`] (a
    /// single constraint is stored bare; zero constraints become
    /// `And(vec![])`, which is always satisfied).
    ///
    /// To check internal consistency before solving, use
    /// [`try_build`](Self::try_build) instead, or call
    /// [`Problem::validate`] on the returned value.
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

    /// Like [`build`](Self::build) but runs [`Problem::validate`] on
    /// the result, returning the validation error instead of the
    /// problem if it fails.
    pub fn try_build(self) -> Result<Problem<P, C>, ValidationError> {
        let problem = self.build();
        problem.validate()?;
        Ok(problem)
    }
}

/// Iterator over satisfying arrangements; see [`Problem::iter`].
struct ProblemIter<'a, P: PieceKind, C: ColorKind> {
    problem: &'a Problem<P, C>,
    state: IterState<P>,
}

enum IterState<P: PieceKind> {
    /// Fully constrained — the declared counts give us a single
    /// multiset; iterate its distinct permutations via the
    /// next-permutation algorithm. Each emitted permutation is then
    /// filtered against the rest of the constraint tree.
    /// `current` is the next candidate to emit (before filtering).
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
        let state = match problem.fixed_count_arrangement() {
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

/// Standard next-permutation algorithm (lexicographic). When the
/// input contains duplicate elements, equal-element swaps never
/// produce a new ordering, so the iteration naturally yields each
/// distinct permutation once. Returns `false` if `v` was already
/// the last permutation.
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
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    fn count_constraint_drives_piece_counts() {
        // Alphabet {A, B}; counts {A: 2, B: 1}; expect 3 perms over
        // the resulting [A, A, B] arrangement.
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
    fn count_constraint_inside_or_does_not_activate_fast_path() {
        // Or-wrapped Count-Eq must NOT be picked up as a piece-count
        // declaration — the solver should fall back to Cartesian
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
    fn validate_accepts_consistent_problem() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B],
            constraint: fixed_counts(&[(Tile::A, 1), (Tile::B, 2)]),
        };
        assert!(problem.validate().is_ok());
    }

    #[test]
    fn validate_accepts_empty_colors() {
        // Empty `square_colors` is "no colour partition declared".
        let problem: Problem<Tile> = Problem {
            num_squares: 3,
            square_colors: vec![],
            pieces: vec![Tile::A, Tile::B],
            constraint: Constraint::And(vec![]),
        };
        assert!(problem.validate().is_ok());
    }

    #[test]
    fn validate_rejects_mismatched_color_length() {
        let problem: Problem<Tile> = Problem {
            num_squares: 3,
            square_colors: vec![SquareColor::Light], // only 1 of 3
            pieces: vec![Tile::A],
            constraint: Constraint::And(vec![]),
        };
        assert_eq!(
            problem.validate(),
            Err(ValidationError::ColorLengthMismatch {
                expected: 3,
                actual: 1,
            }),
        );
    }

    #[test]
    fn validate_rejects_unknown_piece() {
        let problem: Problem<Tile> = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A], // Tile::B not declared
            constraint: Constraint::At {
                piece: Tile::B,
                square: 0,
            },
        };
        assert_eq!(problem.validate(), Err(ValidationError::UnknownPiece));
    }

    #[test]
    fn validate_rejects_unknown_color() {
        // square_colors only has Light; constraint references Dark.
        let problem: Problem<Tile> = Problem {
            num_squares: 2,
            square_colors: vec![SquareColor::Light, SquareColor::Light],
            pieces: vec![Tile::A],
            constraint: Constraint::CountOnColor {
                piece: Tile::A,
                color: SquareColor::Dark,
                op: CountOp::Eq,
                value: 0,
            },
        };
        assert_eq!(problem.validate(), Err(ValidationError::UnknownColor));
    }

    #[test]
    fn validate_rejects_instance_out_of_range_in_order() {
        // Declare exactly 2 Bs but reference (B, 2).
        let problem: Problem<Tile> = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B],
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
                Constraint::Order(vec![(Tile::B, 0), (Tile::B, 1), (Tile::B, 2)]),
            ]),
        };
        assert_eq!(
            problem.validate(),
            Err(ValidationError::InstanceOutOfRange {
                instance: 2,
                declared: 2,
            }),
        );
    }

    #[test]
    fn validate_rejects_instance_out_of_range_in_relative() {
        let problem: Problem<Tile> = Problem {
            num_squares: 2,
            square_colors: light_dark(2),
            pieces: vec![Tile::A, Tile::B],
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
                Constraint::Relative {
                    lhs: (Tile::A, 1), // only 1 A declared
                    rhs: (Tile::B, 0),
                    op: CountOp::Eq,
                    offset: 0,
                },
            ]),
        };
        assert_eq!(
            problem.validate(),
            Err(ValidationError::InstanceOutOfRange {
                instance: 1,
                declared: 1,
            }),
        );
    }

    #[test]
    fn validate_rejects_count_on_color_when_no_colors_declared() {
        // Empty square_colors + a CountOnColor → contradiction.
        let problem: Problem<Tile> = Problem {
            num_squares: 2,
            square_colors: vec![],
            pieces: vec![Tile::A],
            constraint: Constraint::CountOnColor {
                piece: Tile::A,
                color: SquareColor::Light,
                op: CountOp::Eq,
                value: 1,
            },
        };
        assert_eq!(problem.validate(), Err(ValidationError::UnknownColor));
    }

    #[test]
    fn validate_rejects_square_out_of_range() {
        let problem: Problem<Tile> = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A],
            constraint: Constraint::At {
                piece: Tile::A,
                square: 5, // out of range
            },
        };
        assert_eq!(
            problem.validate(),
            Err(ValidationError::SquareOutOfRange {
                square: 5,
                num_squares: 3,
            }),
        );
    }

    #[test]
    fn validate_walks_into_combinators() {
        // UnknownPiece inside a nested Or should still be caught.
        let problem: Problem<Tile> = Problem {
            num_squares: 2,
            square_colors: light_dark(2),
            pieces: vec![Tile::A],
            constraint: Constraint::Or(vec![
                Constraint::At {
                    piece: Tile::A,
                    square: 0,
                },
                Constraint::Not(Box::new(Constraint::At {
                    piece: Tile::B, // unknown
                    square: 1,
                })),
            ]),
        };
        assert_eq!(problem.validate(), Err(ValidationError::UnknownPiece));
    }

    #[test]
    fn try_build_returns_error_for_invalid_problem() {
        let result: Result<Problem<Tile>, _> = Problem::builder()
            .squares(3)
            .colors(vec![SquareColor::Light]) // mismatched
            .pieces([Tile::A])
            .try_build();
        assert!(matches!(
            result,
            Err(ValidationError::ColorLengthMismatch { .. })
        ));
    }

    #[test]
    fn builder_colors_fn_assigns_per_index() {
        let problem: Problem<Tile> = Problem::builder()
            .squares(6)
            .colors_fn(|i| {
                if i < 3 {
                    SquareColor::Light
                } else {
                    SquareColor::Dark
                }
            })
            .pieces([Tile::A])
            .build();
        assert_eq!(problem.square_colors.len(), 6);
        assert_eq!(problem.square_colors[0], SquareColor::Light);
        assert_eq!(problem.square_colors[5], SquareColor::Dark);
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

    #[cfg(feature = "serde")]
    #[test]
    fn problem_serde_roundtrip() {
        let problem = Problem {
            num_squares: 3,
            square_colors: light_dark(3),
            pieces: vec![Tile::A, Tile::B],
            constraint: fixed_counts(&[(Tile::A, 1), (Tile::B, 2)]),
        };
        let json = serde_json::to_string(&problem).expect("serialise");
        let back: Problem<Tile> = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back.num_squares, problem.num_squares);
        assert_eq!(back.pieces, problem.pieces);
        assert_eq!(back.count(), problem.count());
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
