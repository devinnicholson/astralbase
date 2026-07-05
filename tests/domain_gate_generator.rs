use astralbase::{
    dataset_label::{self, LabelKind, LabelPayload, RejectedStatus},
    domain::{self, DomainRejectionCode, TerminalStatus},
};

#[test]
fn public_domain_gate_accepts_terminal_checkmate() {
    let validated =
        domain::validate_first_constrained_fen("7k/5KQ1/8/8/8/8/8/8 b - - 0 1").unwrap();

    assert_eq!(validated.terminal_status(), Some(TerminalStatus::Checkmate));
}

#[test]
fn public_domain_gate_rejects_castling_rights() {
    let report =
        domain::validate_first_constrained_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap_err();

    assert_eq!(
        report.reasons()[0].code,
        DomainRejectionCode::CastlingRights
    );
}

#[test]
fn generated_jsonl_keeps_unsupported_positions_out_of_exact_rows() {
    let jsonl = dataset_label::sample_audited_shard_jsonl().unwrap();
    let rows = dataset_label::parse_and_validate_jsonl(&jsonl).unwrap();

    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].label_kind(), LabelKind::Exact);
    assert_eq!(rows[1].label_kind(), LabelKind::Exact);
    assert_eq!(rows[2].label_kind(), LabelKind::Exact);
    assert!(
        rows[3..]
            .iter()
            .all(|row| row.label_kind() == LabelKind::Rejected)
    );

    for row in &rows[3..] {
        let LabelPayload::Rejected { rejected } = &row.label else {
            panic!("unsupported generated rows must be rejected rows");
        };
        assert!(!rejected.reasons.is_empty());
    }
}

