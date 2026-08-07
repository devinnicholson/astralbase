//! Candidate-side regression adapters for Astralbase claims A01-A07.
//!
//! Expected transitions are frozen in the Partizan v0.1 validation corpus.
//! These tests use `shakmaty`, the same rules library used by Astralbase, so
//! they are consistency tests only. They do not satisfy the protocol's pending
//! independent python-chess lane.

use astralbase::{GameValue, ProbeResult, RetrogradeEngine, retrograde};
use shakmaty::{CastlingMode, Chess, EnPassantMode, Position, fen::Fen, uci::UciMove};
use std::{collections::BTreeSet, str::FromStr};

fn chess(fen: &str) -> Chess {
    Fen::from_str(fen)
        .expect("fixture FEN parses")
        .into_position(CastlingMode::Standard)
        .expect("fixture is a valid standard-chess position")
}

fn normalized_fen(position: &Chess) -> String {
    Fen::from_position(position.clone(), EnPassantMode::Legal).to_string()
}

fn legal_uci(position: &Chess) -> BTreeSet<String> {
    position
        .legal_moves()
        .iter()
        .map(|mv| UciMove::from_standard(mv).to_string())
        .collect()
}

fn assert_inverse_contains(parent_fen: &str, child_fen: &str) {
    let child = chess(child_fen);
    let expected = normalized_fen(&chess(parent_fen));
    let mut parents = Vec::new();
    retrograde::inverse_moves(&child, &mut parents);
    let observed: BTreeSet<_> = parents.iter().map(normalized_fen).collect();

    assert_eq!(
        observed.len(),
        parents.len(),
        "inverse generation emitted duplicate normalized parents"
    );

    assert!(
        observed.contains(&expected),
        "expected inverse parent {expected}; observed {observed:#?}"
    );

    // Same-library round trips catch state-reconstruction bugs but are not the
    // independent oracle required by A05/A06.
    for parent in parents {
        assert!(parent.legal_moves().iter().any(|mv| {
            parent
                .clone()
                .play(mv)
                .is_ok_and(|next| normalized_fen(&next) == normalized_fen(&child))
        }));
    }
}

fn assert_inverse_excludes(parent_fen: &str, child_fen: &str) {
    let child = chess(child_fen);
    let excluded = normalized_fen(&chess(parent_fen));
    let mut parents = Vec::new();
    retrograde::inverse_moves(&child, &mut parents);
    let observed: BTreeSet<_> = parents.iter().map(normalized_fen).collect();

    assert!(
        !observed.contains(&excluded),
        "illegal inverse parent was emitted: {excluded}"
    );
}

#[test]
fn a01_a02_a03_seed_budget_and_loss_parent_propagation() {
    let terminal = chess("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3");
    let mut engine = RetrogradeEngine::new();
    engine.add_seed(terminal.clone(), GameValue::Loss(0));

    assert_eq!(
        engine.probe(&terminal),
        ProbeResult::Present(GameValue::Loss(0))
    );
    assert_eq!(engine.solve(0), 0);
    assert_eq!(engine.solve(1), 1);
    assert!(
        engine
            .tablebase
            .values()
            .any(|value| *value == GameValue::Win(1))
    );
}

#[test]
fn a05_inverse_quiet_pawn_parent() {
    assert_inverse_contains(
        "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
        "4k3/8/8/8/8/4P3/8/4K3 b - - 0 1",
    );
}

#[test]
fn a05_a06_inverse_promotion_parent() {
    assert_inverse_contains(
        "4k3/P7/8/8/8/8/8/4K3 w - - 0 1",
        "Q3k3/8/8/8/8/8/8/4K3 b - - 0 1",
    );
}

#[test]
fn a05_a06_inverse_en_passant_parent() {
    assert_inverse_contains(
        "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
        "4k3/8/3P4/8/8/8/8/4K3 b - - 0 1",
    );
}

#[test]
fn a05_a06_inverse_castling_parent() {
    assert_inverse_contains(
        "4k2r/8/8/8/8/8/8/4K2R w Kk - 0 1",
        "4k2r/8/8/8/8/8/8/5RK1 b k - 1 1",
    );
}

#[test]
fn a05_a06_frozen_independent_oracle_cases_match_inverse_generation() {
    let fixture = include_str!("fixtures/python_chess_transitions_v1.tsv");
    let mut case_count = 0;

    for line in fixture.lines().skip(1).filter(|line| !line.is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 5, "malformed oracle fixture row: {line}");
        assert_inverse_contains(fields[2], fields[4]);
        case_count += 1;
    }

    assert_eq!(case_count, 4);
}

#[test]
fn a05_a06_inverse_castling_rejects_attacked_transit_square() {
    assert_inverse_excludes(
        "4kr2/8/8/8/8/8/8/4K2R w K - 0 1",
        "4kr2/8/8/8/8/8/8/5RK1 b - - 1 1",
    );
}

#[test]
fn a06_forward_rule_edges_match_frozen_fixture_contract() {
    let promotions = legal_uci(&chess("4k3/P7/8/8/8/8/8/4K3 w - - 0 1"));
    for expected in ["a7a8q", "a7a8r", "a7a8b", "a7a8n"] {
        assert!(promotions.contains(expected));
    }

    assert!(legal_uci(&chess("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1")).contains("e5d6"));
    assert!(!legal_uci(&chess("4k3/8/8/3pP3/8/8/8/4K3 w - - 0 1")).contains("e5d6"));
    assert!(legal_uci(&chess("4k2r/8/8/8/8/8/8/4K2R w Kk - 0 1")).contains("e1g1"));
    assert!(!legal_uci(&chess("4kr2/8/8/8/8/8/8/4K2R w K - 0 1")).contains("e1g1"));
}

#[test]
fn a07_absent_unknown_and_rules_draw_are_not_conflated() {
    let dead_position = chess("4k3/8/8/8/8/8/8/4K3 w - - 0 1");
    let mut engine = RetrogradeEngine::new();
    assert_eq!(engine.probe(&dead_position), ProbeResult::Absent);

    engine.add_seed(dead_position.clone(), GameValue::Unknown);
    assert_eq!(
        engine.probe(&dead_position),
        ProbeResult::Present(GameValue::Unknown)
    );
    assert!(dead_position.is_insufficient_material());
}

#[test]
fn a03_work_budget_is_incremental_and_preserves_epistemic_states() {
    let terminal = chess("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3");
    let unknown = chess("4k3/8/8/8/8/8/8/4K3 w - - 0 1");
    let absent = chess("4k3/8/8/8/8/8/3K4/8 w - - 0 1");
    let mut engine = RetrogradeEngine::new();
    engine.add_seed(terminal, GameValue::Loss(0));
    engine.add_seed(unknown.clone(), GameValue::Unknown);

    assert_eq!(engine.solve(1), 1);
    assert_eq!(
        engine.probe(&unknown),
        ProbeResult::Present(GameValue::Unknown)
    );
    assert_eq!(engine.probe(&absent), ProbeResult::Absent);
    assert_eq!(engine.solve(0), 0);
    assert_eq!(
        engine.probe(&unknown),
        ProbeResult::Present(GameValue::Unknown)
    );
    assert_eq!(engine.probe(&absent), ProbeResult::Absent);
    assert_eq!(engine.solve(1), 1);
    assert_eq!(
        engine.probe(&unknown),
        ProbeResult::Present(GameValue::Unknown)
    );
    assert_eq!(engine.probe(&absent), ProbeResult::Absent);
}
