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
        Some("--signature-target-diagnostic-shard") => {
            let rows_per_family = parse_signature_target_diagnostic_rows(args);
            print!(
                "{}",
                dataset_label::signature_target_diagnostic_shard_jsonl(rows_per_family)
                    .expect("signature target diagnostic shard must serialize")
            );
            return;
        }
        Some("--signature-target-exact-shard") => {
            let rows_per_family = parse_signature_target_exact_rows(args);
            print!(
                "{}",
                dataset_label::signature_target_exact_shard_jsonl(rows_per_family)
                    .expect("signature target exact shard must serialize")
            );
            return;
        }
        Some("--signature-target-mixed-hook-exact-shard") => {
            let rows_per_family = parse_signature_target_mixed_hook_exact_rows(args);
            print!(
                "{}",
                dataset_label::signature_target_mixed_hook_exact_shard_jsonl(rows_per_family)
                    .expect("signature target mixed-hook exact shard must serialize")
            );
            return;
        }
        Some("--signature-target-replay-preflight") => {
            let path = parse_signature_target_replay_preflight_path(args);
            let input = std::fs::read_to_string(path.as_str()).unwrap_or_else(|error| {
                eprintln!("could not read {path}: {error}");
                std::process::exit(2);
            });
            match dataset_label::signature_target_replay_preflight_jsonl(input.as_str()) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&report)
                            .expect("signature target replay preflight report must serialize")
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
        Some("--generated-depth-two-signature-profile-search") => {
            let rows_per_family = parse_generated_depth_two_signature_profile_search_rows(args);
            let report =
                dataset_label::generated_depth_two_signature_profile_search_report(rows_per_family);
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("generated depth-two signature profile search report must serialize")
            );
            return;
        }
        Some("--generated-depth-two-value-unique-signature-profile-search") => {
            let rows_per_family =
                parse_generated_depth_two_value_unique_signature_profile_search_rows(args);
            let report =
                dataset_label::generated_depth_two_value_unique_signature_profile_search_report(
                    rows_per_family,
                );
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect(
                    "generated depth-two value-unique signature profile search report must serialize",
                )
            );
            return;
        }
        Some("--generated-depth-two-value-unique-signature-upper-bound") => {
            let rows_per_family =
                parse_generated_depth_two_value_unique_signature_upper_bound_rows(args);
            let report =
                dataset_label::generated_depth_two_value_unique_signature_upper_bound_report(
                    rows_per_family,
                );
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect(
                    "generated depth-two value-unique signature upper-bound report must serialize",
                )
            );
            return;
        }
        Some("--generated-depth-two-value-unique-signature-source-sweep") => {
            let rows_per_family =
                parse_generated_depth_two_value_unique_signature_source_sweep_rows(args);
            let report =
                dataset_label::generated_depth_two_value_unique_signature_source_sweep_report(
                    rows_per_family,
                );
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect(
                    "generated depth-two value-unique signature source-sweep report must serialize",
                )
            );
            return;
        }
        Some("--generated-depth-two-value-unique-signature-source-atlas") => {
            let rows_per_family =
                parse_generated_depth_two_value_unique_signature_source_atlas_rows(args);
            let report =
                dataset_label::generated_depth_two_value_unique_signature_source_atlas_report(
                    rows_per_family,
                );
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect(
                    "generated depth-two value-unique signature source-atlas report must serialize",
                )
            );
            return;
        }
        Some("--generated-depth-two-value-unique-signature-mixed-hook-upper-bound") => {
            let rows_per_family =
                parse_generated_depth_two_value_unique_signature_mixed_hook_upper_bound_rows(args);
            let report =
                dataset_label::generated_depth_two_value_unique_signature_mixed_hook_upper_bound_report(
                    rows_per_family,
                );
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect(
                    "generated depth-two value-unique signature mixed-hook upper-bound report must serialize",
                )
            );
            return;
        }
        Some("--generated-depth-two-value-unique-signature-expanded-mixed-hook-upper-bound") => {
            let rows_per_family =
                parse_generated_depth_two_value_unique_signature_expanded_mixed_hook_upper_bound_rows(args);
            let report =
                dataset_label::generated_depth_two_value_unique_signature_expanded_mixed_hook_upper_bound_report(
                    rows_per_family,
                );
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect(
                    "generated depth-two value-unique signature expanded mixed-hook upper-bound report must serialize",
                )
            );
            return;
        }
        Some("--generated-depth-two-value-unique-signature-interior-mixed-hook-source") => {
            let rows_per_family =
                parse_generated_depth_two_value_unique_signature_interior_mixed_hook_source_rows(
                    args,
                );
            let report =
                dataset_label::generated_depth_two_value_unique_signature_interior_mixed_hook_source_report(
                    rows_per_family,
                );
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect(
                    "generated depth-two value-unique signature interior mixed-hook source report must serialize",
                )
            );
            return;
        }
        Some("--generated-depth-two-value-unique-signature-pattern-limit-atlas") => {
            let rows_per_family =
                parse_generated_depth_two_value_unique_signature_pattern_limit_atlas_rows(args);
            let report =
                dataset_label::generated_depth_two_value_unique_signature_pattern_limit_atlas_report(
                    rows_per_family,
                );
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect(
                    "generated depth-two value-unique signature pattern-limit atlas report must serialize",
                )
            );
            return;
        }
        Some("--generated-depth-two-value-unique-signature-left-supply-atlas") => {
            let rows_per_family =
                parse_generated_depth_two_value_unique_signature_left_supply_atlas_rows(args);
            let report =
                dataset_label::generated_depth_two_value_unique_signature_left_supply_atlas_report(
                    rows_per_family,
                );
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect(
                    "generated depth-two value-unique signature left-supply atlas report must serialize",
                )
            );
            return;
        }
        Some("--generated-depth-two-value-unique-signature-left-supply-bounded-selection") => {
            let (rows_per_family, candidate_pair_limit) =
                parse_generated_depth_two_value_unique_signature_left_supply_bounded_selection_args(
                    args,
                );
            let report =
                dataset_label::generated_depth_two_value_unique_signature_left_supply_bounded_selection_report(
                    rows_per_family,
                    candidate_pair_limit,
                );
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect(
                    "generated depth-two value-unique signature left-supply bounded-selection report must serialize",
                )
            );
            return;
        }
        Some("--generated-depth-two-signature-bounded-support") => {
            let (rows_per_family, candidate_pair_limit) =
                parse_generated_depth_two_signature_bounded_support_args(args);
            let report = dataset_label::generated_depth_two_signature_bounded_support_report(
                rows_per_family,
                candidate_pair_limit,
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("generated depth-two signature bounded support report must serialize")
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
        Some("--generated-depth-two-duplicate-clusters") => {
            let report = dataset_label::generated_depth_two_duplicate_cluster_report();
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("generated depth-two duplicate cluster report must serialize")
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
  --signature-target-diagnostic-shard [--rows-per-family N]\n\
  --signature-target-exact-shard [--rows-per-family N]\n\
  --signature-target-mixed-hook-exact-shard [--rows-per-family N]\n\
  --signature-target-replay-preflight PATH\n\
  --replay-non-fixture-composed-domain-shard PATH\n\
  --generated-depth-two-profile-search [--rows-per-family N]\n\
  --generated-depth-two-combined-source-profile-search [--rows-per-family N]\n\
  --generated-depth-two-signature-profile-search [--rows-per-family N]\n\
  --generated-depth-two-value-unique-signature-profile-search [--rows-per-family N]\n\
  --generated-depth-two-value-unique-signature-upper-bound [--rows-per-family N]\n\
  --generated-depth-two-value-unique-signature-source-sweep [--rows-per-family N]\n\
  --generated-depth-two-value-unique-signature-source-atlas [--rows-per-family N]\n\
  --generated-depth-two-value-unique-signature-mixed-hook-upper-bound [--rows-per-family N]\n\
  --generated-depth-two-value-unique-signature-expanded-mixed-hook-upper-bound [--rows-per-family N]\n\
  --generated-depth-two-value-unique-signature-interior-mixed-hook-source [--rows-per-family N]\n\
  --generated-depth-two-value-unique-signature-pattern-limit-atlas [--rows-per-family N]\n\
  --generated-depth-two-value-unique-signature-left-supply-atlas [--rows-per-family N]\n\
  --generated-depth-two-value-unique-signature-left-supply-bounded-selection [--rows-per-family N] [--candidate-pair-limit N]\n\
  --generated-depth-two-signature-bounded-support [--rows-per-family N] [--candidate-pair-limit N]\n\
  --generated-depth-two-profile-inventory\n\
  --generated-depth-two-profile-source-inventory\n\
  --generated-depth-two-duplicate-clusters\n\
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

fn parse_generated_depth_two_signature_profile_search_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--generated-depth-two-signature-profile-search",
        10,
    )
}

fn parse_generated_depth_two_value_unique_signature_profile_search_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--generated-depth-two-value-unique-signature-profile-search",
        dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY,
    )
}

