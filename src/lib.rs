//! Generate, count, and sample chess back-rank arrangements under
//! composable constraints.
//!
//! The crate is generic — parameterised over a user-defined piece
//! kind, generic over board size — and built around a small set of
//! composable constraint primitives with `And` / `Or` / `Not`
//! combinators. An opinionated `chess` module ships ready-to-use
//! presets for the canonical chess shuffle variants (Chess960,
//! Pre-Chess, full shuffle).
//!
//! This crate is in initial development; the API will stabilise in
//! v0.1.0.
