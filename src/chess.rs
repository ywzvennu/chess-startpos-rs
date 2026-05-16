//! Opinionated chess preset module — batteries included for the
//! canonical 8-square back-rank arrangements.
//!
//! Callers who don't want to define their own piece kind, board, or
//! constraint set can use one of the four named presets directly.

use crate::{Constraint, CountOp, Problem, SquareColor};

/// The six standard chess piece kinds.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
    /// Pawn (not used in back-rank arrangements, included for API
    /// completeness).
    Pawn,
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

/// The standard back-rank piece multiset (KQRRBBNN), in a1..h1 order
/// matching the FIDE starting position.
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

/// Returns the 8-square back-rank board: `(num_squares, square_colors)`.
///
/// Square 0 is `a1`, which is a dark square in standard chess. Colours
/// alternate dark / light / dark / ... so the diagonal a1–h8 is dark.
#[must_use]
pub fn back_rank_board() -> (usize, Vec<SquareColor>) {
    let colors: Vec<SquareColor> = (0..8)
        .map(|i| {
            if i % 2 == 0 {
                SquareColor::Dark
            } else {
                SquareColor::Light
            }
        })
        .collect();
    (8, colors)
}

fn back_rank_multiset() -> Vec<Piece> {
    vec![
        Piece::King,
        Piece::Queen,
        Piece::Rook,
        Piece::Rook,
        Piece::Bishop,
        Piece::Bishop,
        Piece::Knight,
        Piece::Knight,
    ]
}

/// Preset: only the FIDE standard starting back rank. `count() == 1`.
#[must_use]
pub fn standard() -> Problem<Piece> {
    let (num_squares, square_colors) = back_rank_board();
    // The constraint pins every square to the FIDE arrangement.
    let constraint = Constraint::And(
        STANDARD_BACK_RANK
            .iter()
            .enumerate()
            .map(|(i, p)| Constraint::At {
                piece: *p,
                square: i,
            })
            .collect(),
    );
    Problem {
        num_squares,
        square_colors,
        pieces: back_rank_multiset(),
        constraint,
    }
}

/// Preset: any arrangement of the standard back-rank multiset, no
/// extra constraints. `count() == 5040`.
#[must_use]
pub fn shuffle() -> Problem<Piece> {
    let (num_squares, square_colors) = back_rank_board();
    Problem {
        num_squares,
        square_colors,
        pieces: back_rank_multiset(),
        constraint: Constraint::And(vec![]),
    }
}

/// Preset: bishops on opposite-colour squares. `count() == 2880`.
#[must_use]
pub fn chess_2880() -> Problem<Piece> {
    let (num_squares, square_colors) = back_rank_board();
    let constraint = Constraint::And(vec![
        Constraint::CountOnColor {
            piece: Piece::Bishop,
            color: SquareColor::Light,
            op: CountOp::Eq,
            value: 1,
        },
        Constraint::CountOnColor {
            piece: Piece::Bishop,
            color: SquareColor::Dark,
            op: CountOp::Eq,
            value: 1,
        },
    ]);
    Problem {
        num_squares,
        square_colors,
        pieces: back_rank_multiset(),
        constraint,
    }
}

/// Preset: bishops on opposite-colour squares plus king strictly
/// between the two rooks. `count() == 960`. Equivalent to the
/// Chess960 (Fischer Random) starting-position set.
#[must_use]
pub fn chess_960() -> Problem<Piece> {
    let (num_squares, square_colors) = back_rank_board();
    let constraint = Constraint::And(vec![
        Constraint::CountOnColor {
            piece: Piece::Bishop,
            color: SquareColor::Light,
            op: CountOp::Eq,
            value: 1,
        },
        Constraint::CountOnColor {
            piece: Piece::Bishop,
            color: SquareColor::Dark,
            op: CountOp::Eq,
            value: 1,
        },
        Constraint::Order(vec![(Piece::Rook, 0), (Piece::King, 0), (Piece::Rook, 1)]),
    ]);
    Problem {
        num_squares,
        square_colors,
        pieces: back_rank_multiset(),
        constraint,
    }
}

/// Canonical Chess960 starting-position identifier (SP-ID).
///
/// The official FIDE encoding maps each of the 960 legal Chess960
/// arrangements to an integer `0..=959`. The standard FIDE starting
/// position is SP-ID `518`. The encoding is `((KN · 6 + Q) · 4 + DB) · 4 + LB`
/// where:
///
/// - `LB` (0–3) — index of the light-square bishop among files b/d/f/h.
/// - `DB` (0–3) — index of the dark-square bishop among files a/c/e/g.
/// - `Q` (0–5) — index of the queen among the six non-bishop squares.
/// - `KN` (0–9) — index of the knight pair among the 10 unordered
///   placements on the five non-bishop, non-queen squares.
///
/// The remaining three squares are filled rook–king–rook from left to
/// right, which automatically satisfies the king-between-rooks rule.
///
/// This module honours the canonical encoding, so SP-IDs round-trip
/// with other chess software (Stockfish, Lichess, python-chess, ...).
///
/// Note: [`chess_960`]'s `at(N)` returns the arrangement at lexicographic
/// index `N`, not at SP-ID `N`. Use [`sp_id::at`] to look up by SP-ID.
pub mod sp_id {
    use super::Piece;

