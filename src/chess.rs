//! Opinionated chess preset module — batteries included for the
//! canonical 8-square back-rank arrangements.
//!
//! Callers who don't want to define their own piece kind, board, or
//! constraint set can use one of the four named presets directly.

use std::fmt;

use crate::{alternating, Constraint, CountOp, Problem, SquareColor};

/// The five standard back-rank chess piece kinds.
///
/// `Pawn` is intentionally absent — pawns never appear on the back
/// rank, so they have no role in this crate's combinatorial problem.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Piece {
    /// King.
    King,
    /// Queen.
    Queen,
    /// Rook.
    Rook,
    /// Bishop.
    Bishop,
    /// Knight.
    Knight,
}

impl fmt::Display for Piece {
    /// Single-letter algebraic notation, white-side conventional
    /// uppercase: `K`, `Q`, `R`, `B`, `N`.
    ///
    /// ```
    /// use chess_startpos_rs::chess::Piece;
    ///
    /// assert_eq!(Piece::King.to_string(),   "K");
    /// assert_eq!(Piece::Queen.to_string(),  "Q");
    /// assert_eq!(Piece::Rook.to_string(),   "R");
    /// assert_eq!(Piece::Bishop.to_string(), "B");
    /// assert_eq!(Piece::Knight.to_string(), "N");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::King => "K",
            Self::Queen => "Q",
            Self::Rook => "R",
            Self::Bishop => "B",
            Self::Knight => "N",
        })
    }
}

/// File-letter constants and the char-to-index helper.
///
/// Use the constants to keep `Constraint::At` / `NotAt` square arguments
/// self-documenting:
///
/// ```
/// use chess_startpos_rs::{chess, Constraint};
///
/// let _ = Constraint::<chess::Piece>::At {
///     piece: chess::Piece::Queen,
///     square: chess::file::D,
/// };
/// ```
pub mod file {
    /// File index of the `a` file (square 0 on the back rank).
    pub const A: usize = 0;
    /// File index of the `b` file.
    pub const B: usize = 1;
    /// File index of the `c` file.
    pub const C: usize = 2;
    /// File index of the `d` file.
    pub const D: usize = 3;
    /// File index of the `e` file.
    pub const E: usize = 4;
    /// File index of the `f` file.
    pub const F: usize = 5;
    /// File index of the `g` file.
    pub const G: usize = 6;
    /// File index of the `h` file (square 7 on the back rank).
    pub const H: usize = 7;

    /// Converts a file letter to its 0-based square index on the back rank.
    ///
    /// Accepts both lowercase (`'a'..='h'`) and uppercase (`'A'..='H'`) and
    /// returns `None` for any other character.
    ///
    /// ```
    /// use chess_startpos_rs::chess;
    ///
    /// assert_eq!(chess::file::of('a'), Some(0));
    /// assert_eq!(chess::file::of('H'), Some(7));
    /// assert_eq!(chess::file::of('1'), None);
    /// ```
    #[must_use]
    pub fn of(letter: char) -> Option<usize> {
        match letter {
            'a'..='h' => Some(letter as usize - 'a' as usize),
            'A'..='H' => Some(letter as usize - 'A' as usize),
            _ => None,
        }
    }
}

/// The FIDE standard back-rank arrangement, in a1..h1 order.
pub const STANDARD_BACK_RANK: [Piece; 8] = [
    Piece::Rook,
    Piece::Knight,
    Piece::Bishop,
    Piece::Queen,
    Piece::King,
    Piece::Bishop,
    Piece::Knight,
    Piece::Rook,
];

/// Returns the standard 8-square back-rank colour layout, with
/// `a1 = Dark` and squares alternating.
#[must_use]
pub fn back_rank_colors() -> Vec<SquareColor> {
    alternating(8, SquareColor::Dark, SquareColor::Light)
}

/// The alphabet of back-rank piece kinds.
fn alphabet() -> Vec<Piece> {
    vec![
        Piece::King,
        Piece::Queen,
        Piece::Rook,
        Piece::Bishop,
        Piece::Knight,
    ]
}