fn parse_generated_depth_two_value_unique_signature_upper_bound_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--generated-depth-two-value-unique-signature-upper-bound",
        dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY,
    )
}

fn parse_generated_depth_two_value_unique_signature_source_sweep_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--generated-depth-two-value-unique-signature-source-sweep",
        dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY,
    )
}

fn parse_generated_depth_two_value_unique_signature_source_atlas_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--generated-depth-two-value-unique-signature-source-atlas",
        dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY,
    )
}

fn parse_generated_depth_two_value_unique_signature_mixed_hook_upper_bound_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--generated-depth-two-value-unique-signature-mixed-hook-upper-bound",
        dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY,
    )
}

fn parse_generated_depth_two_value_unique_signature_expanded_mixed_hook_upper_bound_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--generated-depth-two-value-unique-signature-expanded-mixed-hook-upper-bound",
        dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY,
    )
}

fn parse_generated_depth_two_value_unique_signature_interior_mixed_hook_source_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--generated-depth-two-value-unique-signature-interior-mixed-hook-source",
        dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY,
    )
}

fn parse_generated_depth_two_value_unique_signature_pattern_limit_atlas_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--generated-depth-two-value-unique-signature-pattern-limit-atlas",
        dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY,
    )
}

fn parse_generated_depth_two_value_unique_signature_left_supply_atlas_rows(
    mut args: impl Iterator<Item = String>,
) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--generated-depth-two-value-unique-signature-left-supply-atlas",
        dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY,
    )
}

