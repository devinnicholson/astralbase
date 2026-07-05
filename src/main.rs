use astralbase::{GameValue, RetrogradeEngine, dataset_label};
use shakmaty::{CastlingMode, Chess, fen::Fen};
use std::str::FromStr;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--help" | "-h") => {
            print_help();
            return;
        }
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
        Some("--composition-hard-target-shard") => {
            let limit = parse_composition_hard_target_limit(args);
            print!(
                "{}",
                dataset_label::composition_hard_target_shard_jsonl(limit)
                    .expect("composition hard-target shard must serialize")
            );
            return;
        }
        Some("--non-fixture-composed-domain-shard") => {
            let limit = parse_non_fixture_composed_domain_limit(args);
            print!(
                "{}",
                dataset_label::non_fixture_composed_domain_shard_jsonl(limit)
                    .expect("non-fixture composed-domain shard must serialize")
            );
            return;
        }
        Some("--expanded-non-fixture-composed-domain-shard") => {
            let rows_per_family = parse_expanded_non_fixture_composed_domain_rows(args);
            print!(
                "{}",
                dataset_label::expanded_non_fixture_composed_domain_shard_jsonl(rows_per_family)
                    .expect("expanded non-fixture composed-domain shard must serialize")
            );
            return;
        }
        Some("--leakage-clean-non-fixture-composed-domain-shard") => {
            let rows_per_family = parse_leakage_clean_non_fixture_composed_domain_rows(args);
            print!(
                "{}",
                dataset_label::leakage_clean_non_fixture_composed_domain_shard_jsonl(
                    rows_per_family
                )
                .expect("leakage-clean non-fixture composed-domain shard must serialize")
            );
            return;
        }
        Some("--replay-non-fixture-composed-domain-shard") => {
            let path = parse_replay_non_fixture_composed_domain_path(args);
            let input = std::fs::read_to_string(path.as_str()).unwrap_or_else(|error| {
                eprintln!("could not read {path}: {error}");
                std::process::exit(2);
            });
            match dataset_label::replay_verify_non_fixture_composed_domain_jsonl(input.as_str()) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&report)
                            .expect("replay report must serialize")
                    );
                }
                Err(issues) => {
                    for issue in issues {
                        match issue.line_number {
                            Some(line_number) => eprintln!("line {line_number}: {}", issue.message),
                            None => eprintln!("{}", issue.message),
                        }
                    }
                    std::process::exit(1);
                }
            }
            return;
        }
        Some("--generated-depth-two-profile-search") => {
            let rows_per_family = parse_generated_depth_two_profile_search_rows(args);
            let report = dataset_label::generated_depth_two_profile_search_report(rows_per_family);
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("generated depth-two profile search report must serialize")
            );
            return;
        }
        Some("--generated-depth-two-combined-source-profile-search") => {
            let rows_per_family =
                parse_generated_depth_two_combined_source_profile_search_rows(args);
            let report = dataset_label::generated_depth_two_combined_source_profile_search_report(
                rows_per_family,
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect(
                    "generated depth-two combined-source profile search report must serialize"
                )
            );
            return;
        }
        Some("--generated-depth-two-profile-inventory") => {
            let report = dataset_label::generated_depth_two_profile_inventory_report();
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("generated depth-two profile inventory report must serialize")
            );
            return;
        }
        Some("--generated-depth-two-profile-source-inventory") => {
            let report = dataset_label::generated_depth_two_profile_source_inventory_report();
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("generated depth-two profile source inventory report must serialize")
            );
            return;
        }
        Some("--generated-depth-three-profile-inventory") => {
            let report = dataset_label::generated_depth_three_profile_inventory_report();
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("generated depth-three profile inventory report must serialize")
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

fn print_help() {
    println!(
        "Usage: astralbase [COMMAND]\n\
\n\
Commands:\n\
  --sample-label-shard\n\
  --frontier-label-shard [--limit N]\n\
  --family-frontier-label-shard [--limit-per-family N]\n\
  --expanded-family-frontier-label-shard [--limit-per-family N]\n\
  --composition-hard-target-shard [--limit N]\n\
  --non-fixture-composed-domain-shard [--limit N]\n\
  --expanded-non-fixture-composed-domain-shard [--rows-per-family N]\n\
  --leakage-clean-non-fixture-composed-domain-shard [--rows-per-family N]\n\
  --replay-non-fixture-composed-domain-shard PATH\n\
  --generated-depth-two-profile-search [--rows-per-family N]\n\
  --generated-depth-two-combined-source-profile-search [--rows-per-family N]\n\
  --generated-depth-two-profile-inventory\n\
  --generated-depth-two-profile-source-inventory\n\
  --generated-depth-three-profile-inventory\n\
  --help\n"
    );
}

fn parse_generated_depth_two_profile_search_rows(mut args: impl Iterator<Item = String>) -> usize {
    parse_rows_per_family_arg(&mut args, "--generated-depth-two-profile-search", 10)
}

fn parse_generated_depth_two_combined_source_profile_search_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--generated-depth-two-combined-source-profile-search",
        10,
    )
}

