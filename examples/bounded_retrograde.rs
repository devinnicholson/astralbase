//! Five-minute library example for the reusable bounded retrograde engine.

use astralbase::{GameValue, ProbeResult, RetrogradeEngine};
use shakmaty::{CastlingMode, Chess, fen::Fen};
use std::str::FromStr;

fn main() {
    // Fool's Mate: White to move is checkmated. This example declares the
    // rules result as a Loss(0); Astralbase does not infer terminality itself.
    let fen = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3";
    let terminal: Chess = Fen::from_str(fen)
        .expect("FEN parses")
        .into_position(CastlingMode::Standard)
        .expect("position is legal standard chess");

    let mut engine = RetrogradeEngine::new();
    engine.add_seed(terminal.clone(), GameValue::Loss(0));
    assert_eq!(
        engine.probe(&terminal),
        ProbeResult::Present(GameValue::Loss(0))
    );

    let expanded = engine.solve(1);
    let winning_parents = engine
        .tablebase
        .values()
        .filter(|value| **value == GameValue::Win(1))
        .count();

    println!("expanded={expanded}");
    println!("winning_parents_at_distance_1={winning_parents}");
    println!("draws_proved=0");

    assert_eq!(expanded, 1);
    assert!(winning_parents > 0);
}
