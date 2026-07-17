# Contributing to Astralbase

Astralbase is pre-1.0 research software. Changes should preserve the distinction
between executable contracts, independently validated claims, hypotheses, and
roadmap items.

## Before opening a change

1. State whether the change affects the reusable retrograde API, the optional
   Partizan dataset surface, or both.
2. Add a small deterministic fixture before changing an algorithm.
3. Identify expected-result provenance. Candidate output cannot validate itself.
4. Update the README and changelog when semantics, scope, or limitations change.
5. Do not add a license header until the repository owner records the license
   decision for the complete code history.

## Validation rules

- A `shakmaty` replay of an Astralbase inverse move is a same-library consistency
  test. Label it that way; do not present it as an independent chess oracle.
- Changes to promotions, en passant, castling, or FEN normalization also require
  comparison with the frozen python-chess lane in the Partizan validation
  protocol before release sign-off.
- `Unknown` and `Absent` are epistemic states. Neither may be treated as a draw.
- Loss distance is one plus the maximum distance of all proved-Win children.
- A work-budget cutoff never proves completeness.
- Partizan dataset outputs must be deterministic and carry a versioned manifest
  plus checksums.

## Local checks

While Bitmesh and Thermograph are unpublished, define these shell arguments:

```text
--config 'patch."crates-io".bitmesh.path="../bitmesh"'
--config 'patch."crates-io".thermograph.path="../thermograph"'
```

Keep the quotes around `"crates-io"`; without them Cargo does not address the
intended registry patch table. Smoke dependency resolution before expensive
tests with `cargo <PATCHES> check --locked --no-default-features`.

Use both patch arguments for full-feature checks:

```text
cargo fmt --check
cargo clippy <PATCHES> --locked --all-targets --all-features -- -D warnings
cargo <PATCHES> test --locked --all-targets --all-features
cargo <PATCHES> rustdoc --locked --lib --all-features -- -D missing-docs
cargo <PATCHES> run --locked --no-default-features --example bounded_retrograde
```

`cargo-clippy` forwards configuration reliably when `<PATCHES>` follows the
`clippy` subcommand, as shown above. The other built-in Cargo subcommands accept
the patch arguments in the global position.

Do not commit absolute path patches or a developer-specific `.cargo/config.toml`.

## Pull-request evidence

Include the exact commands, toolchain versions, pass/fail counts, wall time for
the full generator suite, changed public semantics, and unresolved independent
validation gaps. Large generated files belong in a versioned artifact release,
not an undocumented temporary directory.