#[test]
fn non_fixture_composed_domain_jsonl_has_exact_board_rows_and_rejected_chess_rows() {
    let jsonl = dataset_label::non_fixture_composed_domain_shard_jsonl(
        dataset_label::DEFAULT_NON_FIXTURE_COMPOSED_DOMAIN_SHARD_LIMIT,
    )
    .unwrap();
    let rows = dataset_label::parse_and_validate_jsonl(&jsonl).unwrap();

    assert_eq!(
        rows.len(),
        dataset_label::DEFAULT_NON_FIXTURE_COMPOSED_DOMAIN_SHARD_LIMIT
    );
    assert_eq!(
        dataset_label::non_fixture_composed_domain_shard(50).len(),
        dataset_label::DEFAULT_NON_FIXTURE_COMPOSED_DOMAIN_SHARD_LIMIT
    );
    assert!(rows.iter().all(|row| {
        !row.domain.contains("fixture")
            && (row
                .row_id
                .starts_with(dataset_label::NON_FIXTURE_COMPOSED_DOMAIN_SHARD_CONFIG.row_id_prefix)
                || row.row_id.starts_with(
                    dataset_label::NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG.row_id_prefix,
                ))
    }));
    assert_eq!(
        rows.iter()
            .filter(
                |row| row.domain == dataset_label::NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID
                    && row.label_kind() == LabelKind::Exact
            )
            .count(),
        17
    );
    assert_eq!(
        rows.iter()
            .filter(
                |row| row.domain == dataset_label::NON_FIXTURE_COMPOSED_DOMAIN_ID
                    && row.label_kind() == LabelKind::Rejected
            )
            .count(),
        3
    );
    assert_eq!(
        rows.iter()
            .filter(|row| {
                matches!(
                    &row.label,
                    LabelPayload::Exact { exact, .. }
                        if exact
                            .value
                            .get("composition_value_rule")
                            .map(String::as_str)
                            == Some("component_agency_atom_sum_v0")
                            && exact.value_class == dataset_label::ExactValueClass::GameTree
                )
            })
            .count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|row| {
                matches!(
                    &row.label,
                    LabelPayload::Exact { exact, .. }
                        if exact
                            .value
                            .get("composition_value_rule")
                            .map(String::as_str)
                            == Some("component_local_move_game_v0")
                            && exact.value_class == dataset_label::ExactValueClass::GameTree
                )
            })
            .count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|row| {
                matches!(
                    &row.label,
                    LabelPayload::Exact { exact, .. }
                        if exact
                            .value
                            .get("composition_value_rule")
                            .map(String::as_str)
                            == Some("component_depth2_local_move_game_v0")
                            && exact.value_class == dataset_label::ExactValueClass::GameTree
                )
            })
            .count(),
        12
    );
    assert_eq!(
        rows.iter()
            .filter(|row| {
                row.row_id.starts_with(
                    dataset_label::NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG.row_id_prefix,
                ) && row.row_id.as_str() >= "astralbase-w18-non-fixture-composed-board-012"
                    && row.label_kind() == LabelKind::Exact
            })
            .count(),
        6
    );
    let spec_source_counts = rows
        .iter()
        .filter_map(|row| match &row.label {
            LabelPayload::Exact { exact, .. } => {
                exact.value.get("composition_spec_source").cloned()
            }
            _ => None,
        })
        .fold(
            std::collections::BTreeMap::<String, usize>::new(),
            |mut counts, source| {
                *counts.entry(source).or_default() += 1;
                counts
            },
        );
    assert_eq!(
        spec_source_counts,
        std::collections::BTreeMap::from([
            ("curated_non_fixture_board_spec_v0".to_owned(), 11),
            ("profiled_depth2_component_pair_generator_v0".to_owned(), 6),
        ])
    );
    let depth_two_family_counts = rows
        .iter()
        .filter_map(|row| match &row.label {
            LabelPayload::Exact { exact, .. }
                if exact
                    .value
                    .get("composition_value_rule")
                    .map(String::as_str)
                    == Some("component_depth2_local_move_game_v0") =>
            {
                exact.value.get("component_topology_family").cloned()
            }
            _ => None,
        })
        .fold(
            std::collections::BTreeMap::<String, usize>::new(),
            |mut counts, family| {
                *counts.entry(family).or_default() += 1;
                counts
            },
        );
    assert_eq!(
        depth_two_family_counts,
        std::collections::BTreeMap::from([
            ("dfile_two_component_depth2_asymmetric_fan_v0".to_owned(), 4,),
            ("dfile_two_component_depth2_local_move_v0".to_owned(), 4),
            ("dfile_two_component_depth2_pawn_phalanx_v0".to_owned(), 4,),
        ])
    );
    let generated_depth_two_family_counts = rows
        .iter()
        .filter_map(|row| match &row.label {
            LabelPayload::Exact { exact, .. }
                if exact
                    .value
                    .get("composition_spec_source")
                    .map(String::as_str)
                    == Some("profiled_depth2_component_pair_generator_v0") =>
            {
                exact.value.get("component_topology_family").cloned()
            }
            _ => None,
        })
        .fold(
            std::collections::BTreeMap::<String, usize>::new(),
            |mut counts, family| {
                *counts.entry(family).or_default() += 1;
                counts
            },
        );
    assert_eq!(
        generated_depth_two_family_counts,
        std::collections::BTreeMap::from([
            ("dfile_two_component_depth2_asymmetric_fan_v0".to_owned(), 2,),
            ("dfile_two_component_depth2_local_move_v0".to_owned(), 2),
            ("dfile_two_component_depth2_pawn_phalanx_v0".to_owned(), 2,),
        ])
    );

    let mut positions = std::collections::BTreeSet::new();
    let mut decomposition_digests = std::collections::BTreeSet::new();
    let mut composition_digests = std::collections::BTreeSet::new();
    let mut result_digests = std::collections::BTreeSet::new();
    let mut component_value_digests = std::collections::BTreeSet::new();
    for row in rows
        .iter()
        .filter(|row| row.label_kind() == LabelKind::Exact)
    {
        assert!(positions.insert(row.position.text.clone()));
        let LabelPayload::Exact { provenance, .. } = &row.label else {
            unreachable!("exact rows are filtered above");
        };
        let composition = provenance
            .certificate
            .composition
            .as_ref()
            .expect("exact composition row carries structured certificate");
        assert!(decomposition_digests.insert(composition.decomposition_digest.clone()));
        assert!(composition_digests.insert(composition.composition_digest.clone()));
        assert!(result_digests.insert(composition.result_value_digest.clone()));
        let mut row_component_value_digests = std::collections::BTreeSet::new();
        for value_digest in composition.component_values.values() {
            assert!(row_component_value_digests.insert(value_digest.clone()));
            assert!(component_value_digests.insert(value_digest.clone()));
        }
    }

    for row in rows {
        match row.label {
            LabelPayload::Exact { exact, provenance } => {
                assert_eq!(
                    row.domain,
                    dataset_label::NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID
                );
                let composition_value_rule = exact
                    .value
                    .get("composition_value_rule")
                    .map(String::as_str);
                assert!(
                    exact
                        .value
                        .get("component_topology_family")
                        .is_some_and(|family| !family.is_empty())
                );
                assert!(matches!(
                    exact
                        .value
                        .get("composition_spec_source")
                        .map(String::as_str),
                    Some("curated_non_fixture_board_spec_v0")
                        | Some("profiled_depth2_component_pair_generator_v0")
                ));
                assert!(
                    exact
                        .value
                        .get("component_local_move_totals")
                        .is_some_and(
                            |totals| totals.starts_with("white:") && totals.contains(",black:")
                        )
                );
                assert!(
                    exact
                        .value
                        .get("component_local_move_imbalance")
                        .and_then(|imbalance| imbalance.parse::<i64>().ok())
                        .is_some()
                );
                assert!(matches!(
                    composition_value_rule,
                    Some("component_material_balance_sum_v0")
                        | Some("component_agency_atom_sum_v0")
                        | Some("component_local_move_game_v0")
                        | Some("component_depth2_local_move_game_v0")
                ));
                if composition_value_rule == Some("component_agency_atom_sum_v0") {
                    assert_eq!(exact.value_class, dataset_label::ExactValueClass::GameTree);
                    let component_value_classes = exact
                        .value
                        .get("component_value_classes")
                        .expect("agency atom row records component value classes");
                    assert!(component_value_classes.contains("=up"));
                    assert!(component_value_classes.contains("=down"));
                }
                if composition_value_rule == Some("component_local_move_game_v0") {
                    assert_eq!(exact.value_class, dataset_label::ExactValueClass::GameTree);
                    let component_local_move_counts = exact
                        .value
                        .get("component_local_move_counts")
                        .expect("local move row records component move counts");
                    assert!(component_local_move_counts.contains("white:"));
                    assert!(component_local_move_counts.contains("black:"));
                }
                if composition_value_rule == Some("component_depth2_local_move_game_v0") {
                    assert_eq!(exact.value_class, dataset_label::ExactValueClass::GameTree);
                    assert_eq!(
                        exact.value.get("solver_depth").map(String::as_str),
                        Some("2")
                    );
                    assert!(matches!(
                        exact
                            .value
                            .get("component_topology_family")
                            .map(String::as_str),
                        Some("dfile_two_component_depth2_local_move_v0")
                            | Some("dfile_two_component_depth2_asymmetric_fan_v0")
                            | Some("dfile_two_component_depth2_pawn_phalanx_v0")
                    ));
                    assert_eq!(
                        exact.value.get("recursive_leaf_rule").map(String::as_str),
                        Some("component_material_balance_at_depth_cutoff_or_no_moves_v0")
                    );
                    let recursive_node_counts = exact
                        .value
                        .get("component_recursive_node_counts")
                        .expect("depth-2 row records component recursive node counts");
                    assert!(recursive_node_counts.contains("depth:2"));
                    assert!(
                        exact
                            .value
                            .get("component_recursive_total_nodes")
                            .and_then(|nodes| nodes.parse::<usize>().ok())
                            .is_some_and(|nodes| nodes > 2 && nodes <= 1_000)
                    );
                }
                assert_eq!(
                    exact.value.get("proof_kind").map(String::as_str),
                    Some("bitmesh:conservative_legal_independence:v0")
                );
                assert!(
                    provenance
                        .certificate
                        .composition
                        .as_ref()
                        .is_some_and(|composition| {
                            composition.result_value_digest == *exact.value.get("digest").unwrap()
                        })
                );
            }
            LabelPayload::Rejected { rejected } => {
                assert_eq!(row.domain, dataset_label::NON_FIXTURE_COMPOSED_DOMAIN_ID);
                assert_eq!(rejected.status, RejectedStatus::Unsupported);
                assert!(
                    rejected
                        .reasons
                        .iter()
                        .any(|reason| reason.starts_with("unsupported_non_fixture_composition:"))
                );
                assert!(rejected.reasons.iter().any(|reason| {
                    reason.contains("conservative legal-independence proof")
                        || reason.contains("invalid FEN for conservative legal-independence proof")
                }));
            }
            _ => panic!("non-fixture composed-domain rows must be exact or rejected"),
        }
    }
}

