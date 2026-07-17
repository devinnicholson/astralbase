# Astralbase

[![CI](https://github.com/devinnicholson/astralbase/actions/workflows/ci.yml/badge.svg)](https://github.com/devinnicholson/astralbase/actions/workflows/ci.yml)

Astralbase is an experimental Rust library for bounded, in-memory retrograde
exploration of orthodox chess positions. Its reusable surface consists of
candidate predecessor generation and queue-based `Win`/`Loss` propagation from
caller-declared seeds. The default `partizan-dataset` feature also retains the
historical schemas and generators used by the Partizan research project.

Astralbase v0.1 is not a complete chess tablebase, a draw solver, or a CGT
canonical-form engine. It has no persistent tablebase format or completeness
certificate.

The package has not been published yet. The repository owner's license decision
is also pending; public source visibility alone does not grant reuse rights.

## Five-minute reusable example

Run the checked library example without the Partizan dataset layer:

```console
cargo run --no-default-features --example bounded_retrograde
```

It declares Fool's Mate as `Loss(0)`, expands one queued position, and reports
the discovered `Win(1)` parents. The expected final lines include:

```text
expanded=1
draws_proved=0
```

The corresponding API is small:

```rust
use astralbase::{GameValue, ProbeResult, RetrogradeEngine};
use shakmaty::{CastlingMode, Chess, fen::Fen};
use std::str::FromStr;

let terminal: Chess = Fen::from_str(
    "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3",
)?
.into_position(CastlingMode::Standard)?;

let mut engine = RetrogradeEngine::new();
engine.add_seed(terminal.clone(), GameValue::Loss(0));
assert_eq!(engine.solve(1), 1);
assert_eq!(engine.probe(&terminal), ProbeResult::Present(GameValue::Loss(0)));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Astralbase trusts the supplied seed. In research workflows, terminality and the
seed value must come from a cited rules oracle or independently validated input.

## Exact v0.1 semantics

Distances are plies (half-moves) from the represented side-to-move position to
a declared terminal seed.

| API result | Meaning | Not implied |
| --- | --- | --- |
| `Win(n)` | The side to move has a discovered route to a seeded loss in `n` plies. v0.1 records the first proving loss child. | A globally minimal distance-to-mate value, general completeness, or a rules draw. |
| `Loss(n)` | Every legal child has been proved `Win`; the losing side delays, so `n = 1 + max(child distances)`. | A value for positions whose children have not all been proved. |
| `Unknown` | The row is stored but no win/loss proof is recorded. | Draw, stalemate, insufficient material, or absence. |
| `Absent` | `probe` found no row for the position. | `Unknown`, draw, or proof that no forced result exists. |

Queue order does not choose a losing distance. The engine records the maximum
winning-child distance before producing a loss. Distance arithmetic saturates
at `u32::MAX`; v0.1 does not support longer certified distances.

## Supported domain and limits

The reusable engine accepts any legal `shakmaty::Chess` position that fits in
memory. It has no built-in piece cap. `solve(max_expansions)` is an explicit
work bound, and all state is lost when the process exits.

Immediate transition handling includes ordinary moves, captures, promotions,
en passant, and standard castling. Inverse parents preserve side to move and
reconstruct relevant castling/en-passant state where possible. Tests replay
each emitted parent with `shakmaty`; that is a same-library consistency check,
not independent validation.

The engine does **not** model or prove:

- threefold/fivefold repetition or position history;
- the fifty-/seventy-five-move rules;
- dead positions or insufficient-material draws;
- stalemate/draw propagation;
- exhaustive enumeration or proof that predecessor generation is complete;
- persistent or distributed tablebases;
- CGT values, canonical forms, temperatures, or additive decomposition.

The optional Partizan domain gate is narrower: legal standard 8×8 FEN, at most
eight pieces, no castling rights, and no en-passant target. A non-terminal
position also needs an immediate terminal tactic or a strict Bitmesh structural
certificate. That certificate is not a full game-tree decomposition theorem.

## Partizan dataset feature

`partizan-dataset` is enabled by default for source compatibility. It adds
Bitmesh/Thermograph-backed dataset schemas, experimental selection diagnostics,
and the CLI binary. Use `--no-default-features` when only the reusable engine is
needed.

Generate a deterministic sample shard with a SHA-256 manifest:

```console
cargo run -- --sample-label-artifact target/sample-artifact
cargo run -- --verify-artifact target/sample-artifact/manifest.json
```

The manifest contains no timestamp or host path. Repeated runs of one package
version produce byte-identical payload and manifest files.

Historical Partizan research commands remain available; list them with:

```console
cargo run -- --help
```

The dataset diagnostics are an unstable research surface. They are separated
from the reusable engine by the `partizan-dataset` feature and are not evidence
that a learning model benefits from CGT supervision.

## Installation and release-candidate testing

After Bitmesh, Thermograph, and Astralbase `0.1.0` are published, a standalone
consumer will use ordinary versioned dependencies. Astralbase intentionally no
longer commits sibling `path` dependencies.

Until the two upstream crates are published, fresh Cargo resolution from a
clean clone is a **G2 release blocker**, even when optional features are off.
Maintainers testing the frozen release
candidates can provide non-committed Cargo patches:

```console
cargo \
  --config 'patch."crates-io".bitmesh.path="../bitmesh"' \
  --config 'patch."crates-io".thermograph.path="../thermograph"' \
  test --locked --all-targets --all-features
```

Before a longer check, verify that Cargo actually resolves both unpublished
release candidates through the patches:

```console
cargo \
  --config 'patch."crates-io".bitmesh.path="../bitmesh"' \
  --config 'patch."crates-io".thermograph.path="../thermograph"' \
  check --locked --no-default-features
```

Quoting `"crates-io"` is required because it is one Cargo configuration-table
key; the superficially similar `patch.crates-io...` spelling does not select
the intended registry patch table.

No absolute developer path is stored in `Cargo.toml` or `.cargo/config.toml`.
The frozen upstream candidate commits for this branch are recorded in
`CHANGELOG.md`.

## Validation status

The Partizan v0.1 protocol maps Astralbase to claims A01-A07. Checked tests cover
seed retention, zero-budget behavior, propagation, delaying loss distance,
`Absent`/`Unknown` distinctions, and targeted quiet-move, promotion, en-passant,
and castling transitions.

These candidate-side tests use `shakmaty`, so they do not count as independent
rules validation. Suite-level independent validation with python-chess 1.11.2
passed against this candidate; its structured result has SHA-256
`0facb6f25c995ab217528de48cb29fb537b5a174b92bdd8ef6d53e82eb04dc0d`.
The raw evidence is maintained in the Partizan-Fugue suite rather than this
repository. Repetition, fifty-/seventy-five-move handling, draw completeness,
persistence, and large-scale generation remain explicitly outside v0.1.

## Development

Rust 1.85 or newer is supported. The release checks are:

```text
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo rustdoc --locked --lib --all-features -- -D missing-docs
```

During the pre-publication release-candidate phase, add the two command-line
patches shown above to Cargo commands that enable `partizan-dataset`.

See `CONTRIBUTING.md` for validation expectations, `CHANGELOG.md` for the
candidate contract, and `CITATION.cff` for citation metadata.