/// The five `Count {kind, Eq, n}` constraints that fix the back-rank
/// counts to KQRRBBNN (1 K, 1 Q, 2 R, 2 B, 2 N).
fn back_rank_counts() -> Vec<Constraint<Piece>> {
    vec![
        Constraint::Count {
            piece: Piece::King,
            op: CountOp::Eq,
            value: 1,
        },
        Constraint::Count {
            piece: Piece::Queen,
            op: CountOp::Eq,
            value: 1,
        },
        Constraint::Count {
            piece: Piece::Rook,
            op: CountOp::Eq,
            value: 2,
        },
        Constraint::Count {
            piece: Piece::Bishop,
            op: CountOp::Eq,
            value: 2,
        },
        Constraint::Count {
            piece: Piece::Knight,
            op: CountOp::Eq,
            value: 2,
        },
    ]
}

/// Preset: only the FIDE standard starting back rank. `count() == 1`.
#[must_use]
pub fn standard() -> Problem<Piece> {
    let mut constraints = back_rank_counts();
    for (i, p) in STANDARD_BACK_RANK.iter().enumerate() {
        constraints.push(Constraint::At {
            piece: *p,
            square: i,
        });
    }
    Problem {
        num_squares: 8,
        square_colors: back_rank_colors(),
        pieces: alphabet(),
        constraint: Constraint::And(constraints),
    }
}

/// Preset: any arrangement of KQRRBBNN. `count() == 5040`.
#[must_use]
pub fn shuffle() -> Problem<Piece> {
    Problem {
        num_squares: 8,
        square_colors: back_rank_colors(),
        pieces: alphabet(),
        constraint: Constraint::And(back_rank_counts()),
    }
}

/// Preset: KQRRBBNN with bishops on opposite-colour squares.
/// `count() == 2880`.
#[must_use]
pub fn chess_2880() -> Problem<Piece> {
    let mut constraints = back_rank_counts();
    constraints.push(Constraint::CountOnColor {
        piece: Piece::Bishop,
        color: SquareColor::Light,
        op: CountOp::Eq,
        value: 1,
    });
    constraints.push(Constraint::CountOnColor {
        piece: Piece::Bishop,
        color: SquareColor::Dark,
        op: CountOp::Eq,
        value: 1,
    });
    Problem {
        num_squares: 8,
        square_colors: back_rank_colors(),
        pieces: alphabet(),
        constraint: Constraint::And(constraints),
    }
}

/// Preset: KQRRBBNN with bishops on opposite-colour squares plus king
/// strictly between the two rooks. `count() == 960`. Equivalent to the
/// Chess960 (Fischer Random) starting-position set.
///
/// Returns a [`Chess960Problem`] wrapper that exposes both lexicographic
/// indexing ([`Chess960Problem::at`]) and the canonical Chess960 SP-ID
/// bijection ([`Chess960Problem::sp_id`] / [`Chess960Problem::sp_id_of`]).
#[must_use]
pub fn chess_960() -> Chess960Problem {
    let mut constraints = back_rank_counts();
    constraints.push(Constraint::CountOnColor {
        piece: Piece::Bishop,
        color: SquareColor::Light,
        op: CountOp::Eq,
        value: 1,
    });
    constraints.push(Constraint::CountOnColor {
        piece: Piece::Bishop,
        color: SquareColor::Dark,
        op: CountOp::Eq,
        value: 1,
    });
    constraints.push(Constraint::Order(vec![
        (Piece::Rook, 0),
        (Piece::King, 0),
        (Piece::Rook, 1),
    ]));
    Chess960Problem {
        inner: Problem {
            num_squares: 8,
            square_colors: back_rank_colors(),
            pieces: alphabet(),
            constraint: Constraint::And(constraints),
        },
    }
}