#[test]
fn non_fixture_composed_domain_jsonl_replays_composition_certificates() {
    let jsonl = dataset_label::non_fixture_composed_domain_shard_jsonl(
        dataset_label::DEFAULT_NON_FIXTURE_COMPOSED_DOMAIN_SHARD_LIMIT,
    )
    .unwrap();

    let report = dataset_label::replay_verify_non_fixture_composed_domain_jsonl(&jsonl).unwrap();

    assert_eq!(report.row_count, 20);
    assert_eq!(report.checked_exact_rows, 17);
    assert_eq!(report.skipped_rejected_rows, 3);
    assert_eq!(report.skipped_non_target_rows, 0);
}

#[test]
fn expanded_non_fixture_composed_domain_jsonl_has_profiled_rows_per_topology() {
    let jsonl = dataset_label::expanded_non_fixture_composed_domain_shard_jsonl(
        dataset_label::DEFAULT_EXPANDED_NON_FIXTURE_COMPOSED_DOMAIN_ROWS_PER_FAMILY,
    )
    .unwrap();
    let rows = dataset_label::parse_and_validate_jsonl(&jsonl).unwrap();

    assert_eq!(
        rows.len(),
        dataset_label::DEFAULT_EXPANDED_NON_FIXTURE_COMPOSED_DOMAIN_SHARD_LIMIT
    );
    assert_eq!(
        rows.iter()
            .filter(
                |row| row.domain == dataset_label::NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID
                    && row.label_kind() == LabelKind::Exact
            )
            .count(),
        41
    );
    assert_eq!(
        rows.iter()
            .filter(
                |row| row.domain == dataset_label::NON_FIXTURE_COMPOSED_DOMAIN_ID
                    && row.label_kind() == LabelKind::Rejected
            )
            .count(),
        3
    );

    let spec_source_counts = rows
        .iter()
        .filter_map(|row| match &row.label {
            LabelPayload::Exact { exact, .. } => {
                exact.value.get("composition_spec_source").cloned()
            }
            _ => None,
        })
        .fold(
            std::collections::BTreeMap::<String, usize>::new(),
            |mut counts, source| {
                *counts.entry(source).or_default() += 1;
                counts
            },
        );
    assert_eq!(
        spec_source_counts,
        std::collections::BTreeMap::from([
            ("curated_non_fixture_board_spec_v0".to_owned(), 11),
            ("profiled_depth2_component_pair_generator_v0".to_owned(), 30),
        ])
    );

    let generated_depth_two_family_counts = rows
        .iter()
        .filter_map(|row| match &row.label {
            LabelPayload::Exact { exact, .. }
                if exact
                    .value
                    .get("composition_spec_source")
                    .map(String::as_str)
                    == Some("profiled_depth2_component_pair_generator_v0") =>
            {
                exact.value.get("component_topology_family").cloned()
            }
            _ => None,
        })
        .fold(
            std::collections::BTreeMap::<String, usize>::new(),
            |mut counts, family| {
                *counts.entry(family).or_default() += 1;
                counts
            },
        );
    assert_eq!(
        generated_depth_two_family_counts,
        std::collections::BTreeMap::from([
            (
                "dfile_two_component_depth2_asymmetric_fan_v0".to_owned(),
                10
            ),
            ("dfile_two_component_depth2_local_move_v0".to_owned(), 10),
            ("dfile_two_component_depth2_pawn_phalanx_v0".to_owned(), 10),
        ])
    );

    let report = dataset_label::replay_verify_non_fixture_composed_domain_jsonl(&jsonl).unwrap();
    assert_eq!(report.row_count, 44);
    assert_eq!(report.checked_exact_rows, 41);
    assert_eq!(report.skipped_rejected_rows, 3);
    assert_eq!(report.skipped_non_target_rows, 0);
}

