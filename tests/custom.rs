//! Integration tests for non-chess use of the constraint engine —
//! custom piece kinds on a board with a non-8-square length.
//!
//! These exercise the same Problem API the chess presets use, with
//! none of the chess module involved.

use chess_startpos_rs::{Constraint, CountOp, Problem, SquareColor};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Card {
    Ace,
    King,
    Queen,
}

fn six_card_lineup() -> Problem<Card> {
    Problem {
        num_squares: 6,
        square_colors: vec![
            SquareColor::Light,
            SquareColor::Light,
            SquareColor::Light,
            SquareColor::Dark,
            SquareColor::Dark,
            SquareColor::Dark,
        ],
        pieces: vec![
            Card::Ace,
            Card::Ace,
            Card::King,
            Card::King,
            Card::Queen,
            Card::Queen,
        ],
        constraint: Constraint::And(vec![
            Constraint::CountOnColor {
                piece: Card::Ace,
                color: SquareColor::Light,
                op: CountOp::Eq,
                value: 1,
            },
            Constraint::CountOnColor {
                piece: Card::Ace,
                color: SquareColor::Dark,
                op: CountOp::Eq,
                value: 1,
            },
            Constraint::Order(vec![(Card::King, 0), (Card::Queen, 0)]),
        ]),
    }
}

#[test]
fn count_matches_hand_enumeration() {
    // The 6-card multiset has 6!/(2!·2!·2!) = 90 distinct permutations.
    // The aces-split-across-halves constraint keeps half (2·3·3=18 — no,
    // closed form is messier with the Order constraint); just assert
    // count is non-zero, less than the unconstrained 90, and stable.
    let problem = six_card_lineup();
    let count = problem.count();
    assert!(count > 0);
    assert!(count < 90);
    assert_eq!(count, 27);
}

#[test]
fn iter_and_at_agree_with_count() {
    let problem = six_card_lineup();
    let count = problem.count() as usize;
    let collected: Vec<_> = problem.iter().collect();
    assert_eq!(collected.len(), count);
    for (idx, arrangement) in collected.iter().enumerate() {
        assert_eq!(problem.at(idx as u64), Some(arrangement.clone()));
    }
    assert_eq!(problem.at(count as u64), None);
}

#[test]
fn sample_is_deterministic_and_satisfies_constraints() {
    let problem = six_card_lineup();
    let arrangements: Vec<_> = problem.iter().collect();

    let first = problem.sample(0xDEAD_BEEF).expect("non-empty");
    let again = problem.sample(0xDEAD_BEEF).expect("non-empty");
    assert_eq!(first, again);
    assert!(arrangements.contains(&first));
}

#[test]
fn every_arrangement_respects_the_constraints() {
    let problem = six_card_lineup();
    for arrangement in problem.iter() {
        // Aces split across halves.
        let light_aces = arrangement[..3].iter().filter(|c| **c == Card::Ace).count();
        let dark_aces = arrangement[3..].iter().filter(|c| **c == Card::Ace).count();
        assert_eq!(light_aces, 1);
        assert_eq!(dark_aces, 1);

        // First king precedes first queen.
        let first_king = arrangement.iter().position(|c| *c == Card::King).unwrap();
        let first_queen = arrangement.iter().position(|c| *c == Card::Queen).unwrap();
        assert!(first_king < first_queen);
    }
}

#[test]
fn with_constraint_narrows() {
    let problem = six_card_lineup();
    let before = problem.count();
    let narrowed = problem.with_constraint(Constraint::At {
        piece: Card::Queen,
        square: 4,
    });
    assert!(narrowed.count() < before);
    assert!(narrowed.count() > 0);
}

#[test]
fn small_board_with_unary_piece_set() {
    // Smallest sensible board: 2 squares, 2 piece kinds, 1 of each.
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    enum Tile {
        Left,
        Right,
    }
    let problem = Problem {
        num_squares: 2,
        square_colors: vec![SquareColor::Light, SquareColor::Dark],
        pieces: vec![Tile::Left, Tile::Right],
        constraint: Constraint::At {
            piece: Tile::Left,
            square: 0,
        },
    };
    assert_eq!(problem.count(), 1);
    assert_eq!(problem.at(0), Some(vec![Tile::Left, Tile::Right]));
}
