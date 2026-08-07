# Release checklist

Astralbase releases follow Bitmesh and Thermograph because the optional
`partizan-dataset` feature depends on both crates.

## Contract review

- [ ] Confirm public API and feature changes in `CHANGELOG.md`.
- [ ] Confirm `default = []` and that `partizan-dataset` remains explicit.
- [ ] Review the meanings of `Win`, `Loss`, `Unknown`, and `Absent`.
- [ ] Confirm no claim of predecessor completeness, draw solving, or exhaustive tablebase coverage.
- [ ] Confirm dataset schema identifiers and deterministic sample bytes are unchanged, or document a versioned schema change.
- [ ] Refresh the independent python-chess fixture lane.

## Clean validation

- [ ] `cargo fmt --check`
- [ ] strict Clippy on all targets and features
- [ ] tests on the minimum supported Rust version
- [ ] default/no-default core tests
- [ ] all-target, all-feature tests
- [ ] strict library rustdoc
- [ ] five-minute reusable example
- [ ] deterministic sample generation and artifact replay
- [ ] `cargo package --locked`
- [ ] inspect `cargo package --list`
- [ ] test the packaged crate from a clean consumer project
- [ ] run advisory and license checks

## Publication

- [ ] Confirm Bitmesh and Thermograph release versions resolve from crates.io.
- [ ] Run `cargo publish --dry-run --locked`.
- [ ] Record toolchain versions and all command results.
- [ ] Obtain explicit maintainer approval for the irreversible registry release.
- [ ] Publish, create an immutable `v0.1.x` tag, and create the GitHub release.
- [ ] Confirm docs.rs built with all features.
- [ ] Record the crate checksum and release provenance.
