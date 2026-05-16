# Contributing to chess-startpos-rs

Thanks for your interest in improving the crate. This document covers the
day-to-day workflow and the bar for accepted changes.

## Scope

`chess-startpos-rs` is a constraint-based generator for back-rank
arrangements in chess and chess-like games. It provides composable
constraints (count, position, ordering, colour-keyed counts) with
`And` / `Or` / `Not` combinators, a deterministic indexed lookup, and
seeded uniform sampling. It deliberately does not include full-game
logic, move generation, or position evaluation — those belong in
downstream crates that build on this one.

## Development environment

A stable Rust toolchain is sufficient. Required components:

```sh
rustup component add rustfmt clippy
```

All checks the CI runs locally:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps --all-features
```

Tests must remain green and clippy must stay clean before submitting a
pull request.

## Reporting bugs and proposing changes

Open an issue first, especially for behavioural or API changes. Use
the bug report or feature request templates in
`.github/ISSUE_TEMPLATE/`. Link the issue from your pull request so
the merge auto-closes it.

## Pull request guidelines

- Branch off `main`. Use `feature/<slug>`, `fix/<slug>`,
  `refactor/<slug>`, `test/<slug>`, or `chore/<slug>` naming.
- Keep PRs focused. Smaller, single-purpose PRs are easier to review.
- Match the existing commit style: imperative subject under ~72
  characters, optional body explaining the why.
- Every PR must pass CI (fmt, clippy, test, doc).
- Public API changes need rustdoc updates.

## License

By contributing, you agree that your contribution will be licensed
under the same [MIT License](LICENSE) that covers this project.
