//! Bring-your-own piece kind, colour kind, and board size.
//!
//! The crate is generic over all three, so it can solve any "lay
//! these pieces out under these positional constraints" problem —
//! not only chess back-rank shuffles.
//!
//! This example arranges a six-card lineup of `Ace` / `King` / `Queen`
//! pairs over a 6-square board with a user-defined three-zone colour
//! set, then walks through the public API: `count`, `iter`, `at`,
//! `sample`, and `with_constraint`.
//!
//! Run with `cargo run --example custom`.

// The crate is generic over piece kind and colour kind; we use a
// `Card` enum for pieces and a `Zone` enum for colours.

use chess_startpos_rs::{Constraint, CountOp, Problem};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Card {
    Ace,
    King,
    Queen,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
enum Zone {
    Red,
    Green,
    Blue,
}

fn main() {
    // 6 squares split into three pairs of zones. `CountOnColor` can
    // key off any user-defined colour kind.
    let colors = vec![
        Zone::Red,
        Zone::Red,
        Zone::Green,
        Zone::Green,
        Zone::Blue,
        Zone::Blue,
    ];

    // Alphabet (set of distinct kinds available). Counts come from
    // Count constraints below, not from this field.
    let alphabet = vec![Card::Ace, Card::King, Card::Queen];

    // Constraints:
    //   1. Pin the per-piece counts: 2 Aces, 2 Kings, 2 Queens.
    //   2. Aces distributed one in Red, one in Blue, none in Green.
    //   3. The first King precedes the first Queen.
    let constraint = Constraint::And(vec![
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
            color: Zone::Red,
            op: CountOp::Eq,
            value: 1,
        },
        Constraint::CountOnColor {
            piece: Card::Ace,
            color: Zone::Blue,
            op: CountOp::Eq,
            value: 1,
        },
        Constraint::CountOnColor {
            piece: Card::Ace,
            color: Zone::Green,
            op: CountOp::Eq,
            value: 0,
        },
        Constraint::Order(vec![(Card::King, 0), (Card::Queen, 0)]),
    ]);

    let problem: Problem<Card, Zone> = Problem {
        num_squares: 6,
        square_colors: colors,
        pieces: alphabet,
        constraint,
    };

    println!("count          = {}", problem.count());
    println!("first (at 0)   = {:?}", problem.at(0).unwrap());
    println!(
        "last (at last) = {:?}",
        problem.at(problem.count() - 1).unwrap()
    );
    println!("sample(seed=7) = {:?}", problem.sample(7).unwrap());

    // Narrow further: pin a Queen onto square 4.
    let narrowed = problem.with_constraint(Constraint::At {
        piece: Card::Queen,
        square: 4,
    });
    println!(
        "with Queen on square 4: count = {} (down from {})",
        narrowed.count(),
        problem.count()
    );
}
