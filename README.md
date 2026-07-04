# Astralbase

[![Crates.io](https://img.shields.io/crates/v/astralbase.svg)](https://crates.io/crates/astralbase)
[![Docs.rs](https://docs.rs/astralbase/badge.svg)](https://docs.rs/astralbase)

Astralbase is an early Rust prototype for chess retrograde analysis.

## Overview

Standard endgame tablebases (such as Syzygy or Nalimov) evaluate perfect-play positions down to scalar states: Win, Draw, or Loss. Astralbase currently focuses on the lower-level retrograde machinery needed for that work: legal predecessor generation and queue-based propagation from terminal positions.

The current public API exposes a small `RetrogradeEngine` with `Win`, `Loss`, and `Unknown` states. CGT canonical forms, persistence, and large-scale generation are roadmap items rather than implemented behavior.

## Features

- **Inverse Move Generation**: Backtrack from terminal chess positions to legal predecessor positions.
- **Retrograde Propagation**: Propagate scalar win/loss distances through a queue.
- **Library API**: Use `RetrogradeEngine` from Rust crates and the `partizan` Python extension.

## Architecture

Astralbase is implemented as a Rust library with a small demo binary. The library owns the retrograde engine so downstream binaries and bindings use one canonical implementation.

## Usage

```bash
cargo run --release
```

Dataset shard commands used by the Partizan research harness:

```bash
cargo run --quiet -- --non-fixture-composed-domain-shard
cargo run --quiet -- --expanded-non-fixture-composed-domain-shard --rows-per-family 10
cargo run --quiet -- --leakage-clean-non-fixture-composed-domain-shard --rows-per-family 10
cargo run --quiet -- --replay-non-fixture-composed-domain-shard /tmp/astralbase-w22-expanded-composition.jsonl
```

## Research Context

This engine is the core dataset generator for the Partizan research project, aimed at proving that deep reinforcement learning models can learn game-theoretic representations when provided with combinatorial ground-truth data.
