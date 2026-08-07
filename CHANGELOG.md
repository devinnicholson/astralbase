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
- A shared transition corpus checked by both Astralbase/Shakmaty and a pinned
  python-chess 1.11.2 independent rules lane.
- Duplicate-parent, attacked-castling, work-budget, and epistemic-state
  regression coverage.
- Security, support, DCO, contribution templates, Dependabot, and a release
  checklist.

### Changed

- Moved the reusable engine implementation to `engine` while preserving root
  re-exports of `GameValue` and `RetrogradeEngine`.
- Made the bounded retrograde core the default crate surface; Partizan dataset
  schemas, generation, replay diagnostics, dependencies, and CLI require the
  explicit `partizan-dataset` feature.
- Split dataset schemas, validation, replay reports, and module tests out of the
  generator implementation without changing schema identifiers or serialized
  sample artifacts.
- Replaced mandatory sibling dependencies with versioned `0.1.0` specifications.
- Narrowed public claims to bounded, in-memory retrograde exploration; draw
  completeness, persistence, exhaustive tablebases, and CGT values are
  outside the supported contract.
- Licensed the crate GPL-3.0-or-later, matching its direct Shakmaty (GPL-3.0)
  dependency.
- Set the minimum supported Rust version to 1.88, matching the crate's let-chain
  usage and CI floor.

### Release-candidate inputs

- Bitmesh `410550c0964004cd7ba9677539f17ae82c139dd8`
- Thermograph `32d6bfbc966f47a87e7249d4ed8818370288e079`
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
