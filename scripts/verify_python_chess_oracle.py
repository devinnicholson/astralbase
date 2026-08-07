#!/usr/bin/env python3
"""Validate frozen predecessor transitions with an independent rules library."""

from __future__ import annotations

import csv
import hashlib
from pathlib import Path

import chess


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "python_chess_transitions_v1.tsv"
EXPECTED_FIXTURE_SHA256 = "d35138b98ace8a77f42fae19d2baafe82c3acf2e9342e056f5a2fa820d8cc272"


def main() -> None:
    payload = FIXTURE.read_bytes()
    digest = hashlib.sha256(payload).hexdigest()
    if digest != EXPECTED_FIXTURE_SHA256:
        raise SystemExit(
            "oracle fixture digest changed; review the cases and update "
            "EXPECTED_FIXTURE_SHA256 explicitly"
        )
    rows = list(csv.DictReader(payload.decode("utf-8").splitlines(), delimiter="\t"))
    if not rows:
        raise SystemExit("oracle fixture is empty")

    seen: set[str] = set()
    for row in rows:
        case_id = row["case_id"]
        if case_id in seen:
            raise SystemExit(f"duplicate case_id: {case_id}")
        seen.add(case_id)

        board = chess.Board(row["parent_fen"])
        move = chess.Move.from_uci(row["uci"])
        if move not in board.legal_moves:
            raise SystemExit(f"{case_id}: {move.uci()} is not legal")
        board.push(move)
        observed = board.fen(en_passant="legal")
        if observed != row["child_fen"]:
            raise SystemExit(
                f"{case_id}: child mismatch\nexpected: {row['child_fen']}\nobserved: {observed}"
            )

    print(f"python-chess={chess.__version__}")
    print(f"cases={len(rows)}")
    print(f"fixture_sha256={digest}")


if __name__ == "__main__":
    main()