    /// Number of canonical Chess960 starting positions.
    pub const COUNT: u32 = 960;

    /// Knight-pair lookup: index `0..=9` to the 0-based positions of the
    /// two knights among the five non-bishop, non-queen squares.
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

    /// Returns the back-rank arrangement at the given canonical SP-ID,
    /// or `None` if `id >= 960`.
    ///
    /// ```
    /// use chess_startpos_rs::chess::{self, sp_id, Piece, STANDARD_BACK_RANK};
    ///
    /// assert_eq!(sp_id::at(518), Some(STANDARD_BACK_RANK.to_vec()));
    /// assert_eq!(sp_id::at(960), None);
    /// # let _ = chess::Piece::King;
    /// ```
    #[must_use]
    pub fn at(id: u32) -> Option<Vec<Piece>> {
        if id >= COUNT {
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
    /// position (wrong piece multiset, bishops on same colour, king not
    /// strictly between the rooks, ...).
    ///
    /// `at` and `of` are inverses: `of(&at(id).unwrap()) == Some(id)` for
    /// all `id < 960`.
    ///
    /// ```
    /// use chess_startpos_rs::chess::{sp_id, STANDARD_BACK_RANK};
    ///
    /// assert_eq!(sp_id::of(&STANDARD_BACK_RANK), Some(518));
    /// ```
    #[must_use]
    pub fn of(arrangement: &[Piece]) -> Option<u32> {
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
        // Sanity check the combinator semantics: chess_960's count
        // divided by the king-between-rooks factor (1 in 3) equals
        // chess_2880's count.
        assert_eq!(chess_960().count() * 3, chess_2880().count());
    }

    #[test]
    fn with_constraint_narrows_chess_960() {
        // Force the queen onto file 3 — narrows the population.
        let narrowed = chess_960().with_constraint(Constraint::At {
            piece: Piece::Queen,
            square: 3,
        });
        assert!(narrowed.count() < chess_960().count());
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
        // Same narrowing as with_constraint_narrows_chess_960 but using
        // the file constant for readability.
        let with_queen_on_d = chess_960().with_constraint(Constraint::At {
            piece: Piece::Queen,
            square: file::D,
        });
        assert!(with_queen_on_d.count() > 0);
        assert!(with_queen_on_d.count() < chess_960().count());
    }

    #[test]
    fn sp_id_518_is_standard_position() {
        assert_eq!(sp_id::at(518), Some(STANDARD_BACK_RANK.to_vec()));
        assert_eq!(sp_id::of(&STANDARD_BACK_RANK), Some(518));
    }

    #[test]
    fn sp_id_at_out_of_range_returns_none() {
        assert_eq!(sp_id::at(960), None);
        assert_eq!(sp_id::at(u32::MAX), None);
    }

    #[test]
    fn sp_id_roundtrip_over_full_range() {
        for id in 0..sp_id::COUNT {
            let arrangement = sp_id::at(id).expect("in range");
            assert_eq!(arrangement.len(), 8);
            assert_eq!(sp_id::of(&arrangement), Some(id), "round-trip {id}");
        }
    }

    #[test]
    fn sp_id_at_yields_valid_chess960_arrangements() {
        // Every SP-ID position must pass the chess_960 constraints
        // (bishops opposite colours, king strictly between rooks).
        let problem = chess_960();
        for id in 0..sp_id::COUNT {
            let arrangement = sp_id::at(id).expect("in range");
            assert!(
                problem
                    .constraint
                    .evaluate(&arrangement, &problem.square_colors),
                "SP-ID {id} violates chess_960 constraints: {arrangement:?}"
            );
        }
    }

    #[test]
    fn sp_id_of_rejects_invalid_arrangements() {
        // Wrong length.
        assert_eq!(sp_id::of(&[Piece::King; 4]), None);

        // Same-colour bishops (both on light squares b1 and d1).
        let mut bad = STANDARD_BACK_RANK.to_vec();
        bad[1] = Piece::Bishop; // b1 = light
        bad[2] = Piece::Knight; // c1 was bishop
        assert_eq!(sp_id::of(&bad), None);

        // King not strictly between rooks (king on left of both rooks).
        // KRRBNN BQ with bishops on light file 3 and dark file 6,
        // but king at file 0 is outside the rook pair on (1, 2).
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
        assert_eq!(sp_id::of(&king_outside), None);

        // Wrong multiset (two queens, no king).
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
        assert_eq!(sp_id::of(&two_queens), None);
    }
}
