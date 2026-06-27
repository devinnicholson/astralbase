use astralbase::{GameValue, RetrogradeEngine, dataset_label};
use shakmaty::{CastlingMode, Chess, fen::Fen};
use std::str::FromStr;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--sample-label-shard") => {
            print!(
                "{}",
                dataset_label::sample_audited_shard_jsonl()
                    .expect("sample audited shard must serialize")
            );
            return;
        }
        Some(arg) => {
            eprintln!("unsupported argument: {arg}");
            std::process::exit(2);
        }
        None => {}
    }

    let fen_str = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3";
    let pos: Chess = Fen::from_str(fen_str)
        .expect("sample FEN must parse")
        .into_position(CastlingMode::Standard)
        .expect("sample FEN must be a valid chess position");

    let mut engine = RetrogradeEngine::new();
    engine.add_terminal(pos, GameValue::Loss(0));

    let expanded = engine.solve(100);
    println!("Astralbase prototype expanded {expanded} nodes from {fen_str}");
}
