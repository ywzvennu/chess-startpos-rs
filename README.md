# chess-startpos-rs

Generate, count, and sample chess back-rank arrangements under composable
constraints.

The crate provides a generic constraint engine: you define a piece kind, a
board (number of squares + per-square colour), and a multiset of pieces;
constraints (count, position, ordering, colour-keyed counts) compose with
`And` / `Or` / `Not`; and the engine then enumerates, counts, samples, or
indexes the satisfying arrangements deterministically.

An opinionated `chess` module ships ready-to-use presets for the canonical
shuffle variants:

- `chess::standard()` — count `1`. The FIDE starting back rank.
- `chess::shuffle()` — count `5040`. No constraints beyond the multiset.
- `chess::pre_chess()` — count `2880`. Adds bishops on opposite-colour
  squares.
- `chess::chess960()` — count `960`. Adds bishops opposite + king between
  the rooks. `at(N)` matches the canonical Chess960 SP-ID numbering.

## Status

Initial development. Public API will stabilise at v0.1.0.

## License

[MIT](LICENSE)
