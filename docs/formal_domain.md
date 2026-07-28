# Astralbase v0.1 domain contracts

This file supplies the stable local anchors embedded in dataset provenance.
They describe executable generator contracts. Chess-decomposition theorems and
empirical machine-learning claims require separate evidence.

## First candidate domain

`formal_domain:first_constrained_chess:v0` accepts a FEN only when:

- it parses as a legal orthodox 8×8 `shakmaty::Chess` position;
- it contains at most eight pieces;
- it has no castling rights and no en-passant target; and
- it is terminal, has an immediate checkmating/stalemating move, or receives a
  strict Bitmesh structural partition.

The Bitmesh gate is a conservative board-structural contract covering the
supplied board and its documented one-ply screen. Future independence, additive
CGT values, draws, repetition history, and fifty-/seventy-five-move state are
outside this domain.

## Wave 17 composition fixture

`formal_domain:bitmesh_composition_fixture:v0` contains synthetic hard-target
rows used to exercise nested certificate validation. Fixture FENs may vary move
counters to trigger missing, stale, duplicate, and unsupported-value controls.
They are validation inputs. Reachability and decomposition theorems require
separate evidence.

## Wave 18 non-fixture composed domain

`formal_domain:bitmesh_composed_chess:v0` contains generated chess-position
candidates. Rows that fail the constrained-domain or conservative certificate
contract receive structured rejection labels.

## Wave 18 board-material composition

`formal_domain:bitmesh_composed_board_material:v0` contains Board-level
composition candidates used by Partizan diagnostics. Board-level acceptance
covers structural material only; orthodox-chess reachability requires separate
evidence. Exact rows must carry the Bitmesh decomposition/composition digests,
component value payloads, and the Thermograph result digest required by the
dataset validator.
