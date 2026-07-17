# Astralbase v0.1 domain contracts

This file supplies the stable local anchors embedded in dataset provenance.
They describe executable generator contracts, not mathematical proofs of chess
decomposition or empirical machine-learning claims.

## First candidate domain

`formal_domain:first_constrained_chess:v0` accepts a FEN only when:

- it parses as a legal orthodox 8×8 `shakmaty::Chess` position;
- it contains at most eight pieces;
- it has no castling rights and no en-passant target; and
- it is terminal, has an immediate checkmating/stalemating move, or receives a
  strict Bitmesh structural partition.

The Bitmesh gate is a conservative board-structural contract. It does not prove
that regions remain independent throughout future play or that their CGT values
add. Draws, repetition history, and fifty-/seventy-five-move state are outside
this domain.

## Wave 17 composition fixture

`formal_domain:bitmesh_composition_fixture:v0` contains synthetic hard-target
rows used to exercise nested certificate validation. Fixture FENs may vary move
counters to trigger missing, stale, duplicate, and unsupported-value controls.
They are validation inputs, not evidence of reachability or a decomposition
theorem.

## Wave 18 non-fixture composed domain

`formal_domain:bitmesh_composed_chess:v0` contains generated chess-position
candidates. Rows that fail the constrained-domain or conservative certificate
contract are emitted as structured rejections rather than exact labels.

## Wave 18 board-material composition

`formal_domain:bitmesh_composed_board_material:v0` contains Board-level
composition candidates used by Partizan diagnostics. A Board-level acceptance
does not establish that the board is a reachable orthodox-chess position. Exact
rows must carry the Bitmesh decomposition/composition digests, component value
payloads, and a Thermograph result digest required by the dataset validator.