fn parse_expanded_non_fixture_composed_domain_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--expanded-non-fixture-composed-domain-shard",
        dataset_label::DEFAULT_EXPANDED_NON_FIXTURE_COMPOSED_DOMAIN_ROWS_PER_FAMILY,
    )
}

fn parse_leakage_clean_non_fixture_composed_domain_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--leakage-clean-non-fixture-composed-domain-shard",
        dataset_label::DEFAULT_LEAKAGE_CLEAN_NON_FIXTURE_COMPOSED_DOMAIN_ROWS_PER_FAMILY,
    )
}

fn parse_rows_per_family_arg(
    args: &mut impl Iterator<Item = String>,
    command: &str,
    default: usize,
) -> usize {
    match args.next().as_deref() {
        None => default,
        Some("--rows-per-family") => {
            let value = args.next().unwrap_or_else(|| {
                eprintln!("--rows-per-family requires a positive integer");
                std::process::exit(2);
            });
            if let Some(extra) = args.next() {
                eprintln!("unsupported argument for {command}: {extra}");
                std::process::exit(2);
            }
            value.parse::<usize>().unwrap_or_else(|error| {
                eprintln!("invalid --rows-per-family: {error}");
                std::process::exit(2);
            })
        }
        Some(arg) => {
            eprintln!("unsupported argument for {command}: {arg}");
            std::process::exit(2);
        }
    }
}

fn parse_replay_non_fixture_composed_domain_path(mut args: impl Iterator<Item = String>) -> String {
    let path = args.next().unwrap_or_else(|| {
        eprintln!("--replay-non-fixture-composed-domain-shard requires a JSONL path");
        std::process::exit(2);
    });
    if let Some(extra) = args.next() {
        eprintln!("unsupported argument for --replay-non-fixture-composed-domain-shard: {extra}");
        std::process::exit(2);
    }
    path
}

fn parse_non_fixture_composed_domain_limit(mut args: impl Iterator<Item = String>) -> usize {
    match args.next().as_deref() {
        None => dataset_label::DEFAULT_NON_FIXTURE_COMPOSED_DOMAIN_SHARD_LIMIT,
        Some("--limit") => {
            let value = args.next().unwrap_or_else(|| {
                eprintln!("--limit requires a positive integer");
                std::process::exit(2);
            });
            if let Some(extra) = args.next() {
                eprintln!("unsupported argument for --non-fixture-composed-domain-shard: {extra}");
                std::process::exit(2);
            }
            value.parse::<usize>().unwrap_or_else(|error| {
                eprintln!("invalid --limit: {error}");
                std::process::exit(2);
            })
        }
        Some(arg) => {
            eprintln!("unsupported argument for --non-fixture-composed-domain-shard: {arg}");
            std::process::exit(2);
        }
    }
}

fn parse_composition_hard_target_limit(mut args: impl Iterator<Item = String>) -> usize {
    match args.next().as_deref() {
        None => dataset_label::DEFAULT_COMPOSITION_HARD_TARGET_SHARD_LIMIT,
        Some("--limit") => {
            let value = args.next().unwrap_or_else(|| {
                eprintln!("--limit requires a positive integer");
                std::process::exit(2);
            });
            if let Some(extra) = args.next() {
                eprintln!("unsupported argument for --composition-hard-target-shard: {extra}");
                std::process::exit(2);
            }
            value.parse::<usize>().unwrap_or_else(|error| {
                eprintln!("invalid --limit: {error}");
                std::process::exit(2);
            })
        }
        Some(arg) => {
            eprintln!("unsupported argument for --composition-hard-target-shard: {arg}");
            std::process::exit(2);
        }
    }
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
