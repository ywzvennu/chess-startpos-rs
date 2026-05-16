//! Micro-benchmarks for the four chess presets and the Cartesian
//! fallback solver regime.
//!
//! Run with `cargo bench`.

use std::hint::black_box;

use chess_startpos_rs::{chess, Constraint, CountOp, Problem, SquareColor};
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_count_standard(c: &mut Criterion) {
    c.bench_function("chess::standard().count", |b| {
        b.iter(|| black_box(chess::standard()).count())
    });
}

fn bench_count_shuffle(c: &mut Criterion) {
    c.bench_function("chess::shuffle().count", |b| {
        b.iter(|| black_box(chess::shuffle()).count())
    });
}

fn bench_count_chess_2880(c: &mut Criterion) {
    c.bench_function("chess::chess_2880().count", |b| {
        b.iter(|| black_box(chess::chess_2880()).count())
    });
}

fn bench_count_chess_960(c: &mut Criterion) {
    c.bench_function("chess::chess_960().count", |b| {
        b.iter(|| black_box(chess::chess_960()).count())
    });
}

fn bench_chess_960_sample(c: &mut Criterion) {
    let preset = chess::chess_960();
    c.bench_function("chess::chess_960().sample", |b| {
        b.iter(|| preset.sample(black_box(0xC0FFEE)))
    });
}

fn bench_chess_960_sp_id_forward(c: &mut Criterion) {
    let preset = chess::chess_960();
    c.bench_function("chess::chess_960().sp_id(518)", |b| {
        b.iter(|| preset.sp_id(black_box(518)))
    });
}

fn bench_chess_960_sp_id_reverse(c: &mut Criterion) {
    let preset = chess::chess_960();
    let arrangement = chess::STANDARD_BACK_RANK.to_vec();
    c.bench_function("chess::chess_960().sp_id_of(standard)", |b| {
        b.iter(|| preset.sp_id_of(black_box(&arrangement)))
    });
}

fn bench_cartesian_fallback(c: &mut Criterion) {
    // Unconstrained Cartesian regime: 4 kinds on 6 squares = 4^6 = 4096.
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    enum Tile {
        A,
        B,
        C,
        D,
    }

    let problem: Problem<Tile> = Problem {
        num_squares: 6,
        square_colors: vec![SquareColor::Light; 6],
        pieces: vec![Tile::A, Tile::B, Tile::C, Tile::D],
        constraint: Constraint::Count {
            piece: Tile::A,
            op: CountOp::Ge,
            value: 1,
        },
    };

    c.bench_function("cartesian_fallback (4^6 = 4096)", |b| {
        b.iter(|| black_box(&problem).count())
    });
}

criterion_group!(
    benches,
    bench_count_standard,
    bench_count_shuffle,
    bench_count_chess_2880,
    bench_count_chess_960,
    bench_chess_960_sample,
    bench_chess_960_sp_id_forward,
    bench_chess_960_sp_id_reverse,
    bench_cartesian_fallback,
);
criterion_main!(benches);