fn parse_generated_depth_two_value_unique_signature_left_supply_bounded_selection_args(
    mut args: impl Iterator<Item = String>,
) -> (usize, usize) {
    let command = "--generated-depth-two-value-unique-signature-left-supply-bounded-selection";
    let mut rows_per_family = dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY;
    let mut candidate_pair_limit = 2_500usize;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--rows-per-family" => {
                rows_per_family =
                    parse_positive_usize_arg(args.next(), "--rows-per-family", command);
            }
            "--candidate-pair-limit" => {
                candidate_pair_limit =
                    parse_positive_usize_arg(args.next(), "--candidate-pair-limit", command);
            }
            _ => {
                eprintln!("unsupported argument for {command}: {arg}");
                std::process::exit(2);
            }
        }
    }
    (rows_per_family, candidate_pair_limit)
}

fn parse_generated_depth_two_signature_bounded_support_args(
    mut args: impl Iterator<Item = String>,
) -> (usize, usize) {
    let command = "--generated-depth-two-signature-bounded-support";
    let mut rows_per_family = 20usize;
    let mut candidate_pair_limit = 2_500usize;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--rows-per-family" => {
                rows_per_family =
                    parse_positive_usize_arg(args.next(), "--rows-per-family", command);
            }
            "--candidate-pair-limit" => {
                candidate_pair_limit =
                    parse_positive_usize_arg(args.next(), "--candidate-pair-limit", command);
            }
            _ => {
                eprintln!("unsupported argument for {command}: {arg}");
                std::process::exit(2);
            }
        }
    }
    (rows_per_family, candidate_pair_limit)
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

fn parse_signature_target_diagnostic_rows(mut args: impl Iterator<Item = String>) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--signature-target-diagnostic-shard",
        dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY,
    )
}

fn parse_signature_target_exact_rows(mut args: impl Iterator<Item = String>) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--signature-target-exact-shard",
        dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY,
    )
}

fn parse_signature_target_mixed_hook_exact_rows(mut args: impl Iterator<Item = String>) -> usize {
    parse_rows_per_family_arg(
        &mut args,
        "--signature-target-mixed-hook-exact-shard",
        dataset_label::DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY,
    )
}

fn parse_positive_usize_arg(value: Option<String>, flag: &str, command: &str) -> usize {
    let value = value.unwrap_or_else(|| {
        eprintln!("{flag} requires a positive integer");
        std::process::exit(2);
    });
    let parsed = value.parse::<usize>().unwrap_or_else(|error| {
        eprintln!("invalid {flag} for {command}: {error}");
        std::process::exit(2);
    });
    if parsed == 0 {
        eprintln!("{flag} requires a positive integer");
        std::process::exit(2);
    }
    parsed
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

fn parse_signature_target_replay_preflight_path(mut args: impl Iterator<Item = String>) -> String {
    let path = args.next().unwrap_or_else(|| {
        eprintln!("--signature-target-replay-preflight requires a JSONL path");
        std::process::exit(2);
    });
    if let Some(extra) = args.next() {
        eprintln!("unsupported argument for --signature-target-replay-preflight: {extra}");
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
