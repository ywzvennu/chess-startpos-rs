//! Bring-your-own piece kind on a non-8-square board.
//!
//! The crate is generic over both the piece type and the board size, so
//! it can solve any "lay this multiset out under these positional
//! constraints" problem — not only chess back-rank shuffles.
//!
//! This example arranges a six-card lineup of `Ace` / `King` / `Queen`
//! pairs under three constraints, then walks through the public API:
//! `count`, `iter`, `at`, `sample`, and `with_constraint`.
//!
//! Run with `cargo run --example custom`.

use chess_startpos_rs::{Constraint, CountOp, Problem, SquareColor};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Card {
    Ace,
    King,
    Queen,
}

fn main() {
    // A 6-square row split into two halves. We use `square_colors` as
    // an arbitrary partition (Light = first half, Dark = second half),
    // which `CountOnColor` can then key off.
    let halves = vec![
        SquareColor::Light,
        SquareColor::Light,
        SquareColor::Light,
        SquareColor::Dark,
        SquareColor::Dark,
        SquareColor::Dark,
    ];

    // Multiset: two of each card.
    let pieces = vec![
        Card::Ace,
        Card::Ace,
        Card::King,
        Card::King,
        Card::Queen,
        Card::Queen,
    ];

    // Constraints:
    //   1. Exactly one Ace in each half (split aces across the row).
    //   2. The first King precedes the first Queen.
    let constraint = Constraint::And(vec![
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
    ]);

    let problem = Problem {
        num_squares: 6,
        square_colors: halves,
        pieces,
        constraint,
    };

    println!("count           = {}", problem.count());
    println!("first  (at 0)   = {:?}", problem.at(0).unwrap());
    println!(
        "last           = {:?}",
        problem.at(problem.count() - 1).unwrap()
    );
    println!("sample(seed=7)  = {:?}", problem.sample(7).unwrap());

    // Narrow further: pin a Queen onto square 4.
    let narrowed = problem.clone().with_constraint(Constraint::At {
        piece: Card::Queen,
        square: 4,
    });
    println!(
        "with Queen on square 4: count = {} (down from {})",
        narrowed.count(),
        problem.count()
    );
}
