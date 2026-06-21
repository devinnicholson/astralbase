# Astralbase

[![Crates.io](https://img.shields.io/crates/v/astralbase.svg)](https://crates.io/crates/astralbase)
[![Docs.rs](https://docs.rs/astralbase/badge.svg)](https://docs.rs/astralbase)

Astralbase is a Combinatorial Game Theory (CGT) native Retrograde Analysis engine.

## Overview

Standard endgame tablebases (such as Syzygy or Nalimov) evaluate perfect-play positions down to scalar states: Win, Draw, or Loss. Astralbase operates on a higher algebraic dimension. Instead of returning a scalar, it performs inverse move generation to calculate the exact Combinatorial Game Tree (CGT) for every legal position.

By outputting canonical game forms, Astralbase provides the ground-truth data necessary to train machine learning models on topological game decoupling and surreal loss functions.

## Features

- **Inverse Move Generation**: High-performance backtracking from terminal checkmate/stalemate states.
- **CGT Backpropagation**: Propagates exact combinatorial values ($\ast$, $\uparrow$, temperatures) rather than simple evaluation scores.
- **Disk Persistence**: Stores canonical tree forms on disk for $O(1)$ retrieval during Neural Network training.

## Architecture

Astralbase is implemented as a parallelized Rust binary. It uses `bitmesh` for localized reachability limits and `thermograph` to canonicalize the game trees as they are backpropagated up the analysis tree. 

## Usage

```bash
cargo run --release -- --pieces 3 --output ./data/3-piece.cgt
```

## Research Context

This engine is the core dataset generator for the Partizan research project, aimed at proving that deep reinforcement learning models can learn game-theoretic representations when provided with combinatorial ground-truth data.