#[test]
fn leakage_clean_non_fixture_composed_domain_jsonl_has_no_reuse_capacity() {
    let jsonl = dataset_label::leakage_clean_non_fixture_composed_domain_shard_jsonl(
        dataset_label::DEFAULT_LEAKAGE_CLEAN_NON_FIXTURE_COMPOSED_DOMAIN_ROWS_PER_FAMILY,
    )
    .unwrap();
    let rows = dataset_label::parse_and_validate_jsonl(&jsonl).unwrap();

    assert_eq!(rows.len(), 21);
    assert_eq!(
        rows.iter()
            .filter(
                |row| row.domain == dataset_label::NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID
                    && row.label_kind() == LabelKind::Exact
            )
            .count(),
        18
    );
    assert_eq!(
        rows.iter()
            .filter(
                |row| row.domain == dataset_label::NON_FIXTURE_COMPOSED_DOMAIN_ID
                    && row.label_kind() == LabelKind::Rejected
            )
            .count(),
        3
    );

    let generated_depth_two_family_counts = rows
        .iter()
        .filter_map(|row| match &row.label {
            LabelPayload::Exact { exact, .. }
                if exact
                    .value
                    .get("composition_spec_source")
                    .map(String::as_str)
                    == Some("profiled_depth2_component_pair_generator_v0") =>
            {
                exact.value.get("component_topology_family").cloned()
            }
            _ => None,
        })
        .fold(
            std::collections::BTreeMap::<String, usize>::new(),
            |mut counts, family| {
                *counts.entry(family).or_default() += 1;
                counts
            },
        );
    assert_eq!(
        generated_depth_two_family_counts,
        std::collections::BTreeMap::from([
            ("dfile_two_component_depth2_asymmetric_fan_v0".to_owned(), 2),
            ("dfile_two_component_depth2_local_move_v0".to_owned(), 3),
            ("dfile_two_component_depth2_pawn_phalanx_v0".to_owned(), 2),
        ])
    );

    let mut positions = std::collections::BTreeSet::new();
    let mut decomposition_digests = std::collections::BTreeSet::new();
    let mut composition_digests = std::collections::BTreeSet::new();
    let mut component_identities = std::collections::BTreeSet::new();
    let mut component_value_digests = std::collections::BTreeSet::new();
    let mut component_value_identities = std::collections::BTreeSet::new();
    let mut result_value_digests = std::collections::BTreeSet::new();
    for row in rows
        .iter()
        .filter(|row| row.label_kind() == LabelKind::Exact)
    {
        assert!(positions.insert(row.position.text.clone()));
        let LabelPayload::Exact { provenance, .. } = &row.label else {
            unreachable!("exact rows are filtered above");
        };
        let composition = provenance
            .certificate
            .composition
            .as_ref()
            .expect("exact composition row carries structured certificate");
        assert!(decomposition_digests.insert(composition.decomposition_digest.clone()));
        assert!(composition_digests.insert(composition.composition_digest.clone()));
        assert!(result_value_digests.insert(composition.result_value_digest.clone()));
        for (component_root, value_digest) in &composition.component_values {
            let component_identity =
                format!("{}:{component_root}", composition.decomposition_digest);
            assert!(component_identities.insert(component_identity.clone()));
            assert!(component_value_digests.insert(value_digest.clone()));
            assert!(
                component_value_identities.insert(format!("{component_identity}={value_digest}"))
            );
        }
    }

    let report = dataset_label::replay_verify_non_fixture_composed_domain_jsonl(&jsonl).unwrap();
    assert_eq!(report.row_count, 21);
    assert_eq!(report.checked_exact_rows, 18);
    assert_eq!(report.skipped_rejected_rows, 3);
    assert_eq!(report.skipped_non_target_rows, 0);
}

