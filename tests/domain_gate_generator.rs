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
        14
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
        9
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
            ("profiled_depth2_component_pair_generator_v0".to_owned(), 3),
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
            ("dfile_two_component_depth2_asymmetric_fan_v0".to_owned(), 3,),
            ("dfile_two_component_depth2_local_move_v0".to_owned(), 3),
            ("dfile_two_component_depth2_pawn_phalanx_v0".to_owned(), 3,),
        ])
    );

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
