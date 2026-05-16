//! Walks through the four canonical chess presets end-to-end.
//!
//! Run with `cargo run --example quickstart`.

use chess_startpos_rs::{chess, Constraint};

fn main() {
    // 1. Each preset is a `Problem` over the standard back-rank
    //    multiset (KQRRBBNN) with a different constraint.
    println!("standard:    count = {}", chess::standard().count());
    println!("shuffle:     count = {}", chess::shuffle().count());
    println!("chess_2880:  count = {}", chess::chess_2880().count());
    println!("chess_960:    count = {}", chess::chess_960().count());

    // 2. Indexed lookup — get the first and last chess_960 arrangements
    //    in canonical lexicographic order.
    let first = chess::chess_960().at(0).expect("non-empty");
    let last = chess::chess_960()
        .at(chess::chess_960().count() - 1)
        .expect("non-empty");
    println!("chess_960[0]   = {first:?}");
    println!("chess_960[959] = {last:?}");

    // 3. Uniformly random, deterministic in seed.
    let drawn = chess::chess_960().sample(0xC0FFEE).expect("non-empty");
    println!("chess_960 sample(seed=0xC0FFEE) = {drawn:?}");

    // 4. Narrow a preset with an extra constraint.
    let narrowed = chess::chess_960().with_constraint(Constraint::At {
        piece: chess::Piece::Queen,
        square: chess::file::D,
    });
    println!(
        "chess_960 with Queen on d1: count = {} (down from {})",
        narrowed.count(),
        chess::chess_960().count(),
    );
}