#[test]
fn non_fixture_composed_domain_replay_rejects_stale_recomputable_fields() {
    let mut rows = dataset_label::non_fixture_composed_domain_shard(
        dataset_label::DEFAULT_NON_FIXTURE_COMPOSED_DOMAIN_SHARD_LIMIT,
    );
    let exact = rows
        .iter_mut()
        .find_map(|row| match &mut row.label {
            LabelPayload::Exact { exact, .. }
                if row.domain == dataset_label::NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID =>
            {
                Some(exact)
            }
            _ => None,
        })
        .expect("non-fixture shard has exact composed-board rows");
    exact.value.insert(
        "component_roots".to_owned(),
        "stale-root-summary".to_owned(),
    );

    let jsonl = dataset_label::serialize_jsonl(&rows).unwrap();
    let issues =
        dataset_label::replay_verify_non_fixture_composed_domain_jsonl(&jsonl).unwrap_err();

    assert!(issues.iter().any(|issue| {
        issue
            .message
            .contains("exact.value.component_roots replay mismatch")
    }));
}

#[test]
fn generated_depth_two_profile_search_reports_current_capacity() {
    let report = dataset_label::generated_depth_two_profile_search_report(10);

    assert_eq!(report.rows_per_family_target, 10);
    assert_eq!(report.left_profile_count, 14);
    assert_eq!(report.right_profile_count, 13);
    assert_eq!(
        report.candidate_pair_counts_by_topology_family,
        std::collections::BTreeMap::from([
            (
                "dfile_two_component_depth2_asymmetric_fan_v0".to_owned(),
                182
            ),
            ("dfile_two_component_depth2_local_move_v0".to_owned(), 182),
            ("dfile_two_component_depth2_pawn_phalanx_v0".to_owned(), 182),
        ])
    );
    assert_eq!(report.selected_row_count, 7);
    assert_eq!(
        report.selected_counts_by_topology_family,
        std::collections::BTreeMap::from([
            ("dfile_two_component_depth2_asymmetric_fan_v0".to_owned(), 2),
            ("dfile_two_component_depth2_local_move_v0".to_owned(), 3),
            ("dfile_two_component_depth2_pawn_phalanx_v0".to_owned(), 2),
        ])
    );
    assert_eq!(
        report.rejection_counts,
        std::collections::BTreeMap::from([(
            "component_value_digest_reuse_before_materialization".to_owned(),
            539
        )])
    );
}

