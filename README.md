# Astralbase

[![CI](https://github.com/devinnicholson/astralbase/actions/workflows/ci.yml/badge.svg)](https://github.com/devinnicholson/astralbase/actions/workflows/ci.yml)

Astralbase is an experimental Rust library for bounded, in-memory retrograde
exploration of orthodox chess positions. Its reusable surface provides
candidate predecessor generation and queue-based `Win`/`Loss` propagation from
caller-declared seeds. The default `partizan-dataset` feature retains the
versioned schemas, generators, replay checks, and diagnostics used by the
Partizan research suite.

Astralbase v0.1 covers bounded predecessor exploration and proof propagation.
Draw solving, CGT canonicalization, persistent tablebases, and completeness
certificates lie outside its scope.

The package is awaiting its first publication. It requires Rust 1.88 or newer
and uses [GPL-3.0-or-later](LICENSE), matching its direct dependency on
[Shakmaty](https://github.com/niklasf/shakmaty) (GPL-3.0).

## Status and role

| Item | Current state |
| --- | --- |
| Crate | `astralbase` 0.1.0 research candidate |
| Reusable core | In-memory predecessor exploration and `Win`/`Loss` propagation |
| Default feature | `partizan-dataset` research schemas and artifact tooling |
| Minimum Rust | 1.88 |
| License | GPL-3.0-or-later |
| Registry release | Pending on Bitmesh and Thermograph 0.1.0 |

Astralbase is the bounded search layer in the
[Partizan](https://github.com/devinnicholson/partizan) stack. It obtains chess
transitions from Shakmaty, may consume conservative board certificates from
[Bitmesh](https://github.com/devinnicholson/bitmesh), and may attach explicit
finite-game identities from
[Thermograph](https://github.com/devinnicholson/thermograph) in the optional
dataset feature. The reusable retrograde engine itself has no CGT or
decomposition dependency.

Authority remains separated: callers declare terminal seeds, Astralbase
propagates only the bounded proof consequences recorded by the engine, and
Partizan decides whether a resulting artifact satisfies a research protocol.

## Five-minute reusable example

Bitmesh and Thermograph are unpublished `0.1.0` dependencies. From sibling
checkouts, supply local patches only on the Cargo command line:

```console
cargo \
  --config 'patch."crates-io".bitmesh.path="../bitmesh"' \
  --config 'patch."crates-io".thermograph.path="../thermograph"' \
  run --locked --no-default-features --example bounded_retrograde
```

The example declares Fool's Mate as `Loss(0)`, expands one queued position, and
reports the discovered `Win(1)` parents. Its final lines include:

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

The caller supplies terminality and seed value. Research workflows should
ground both in a cited rules oracle or independently validated input.

## Executable v0.1 semantics

Distances count plies from the represented side-to-move position to a declared
terminal seed.

| API result | Meaning | Boundary |
| --- | --- | --- |
| `Win(n)` | The side to move has a discovered route to a seeded loss in `n` plies. v0.1 records the first proving loss child. | Global minimum distance, general completeness, and rules draws remain open. |
| `Loss(n)` | Every legal child has been proved `Win`; the losing side delays, so `n = 1 + max(child distances)`. | Positions with unresolved children remain unproved. |
| `Unknown` | The row is stored without a win/loss proof. | Draw, stalemate, insufficient material, and absence have separate meanings. |
| `Absent` | `probe` found no stored row. | This is a table-membership result only. |

Queue order leaves a losing distance unchanged because the engine records the
maximum winning-child distance before producing a loss. Distance arithmetic
saturates at `u32::MAX`, the v0.1 distance ceiling.

## Supported domain and limits

The reusable engine accepts any legal `shakmaty::Chess` position that fits in
memory. It has no built-in piece cap. `solve(max_expansions)` supplies an
explicit work bound, and all state disappears when the process exits.

Immediate transition handling includes ordinary moves, captures, promotions,
en passant, and standard castling. Inverse parents preserve side to move and
reconstruct relevant castling and en-passant state where possible. Tests replay
each emitted parent with `shakmaty`, establishing same-library consistency.
Independent rules evidence comes from the separately maintained python-chess
validation lane.

The engine scope excludes:

- threefold/fivefold repetition and position history;
- the fifty-/seventy-five-move rules;
- dead positions and insufficient-material draws;
- stalemate/draw propagation;
- exhaustive enumeration and predecessor-completeness proofs;
- persistent or distributed tablebases; and
- CGT values, canonical forms, temperatures, and additive decomposition.

The optional Partizan domain gate is narrower: legal standard 8×8 FEN, at most
eight pieces, no castling rights, and no en-passant target. A non-terminal
position also needs an immediate terminal tactic or a strict Bitmesh structural
certificate. The certificate covers the supplied board and Bitmesh's documented
one-ply screen.

## Partizan dataset feature

`partizan-dataset` is enabled by default for source compatibility. It adds
Bitmesh/Thermograph-backed dataset schemas, experimental diagnostics, artifact
manifests, and the CLI binary. Use `--no-default-features` for the reusable
engine alone.

The dataset layer exposes four disjoint label kinds under
`partizan.dataset_label.v0`: `exact`, `rejected`, `heuristic`, and `prediction`.
`parse_and_validate_jsonl` checks row shape and provenance requirements.
Composition replay functions perform the narrower recomputations named in
their APIs. Schema validation leaves chess rules, component values, and
learning efficacy to their respective validation lanes.

Generate and verify the deterministic sample artifact:

```console
cargo \
  --config 'patch."crates-io".bitmesh.path="../bitmesh"' \
  --config 'patch."crates-io".thermograph.path="../thermograph"' \
  run --locked -- --sample-label-artifact target/sample-artifact
cargo \
  --config 'patch."crates-io".bitmesh.path="../bitmesh"' \
  --config 'patch."crates-io".thermograph.path="../thermograph"' \
  run --locked -- --verify-artifact target/sample-artifact/manifest.json
```

The manifest contains no timestamp or host path. Repeated runs of v0.1 produce
byte-identical files:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `sample-label-shard.jsonl` | 5,220 | `fb1f096175b8dbbe15b8dbb0bed27ac2ccff7f8179219fc9f446cc8ddb0b1925` |
| `manifest.json` | 411 | `0bcdc46b81c684edf5c0391e224980c3dbb608542df404f9c45c987760747531` |

List all bounded generation and replay commands with:

```console
cargo \
  --config 'patch."crates-io".bitmesh.path="../bitmesh"' \
  --config 'patch."crates-io".thermograph.path="../thermograph"' \
  run --locked -- --help
```

The diagnostic report structs remain an unstable Rust surface in v0.1. Model
benefit requires a separate learning experiment.

## Installation and release-candidate testing

After Bitmesh, Thermograph, and Astralbase `0.1.0` reach a registry, consumers
can use ordinary versioned dependencies. Repository files contain no sibling
path dependency or absolute developer path.

Until upstream publication, fresh Cargo resolution from a standalone clone is
a release blocker, including no-default-feature resolution. Maintainers can
test sibling working trees with the command-line patches shown above. Begin
with the inexpensive dependency check:

```console
cargo \
  --config 'patch."crates-io".bitmesh.path="../bitmesh"' \
  --config 'patch."crates-io".thermograph.path="../thermograph"' \
  check --locked --no-default-features
```

Quoting `"crates-io"` addresses the intended Cargo configuration-table key.

The reviewed upstream candidates are Bitmesh
`b7c7858df2365d8ea4bd2f50ff2afbd51a6f8225` and Thermograph
`381e88dad1a1259a4ccc1a015537f8a8acaf7474`. CI checks out those exact
revisions. Astralbase's versioned dependency specifications remain blocked on
registry publication.

## Validation status

The Partizan v0.1 protocol maps Astralbase to claims A01-A07. Checked tests cover
seed retention, zero-budget behavior, propagation, delaying loss distance,
`Absent`/`Unknown` distinctions, and targeted quiet-move, promotion, en-passant,
and castling transitions.

Candidate-side Shakmaty tests establish same-library consistency. A
python-chess 1.11.2 lane passed against an earlier committed integration
baseline; its structured result has SHA-256
`0facb6f25c995ab217528de48cb29fb537b5a174b92bdd8ef6d53e82eb04dc0d`.
The raw evidence lives in the Partizan-Fugue suite. The reviewed upstream
integration requires a refreshed independent result against the pinned
candidate commits.

Repetition, fifty-/seventy-five-move handling, draw completeness, persistence,
large-scale generation, and model-benefit claims remain outside v0.1.

## Development

Run `cargo fmt --check` directly. Add the two command-line patches to every
other command during the pre-publication candidate phase:

```text
cargo clippy <PATCHES> --locked --all-targets --all-features -- -D warnings
cargo <PATCHES> test --locked --all-targets --all-features
cargo <PATCHES> rustdoc --locked --lib --all-features -- -D warnings -D missing-docs
cargo <PATCHES> run --locked --no-default-features --example bounded_retrograde
cargo <PATCHES> package --locked
```

`<PATCHES>` means the two `--config` arguments shown above. For `clippy`, place
them after the subcommand as displayed.

See [CONTRIBUTING.md](CONTRIBUTING.md) for validation expectations,
[CHANGELOG.md](CHANGELOG.md) for the candidate contract,
[docs/formal_domain.md](docs/formal_domain.md) for dataset domains, and
[CITATION.cff](CITATION.cff) for citation metadata.
