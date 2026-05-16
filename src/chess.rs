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
}
