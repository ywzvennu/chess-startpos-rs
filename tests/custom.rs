//! Integration tests for non-chess use of the constraint engine —
//! custom piece kinds, a user-defined colour set, and non-8-square
//! boards.

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
        pieces: vec![Card::Ace, Card::King, Card::Queen], // alphabet
        constraint: Constraint::And(vec![
            Constraint::Count {
                piece: Card::Ace,
                op: CountOp::Eq,
                value: 2,
            },
            Constraint::Count {
                piece: Card::King,
                op: CountOp::Eq,
                value: 2,
            },
            Constraint::Count {
                piece: Card::Queen,
                op: CountOp::Eq,
                value: 2,
            },
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
fn count_matches_expected() {
    // 6!/(2!2!2!) = 90 distinct permutations; aces-split-across-halves
    // and king-before-queen narrows to 27.
    let problem = six_card_lineup();
    assert_eq!(problem.count(), 27);
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
        let light_aces = arrangement[..3].iter().filter(|c| **c == Card::Ace).count();
        let dark_aces = arrangement[3..].iter().filter(|c| **c == Card::Ace).count();
        assert_eq!(light_aces, 1);
        assert_eq!(dark_aces, 1);

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
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    enum Tile {
        Left,
        Right,
    }
    let problem = Problem {
        num_squares: 2,
        square_colors: vec![SquareColor::Light, SquareColor::Dark],
        pieces: vec![Tile::Left, Tile::Right],
        constraint: Constraint::And(vec![
            Constraint::Count {
                piece: Tile::Left,
                op: CountOp::Eq,
                value: 1,
            },
            Constraint::Count {
                piece: Tile::Right,
                op: CountOp::Eq,
                value: 1,
            },
            Constraint::At {
                piece: Tile::Left,
                square: 0,
            },
        ]),
    };
    assert_eq!(problem.count(), 1);
    assert_eq!(problem.at(0), Some(vec![Tile::Left, Tile::Right]));
}

#[test]
fn user_defined_color_set_three_zones() {
    #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
    enum Zone {
        Red,
        Green,
        Blue,
    }

    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    enum Bead {
        White,
        Black,
    }

    let problem: Problem<Bead, Zone> = Problem {
        num_squares: 6,
        square_colors: vec![
            Zone::Red,
            Zone::Red,
            Zone::Green,
            Zone::Green,
            Zone::Blue,
            Zone::Blue,
        ],
        pieces: vec![Bead::White, Bead::Black],
        constraint: Constraint::And(vec![
            Constraint::Count {
                piece: Bead::White,
                op: CountOp::Eq,
                value: 3,
            },
            Constraint::Count {
                piece: Bead::Black,
                op: CountOp::Eq,
                value: 3,
            },
            // One white per zone.
            Constraint::CountOnColor {
                piece: Bead::White,
                color: Zone::Red,
                op: CountOp::Eq,
                value: 1,
            },
            Constraint::CountOnColor {
                piece: Bead::White,
                color: Zone::Green,
                op: CountOp::Eq,
                value: 1,
            },
            Constraint::CountOnColor {
                piece: Bead::White,
                color: Zone::Blue,
                op: CountOp::Eq,
                value: 1,
            },
        ]),
    };

    // 2 positions in each of 3 zones, pick 1 of 2 squares per zone for
    // white → 2^3 = 8 arrangements.
    assert_eq!(problem.count(), 8);
}
