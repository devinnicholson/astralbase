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
- Manifest validation for non-empty generator metadata and payload coverage.
- Golden v0.1 sample-artifact byte lengths and SHA-256 fixtures.

### Changed

- Moved the reusable engine implementation to `engine` while preserving root
  re-exports of `GameValue` and `RetrogradeEngine`.
- Isolated Partizan-specific dataset generation behind the default
  `partizan-dataset` feature.
- Replaced mandatory sibling dependencies with versioned `0.1.0` specifications.
- Narrowed public claims to bounded, in-memory retrograde exploration; draw
  completeness, persistence, exhaustive tablebases, and CGT values are
  outside the supported contract.
- Licensed the crate GPL-3.0-or-later, matching its direct Shakmaty (GPL-3.0)
  dependency.
- Set the minimum supported Rust version to 1.88, matching the crate's let-chain
  usage and CI floor.

### Release-candidate inputs

- Bitmesh `b7c7858df2365d8ea4bd2f50ff2afbd51a6f8225`
- Thermograph `c0f1aae399ff66b0d42681e9735aa1ff889d0816`
- Partizan validation corpus `starter-corpus-v0.1.json`

The reviewed upstream commits and CI pins are frozen above. Release sign-off
still requires registry resolution, the complete patched test gate on remote
CI, and refreshed independent-oracle evidence against these revisions.

The two upstream crates are awaiting publication. Clean standalone resolution
remains blocked until their `0.1.0` releases exist in the chosen registry;
command-line Cargo patches are used only for candidate testing.

## Pre-readiness baseline - 2026-07-08

- Early in-memory `Win`/`Loss`/`Unknown` prototype.
- Partizan-specific dataset generators and diagnostic searches.