/// The Chess960 preset, returned by [`chess_960`].
///
/// Wraps a [`Problem<Piece>`] and adds the canonical Chess960 SP-ID
/// bijection on top of the generic constraint-satisfaction surface.
///
/// The generic methods ([`at`](Self::at), [`iter`](Self::iter),
/// [`sample`](Self::sample), [`count`](Self::count)) operate in
/// **lexicographic** order over the declared piece counts, matching
/// the rest of the crate. The Chess960-specific methods
/// ([`sp_id`](Self::sp_id), [`sp_id_of`](Self::sp_id_of)) use the
/// official FIDE numbering, interoperating with Stockfish, Lichess,
/// python-chess, and other chess software.
///
/// The standard FIDE starting position is `sp_id(518)`.
///
/// The official encoding `((KN · 6 + Q) · 4 + DB) · 4 + LB` derives
/// each Chess960 starting position from four sub-indices:
///
/// - `LB` (0–3) — index of the light-square bishop among files b/d/f/h.
/// - `DB` (0–3) — index of the dark-square bishop among files a/c/e/g.
/// - `Q` (0–5) — index of the queen among the six non-bishop squares.
/// - `KN` (0–9) — index of the knight pair among the 10 unordered
///   placements on the five non-bishop, non-queen squares.
///
/// The remaining three squares are filled rook–king–rook from left to
/// right, which automatically satisfies the king-between-rooks rule.
#[derive(Clone, Debug)]
pub struct Chess960Problem {
    inner: Problem<Piece>,
}

const KNIGHT_PAIRS: [(usize, usize); 10] = [
    (0, 1),
    (0, 2),
    (0, 3),
    (0, 4),
    (1, 2),
    (1, 3),
    (1, 4),
    (2, 3),
    (2, 4),
    (3, 4),
];

const LIGHT_FILES: [usize; 4] = [1, 3, 5, 7];
const DARK_FILES: [usize; 4] = [0, 2, 4, 6];

impl Chess960Problem {
    /// Number of canonical Chess960 starting positions (always 960).
    pub const COUNT: u32 = 960;

    /// Number of arrangements satisfying the constraint. Always 960.
    #[must_use]
    pub fn count(&self) -> u64 {
        self.inner.count()
    }