#[test]
fn generated_depth_two_profile_inventory_reports_profile_collapse() {
    let report = dataset_label::generated_depth_two_profile_inventory_report();

    assert_eq!(report.white.pattern_count, 526);
    assert_eq!(report.white.wall_safe_pattern_count, 129);
    assert_eq!(report.white.accepted_profile_count, 14);
    assert_eq!(
        report.white.rejection_counts,
        std::collections::BTreeMap::from([
            ("component_recursive_node_budget".to_owned(), 6),
            ("duplicate_value_digest".to_owned(), 109),
            ("wall_safety".to_owned(), 397),
        ])
    );
    assert_eq!(
        report
            .white
            .profiles
            .first()
            .map(|profile| profile.active_pieces.as_str()),
        Some("a1N,a2P,b2P")
    );
    assert_eq!(
        report
            .white
            .profiles
            .last()
            .map(|profile| profile.value_digest.as_str()),
        Some("7f770cf2a36c12fa")
    );

    assert_eq!(report.black.pattern_count, 527);
    assert_eq!(report.black.wall_safe_pattern_count, 365);
    assert_eq!(report.black.accepted_profile_count, 13);
    assert_eq!(
        report.black.rejection_counts,
        std::collections::BTreeMap::from([
            ("component_recursive_node_budget".to_owned(), 91),
            ("duplicate_value_digest".to_owned(), 261),
            ("wall_safety".to_owned(), 162),
        ])
    );
    assert_eq!(
        report
            .black
            .profiles
            .first()
            .map(|profile| profile.active_pieces.as_str()),
        Some("h7p,h8n")
    );
    assert_eq!(
        report
            .black
            .profiles
            .last()
            .map(|profile| profile.value_digest.as_str()),
        Some("180b3b4f81e9a743")
    );
}

#[test]
fn generated_depth_three_profile_inventory_reports_recursive_budget_collapse() {
    let report = dataset_label::generated_depth_three_profile_inventory_report();

    assert_eq!(report.component_depth, 3);
    assert_eq!(report.white.pattern_count, 526);
    assert_eq!(report.white.wall_safe_pattern_count, 129);
    assert_eq!(report.white.accepted_profile_count, 6);
    assert_eq!(
        report.white.rejection_counts,
        std::collections::BTreeMap::from([
            ("component_recursive_node_budget".to_owned(), 91),
            ("duplicate_value_digest".to_owned(), 32),
            ("wall_safety".to_owned(), 397),
        ])
    );
    assert_eq!(
        report
            .white
            .profiles
            .first()
            .map(|profile| profile.value_digest.as_str()),
        Some("fba411ddcb9b11ab")
    );
    assert_eq!(
        report
            .white
            .profiles
            .last()
            .map(|profile| profile.value_digest.as_str()),
        Some("d28ab60b0246139f")
    );

    assert_eq!(report.black.pattern_count, 527);
    assert_eq!(report.black.wall_safe_pattern_count, 365);
    assert_eq!(report.black.accepted_profile_count, 4);
    assert_eq!(
        report.black.rejection_counts,
        std::collections::BTreeMap::from([
            ("component_recursive_node_budget".to_owned(), 337),
            ("duplicate_value_digest".to_owned(), 24),
            ("wall_safety".to_owned(), 162),
        ])
    );
    assert_eq!(
        report
            .black
            .profiles
            .first()
            .map(|profile| profile.value_digest.as_str()),
        Some("6a0d959a882bb6c9")
    );
    assert_eq!(
        report
            .black
            .profiles
            .last()
            .map(|profile| profile.value_digest.as_str()),
        Some("e64db62c384d0f52")
    );
}
