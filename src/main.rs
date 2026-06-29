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
        Some("--frontier-label-shard") => {
            let limit = parse_frontier_limit(args);
            print!(
                "{}",
                dataset_label::frontier_audited_shard_jsonl(limit)
                    .expect("frontier audited shard must serialize")
            );
            return;
        }
        Some("--family-frontier-label-shard") => {
            let limit_per_family = parse_family_frontier_limit(args);
            print!(
                "{}",
                dataset_label::family_frontier_audited_shard_jsonl(limit_per_family)
                    .expect("family frontier audited shard must serialize")
            );
            return;
        }
        Some("--expanded-family-frontier-label-shard") => {
            let limit_per_family = parse_expanded_family_frontier_limit(args);
            print!(
                "{}",
                dataset_label::expanded_family_frontier_audited_shard_jsonl(limit_per_family)
                    .expect("expanded family frontier audited shard must serialize")
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

fn parse_family_frontier_limit(mut args: impl Iterator<Item = String>) -> usize {
    match args.next().as_deref() {
        None => dataset_label::DEFAULT_FAMILY_FRONTIER_LIMIT_PER_FAMILY,
        Some("--limit-per-family") => {
            let value = args.next().unwrap_or_else(|| {
                eprintln!("--limit-per-family requires a positive integer");
                std::process::exit(2);
            });
            if let Some(extra) = args.next() {
                eprintln!("unsupported argument for --family-frontier-label-shard: {extra}");
                std::process::exit(2);
            }
            value.parse::<usize>().unwrap_or_else(|error| {
                eprintln!("invalid --limit-per-family: {error}");
                std::process::exit(2);
            })
        }
        Some(arg) => {
            eprintln!("unsupported argument for --family-frontier-label-shard: {arg}");
            std::process::exit(2);
        }
    }
}

fn parse_expanded_family_frontier_limit(mut args: impl Iterator<Item = String>) -> usize {
    match args.next().as_deref() {
        None => dataset_label::DEFAULT_EXPANDED_FAMILY_FRONTIER_LIMIT_PER_FAMILY,
        Some("--limit-per-family") => {
            let value = args.next().unwrap_or_else(|| {
                eprintln!("--limit-per-family requires a positive integer");
                std::process::exit(2);
            });
            if let Some(extra) = args.next() {
                eprintln!(
                    "unsupported argument for --expanded-family-frontier-label-shard: {extra}"
                );
                std::process::exit(2);
            }
            value.parse::<usize>().unwrap_or_else(|error| {
                eprintln!("invalid --limit-per-family: {error}");
                std::process::exit(2);
            })
        }
        Some(arg) => {
            eprintln!("unsupported argument for --expanded-family-frontier-label-shard: {arg}");
            std::process::exit(2);
        }
    }
}

fn parse_frontier_limit(mut args: impl Iterator<Item = String>) -> usize {
    match args.next().as_deref() {
        None => dataset_label::DEFAULT_FRONTIER_SHARD_LIMIT,
        Some("--limit") => {
            let value = args.next().unwrap_or_else(|| {
                eprintln!("--limit requires a positive integer");
                std::process::exit(2);
            });
            if let Some(extra) = args.next() {
                eprintln!("unsupported argument for --frontier-label-shard: {extra}");
                std::process::exit(2);
            }
            value.parse::<usize>().unwrap_or_else(|error| {
                eprintln!("invalid --limit: {error}");
                std::process::exit(2);
            })
        }
        Some(arg) => {
            eprintln!("unsupported argument for --frontier-label-shard: {arg}");
            std::process::exit(2);
        }
    }
}
