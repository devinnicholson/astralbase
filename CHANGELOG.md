# Changelog

All notable changes to Astralbase are recorded here. The project follows
[Semantic Versioning](https://semver.org/) once releases are published.

## [Unreleased]

### Added

- Explicit `Win`, `Loss`, stored `Unknown`, and query-time `Absent` semantics.
- Queue-order-independent delaying distance for losses:
  `1 + max(winning child distance)`.
- A reusable no-default-feature library example.
- Candidate-side A01-A07 regression fixtures for promotions, en passant,
  castling, inverse-parent round trips, bounded propagation, and state meaning.
- Deterministic sample artifacts with versioned JSON manifests and SHA-256
  payload checksums.
- CI, citation metadata, contribution guidance, and strict reusable-API docs.

### Changed

- Moved the reusable engine implementation to `engine` while preserving root
  re-exports of `GameValue` and `RetrogradeEngine`.
- Isolated Partizan-specific dataset generation behind the default
  `partizan-dataset` feature.
- Replaced mandatory sibling dependencies with versioned `0.1.0` specifications.
- Narrowed public claims to bounded, in-memory retrograde exploration; draw
  completeness, persistence, exhaustive tablebases, and CGT values are
  explicit non-claims.
- Licensed the crate GPL-3.0-or-later, matching its direct Shakmaty (GPL-3.0)
  dependency.
- Raised the minimum supported Rust version to 1.88: the crate already used
  let-chains (stabilized in 1.88), so the declared 1.85 floor did not actually
  compile and CI's own MSRV job was failing.

### Release-candidate inputs

- Bitmesh `28aee03bf1acb299fc82d5119f37893264e070e4`
- Thermograph `57df043a5940f4ea3bf29fcb7920f369e035030c`
- Partizan validation corpus `starter-corpus-v0.1.json`

The two upstream crates are not yet published. Clean standalone full-feature
installation remains blocked until their `0.1.0` releases exist in the chosen
registry; command-line Cargo patches are used only for candidate testing.

## Pre-readiness baseline - 2026-07-08

- Early in-memory `Win`/`Loss`/`Unknown` prototype.
- Partizan-specific dataset generators and diagnostic searches.