    /// Streams all 960 arrangements in canonical lexicographic order.
    pub fn iter(&self) -> impl Iterator<Item = Vec<Piece>> + '_ {
        self.inner.iter()
    }

    /// Returns the lexicographic-`index`-th arrangement, or `None` if
    /// `index >= 960`.
    ///
    /// For canonical SP-ID-keyed access use [`Self::sp_id`] instead;
    /// `at(N)` and `sp_id(N)` do **not** return the same position for
    /// any given `N`.
    #[must_use]
    pub fn at(&self, index: u64) -> Option<Vec<Piece>> {
        self.inner.at(index)
    }

    /// Returns a uniformly-random arrangement, deterministic in `seed`.
    ///
    /// Infallible — the Chess960 preset is statically non-empty.
    #[must_use]
    pub fn sample(&self, seed: u64) -> Vec<Piece> {
        self.inner.sample(seed).expect("chess_960() is non-empty")
    }

    /// Returns the back-rank arrangement at the given canonical
    /// Chess960 SP-ID, or `None` if `id >= 960`.
    ///
    /// ```
    /// use chess_startpos_rs::chess::{self, STANDARD_BACK_RANK};
    ///
    /// assert_eq!(chess::chess_960().sp_id(518), Some(STANDARD_BACK_RANK.to_vec()));
    /// assert_eq!(chess::chess_960().sp_id(960), None);
    /// ```
    #[must_use]
    pub fn sp_id(&self, id: u32) -> Option<Vec<Piece>> {
        if id >= Self::COUNT {
            return None;
        }
        let mut n = id;
        let lb = (n % 4) as usize;
        n /= 4;
        let db = (n % 4) as usize;
        n /= 4;
        let q_idx = (n % 6) as usize;
        n /= 6;
        let kn_idx = n as usize;

        let mut board: [Option<Piece>; 8] = [None; 8];
        board[LIGHT_FILES[lb]] = Some(Piece::Bishop);
        board[DARK_FILES[db]] = Some(Piece::Bishop);

        let empty_after_bishops: Vec<usize> = (0..8).filter(|i| board[*i].is_none()).collect();
        board[empty_after_bishops[q_idx]] = Some(Piece::Queen);

        let empty_after_queen: Vec<usize> = (0..8).filter(|i| board[*i].is_none()).collect();
        let (kn_a, kn_b) = KNIGHT_PAIRS[kn_idx];
        board[empty_after_queen[kn_a]] = Some(Piece::Knight);
        board[empty_after_queen[kn_b]] = Some(Piece::Knight);

        let empty_last: Vec<usize> = (0..8).filter(|i| board[*i].is_none()).collect();
        board[empty_last[0]] = Some(Piece::Rook);
        board[empty_last[1]] = Some(Piece::King);
        board[empty_last[2]] = Some(Piece::Rook);

        Some(board.iter().map(|p| p.expect("filled")).collect())
    }

    /// Returns the canonical SP-ID for an 8-square back-rank arrangement,
    /// or `None` if the arrangement is not a valid Chess960 starting
    /// position (wrong piece counts, bishops on same colour, king not
    /// strictly between the rooks, ...).
    ///
    /// `sp_id` and `sp_id_of` are inverses:
    /// `chess_960().sp_id_of(&chess_960().sp_id(id).unwrap()) == Some(id)`
    /// for all `id < 960`.
    ///
    /// ```
    /// use chess_startpos_rs::chess::{self, STANDARD_BACK_RANK};
    ///
    /// assert_eq!(chess::chess_960().sp_id_of(&STANDARD_BACK_RANK), Some(518));
    /// ```
    #[must_use]
    pub fn sp_id_of(&self, arrangement: &[Piece]) -> Option<u32> {
        if arrangement.len() != 8 {
            return None;
        }

        let bishops: Vec<usize> = arrangement
            .iter()
            .enumerate()
            .filter_map(|(i, p)| (*p == Piece::Bishop).then_some(i))
            .collect();
        if bishops.len() != 2 {
            return None;
        }
        let (light_sq, dark_sq) = if bishops[0] % 2 == 1 {
            (bishops[0], bishops[1])
        } else {
            (bishops[1], bishops[0])
        };
        if light_sq % 2 != 1 || dark_sq % 2 != 0 {
            return None;
        }
        let lb = LIGHT_FILES.iter().position(|&x| x == light_sq)?;
        let db = DARK_FILES.iter().position(|&x| x == dark_sq)?;

        let queen = arrangement.iter().position(|p| *p == Piece::Queen)?;
        if arrangement.iter().filter(|p| **p == Piece::Queen).count() != 1 {
            return None;
        }
        let empty_after_bishops: Vec<usize> = (0..8)
            .filter(|i| arrangement[*i] != Piece::Bishop)
            .collect();
        let q_idx = empty_after_bishops.iter().position(|&x| x == queen)?;

        let knight_positions: Vec<usize> = arrangement
            .iter()
            .enumerate()
            .filter_map(|(i, p)| (*p == Piece::Knight).then_some(i))
            .collect();
        if knight_positions.len() != 2 {
            return None;
        }
        let empty_after_queen: Vec<usize> = (0..8)
            .filter(|i| arrangement[*i] != Piece::Bishop && arrangement[*i] != Piece::Queen)
            .collect();
        let kn_a = empty_after_queen
            .iter()
            .position(|&x| x == knight_positions[0])?;
        let kn_b = empty_after_queen
            .iter()
            .position(|&x| x == knight_positions[1])?;
        let kn_idx = KNIGHT_PAIRS.iter().position(|&p| p == (kn_a, kn_b))?;

        let last_three: Vec<usize> = (0..8)
            .filter(|i| {
                arrangement[*i] != Piece::Bishop
                    && arrangement[*i] != Piece::Queen
                    && arrangement[*i] != Piece::Knight
            })
            .collect();
        if last_three.len() != 3 {
            return None;
        }
        if arrangement[last_three[0]] != Piece::Rook
            || arrangement[last_three[1]] != Piece::King
            || arrangement[last_three[2]] != Piece::Rook
        {
            return None;
        }

        let id = ((kn_idx as u32 * 6 + q_idx as u32) * 4 + db as u32) * 4 + lb as u32;
        Some(id)
    }

    /// Returns the underlying generic [`Problem<Piece>`] with `c`
    /// added via AND-composition. The Chess960 SP-ID bijection no
    /// longer applies to the narrowed problem, so the result is the
    /// generic type, not another `Chess960Problem`.
    #[must_use]
    pub fn with_constraint(&self, c: Constraint<Piece>) -> Problem<Piece> {
        self.inner.with_constraint(c)
    }

    /// Consumes self and returns the underlying generic problem.
    #[must_use]
    pub fn into_problem(self) -> Problem<Piece> {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_count_is_one() {
        assert_eq!(standard().count(), 1);
    }

    #[test]
    fn shuffle_count_is_5040() {
        assert_eq!(shuffle().count(), 5040);
    }

    #[test]
    fn chess_2880_count_is_2880() {
        assert_eq!(chess_2880().count(), 2880);
    }

    #[test]
    fn chess_960_count_is_960() {
        assert_eq!(chess_960().count(), 960);
    }

    #[test]
    fn standard_arrangement_matches_fide() {
        let arrangements: Vec<Vec<Piece>> = standard().iter().collect();
        assert_eq!(arrangements.len(), 1);
        assert_eq!(arrangements[0], STANDARD_BACK_RANK.to_vec());
    }

    #[test]
    fn chess_960_minus_king_constraint_equals_chess_2880() {
        assert_eq!(chess_960().count() * 3, chess_2880().count());
    }

    #[test]
    fn with_constraint_narrows_chess_960() {
        let narrowed = chess_960().with_constraint(Constraint::At {
            piece: Piece::Queen,
            square: 3,
        });
        assert!(narrowed.count() < 960);
        assert!(narrowed.count() > 0);
    }

    #[test]
    fn file_constants_match_alphabet() {
        use file::*;
        assert_eq!([A, B, C, D, E, F, G, H], [0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn file_letter_to_index() {
        for (i, ch) in ('a'..='h').enumerate() {
            assert_eq!(file::of(ch), Some(i));
            assert_eq!(file::of(ch.to_ascii_uppercase()), Some(i));
        }
        assert_eq!(file::of('i'), None);
        assert_eq!(file::of('1'), None);
        assert_eq!(file::of(' '), None);
    }

    #[test]
    fn file_constants_usable_in_at_constraint() {
        let with_queen_on_d = chess_960().with_constraint(Constraint::At {
            piece: Piece::Queen,
            square: file::D,
        });
        assert!(with_queen_on_d.count() > 0);
        assert!(with_queen_on_d.count() < 960);
    }

    #[test]
    fn sp_id_518_is_standard_position() {
        let preset = chess_960();
        assert_eq!(preset.sp_id(518), Some(STANDARD_BACK_RANK.to_vec()));
        assert_eq!(preset.sp_id_of(&STANDARD_BACK_RANK), Some(518));
    }

    #[test]
    fn sp_id_out_of_range_returns_none() {
        let preset = chess_960();
        assert_eq!(preset.sp_id(960), None);
        assert_eq!(preset.sp_id(u32::MAX), None);
    }

    #[test]
    fn sp_id_roundtrip_over_full_range() {
        let preset = chess_960();
        for id in 0..Chess960Problem::COUNT {
            let arrangement = preset.sp_id(id).expect("in range");
            assert_eq!(arrangement.len(), 8);
            assert_eq!(preset.sp_id_of(&arrangement), Some(id), "round-trip {id}");
        }
    }

    #[test]
    fn sample_is_deterministic_in_seed() {
        let preset = chess_960();
        let a = preset.sample(0xC0FFEE);
        let b = preset.sample(0xC0FFEE);
        assert_eq!(a, b);
        assert!(preset.sp_id_of(&a).is_some());
    }

    #[test]
    fn sp_id_of_rejects_invalid_arrangements() {
        let preset = chess_960();

        // Wrong length.
        assert_eq!(preset.sp_id_of(&[Piece::King; 4]), None);

        // Same-colour bishops (both on light squares b1 and f1).
        let mut bad = STANDARD_BACK_RANK.to_vec();
        bad[1] = Piece::Bishop;
        bad[2] = Piece::Knight;
        assert_eq!(preset.sp_id_of(&bad), None);

        // King not strictly between rooks.
        let king_outside = vec![
            Piece::King,
            Piece::Rook,
            Piece::Rook,
            Piece::Bishop,
            Piece::Knight,
            Piece::Knight,
            Piece::Bishop,
            Piece::Queen,
        ];
        assert_eq!(preset.sp_id_of(&king_outside), None);

        // Wrong piece counts (two queens, no king).
        let two_queens = vec![
            Piece::Rook,
            Piece::Knight,
            Piece::Bishop,
            Piece::Queen,
            Piece::Queen,
            Piece::Bishop,
            Piece::Knight,
            Piece::Rook,
        ];
        assert_eq!(preset.sp_id_of(&two_queens), None);
    }
}
