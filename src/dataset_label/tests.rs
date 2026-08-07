use super::*;

#[test]
fn sample_audited_shard_is_valid_and_round_trips() {
    let rows = sample_audited_shard();
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].label_kind(), LabelKind::Exact);
    assert_eq!(rows[1].label_kind(), LabelKind::Exact);
    assert_eq!(rows[2].label_kind(), LabelKind::Exact);
    assert_eq!(rows[3].label_kind(), LabelKind::Rejected);
    assert_eq!(rows[4].label_kind(), LabelKind::Rejected);

    for row in &rows {
        validate_dataset_label_row(row).unwrap();
    }

    let jsonl = serialize_jsonl(&rows).unwrap();
    let parsed = parse_and_validate_jsonl(&jsonl).unwrap();
    assert_eq!(parsed, rows);
}

#[test]
fn sample_audited_shard_jsonl_is_deterministic() {
    assert_eq!(
        sample_audited_shard_jsonl().unwrap(),
        sample_audited_shard_jsonl().unwrap()
    );
}

#[test]
fn legacy_certificates_keep_two_field_json_shape() {
    let certificate = LabelCertificate::legacy("legacy-kind", "legacy-digest");

    assert_eq!(
        serde_json::to_value(&certificate).unwrap(),
        serde_json::json!({
            "kind": "legacy-kind",
            "digest": "legacy-digest",
        })
    );

    let sample_jsonl = sample_audited_shard_jsonl().unwrap();
    for structured_field in [
        "decomposition_digest",
        "composition_digest",
        "component_values",
        "result_value_digest",
    ] {
        assert!(
            !sample_jsonl.contains(structured_field),
            "legacy shard JSON unexpectedly emitted {structured_field}"
        );
    }
}

#[test]
fn composition_certificate_serializes_and_round_trips() {
    let component_values = BTreeMap::from([
        (
            "component:king-and-queen-vs-king".to_owned(),
            "thermograph:component-value-a".to_owned(),
        ),
        (
            "component:king-and-rook-vs-king".to_owned(),
            "thermograph:component-value-b".to_owned(),
        ),
    ]);
    let certificate = LabelCertificate::composition(
        "bitmesh-composition+thermograph-exact-value",
        "composition-certificate:root",
        "bitmesh:decomposition-root",
        "bitmesh:composition-root",
        component_values,
        "thermograph:result-value",
    );

    assert_eq!(
        serde_json::to_value(&certificate).unwrap(),
        serde_json::json!({
            "kind": "bitmesh-composition+thermograph-exact-value",
            "digest": "composition-certificate:root",
            "decomposition_digest": "bitmesh:decomposition-root",
            "composition_digest": "bitmesh:composition-root",
            "component_values": {
                "component:king-and-queen-vs-king": "thermograph:component-value-a",
                "component:king-and-rook-vs-king": "thermograph:component-value-b",
            },
            "result_value_digest": "thermograph:result-value",
        })
    );

    let row = DatasetLabelRow::exact(
        "composition-certificate-scaffold",
        FORMAL_CGT_DOMAIN_ID,
        DatasetPosition::cgt_canonical("GameTree(L[Number(1/2^0)];R[])"),
        ExactLabel::verified(
            BTreeMap::from([
                (
                    "canonical_serialization".to_owned(),
                    "GameTree(L[Number(1/2^0)];R[])".to_owned(),
                ),
                ("digest".to_owned(), "thermograph:result-value".to_owned()),
            ]),
            ExactValueClass::GameTree,
        ),
        ExactProvenance {
            code_commit: "workspace".to_owned(),
            generator: "composition_certificate_scaffold_test".to_owned(),
            generator_config_hash: "astralbase:composition-scaffold:v0".to_owned(),
            random_seed: 0,
            domain_definition: FORMAL_CGT_DOMAIN_DEFINITION.to_owned(),
            verifier: "composition_certificate_scaffold_verifier".to_owned(),
            verifier_version: env!("CARGO_PKG_VERSION").to_owned(),
            certificate,
        },
    );

    validate_dataset_label_row(&row).unwrap();
    let jsonl = serialize_jsonl(std::slice::from_ref(&row)).unwrap();
    assert_eq!(parse_and_validate_jsonl(&jsonl).unwrap(), vec![row]);
    assert!(jsonl.contains("\"component_values\""));
    assert!(jsonl.contains("\"result_value_digest\""));
}

#[test]
fn blank_composition_certificate_fields_are_rejected() {
    let certificate = LabelCertificate {
        kind: "blank-composition".to_owned(),
        digest: "digest".to_owned(),
        composition: Some(Box::new(CompositionCertificate {
            decomposition_digest: String::new(),
            composition_digest: String::new(),
            component_values: BTreeMap::new(),
            result_value_digest: String::new(),
        })),
    };
    let row = DatasetLabelRow::exact(
        "blank-composition-certificate",
        FORMAL_CGT_DOMAIN_ID,
        DatasetPosition::cgt_canonical("Number(0/2^0)"),
        ExactLabel::verified(
            BTreeMap::from([("digest".to_owned(), "thermograph:zero".to_owned())]),
            ExactValueClass::Number,
        ),
        ExactProvenance {
            code_commit: "workspace".to_owned(),
            generator: "composition_certificate_scaffold_test".to_owned(),
            generator_config_hash: "astralbase:composition-scaffold:v0".to_owned(),
            random_seed: 0,
            domain_definition: FORMAL_CGT_DOMAIN_DEFINITION.to_owned(),
            verifier: "composition_certificate_scaffold_verifier".to_owned(),
            verifier_version: env!("CARGO_PKG_VERSION").to_owned(),
            certificate,
        },
    );

    let issues = validate_dataset_label_row(&row).unwrap_err();
    assert!(issues.iter().any(|issue| {
        issue
            .message
            .contains("composition_digest must be non-empty")
    }));
    assert!(
        issues
            .iter()
            .any(|issue| { issue.message.contains("component_values must be non-empty") })
    );
    assert!(issues.iter().any(|issue| {
        issue
            .message
            .contains("result_value_digest must be non-empty")
    }));
}

#[test]
fn composition_result_digest_must_match_exact_digest() {
    let mut row = composition_hard_target_shard(1).remove(0);
    let LabelPayload::Exact { provenance, .. } = &mut row.label else {
        panic!("fixture row should be exact");
    };
    let composition = provenance
        .certificate
        .composition
        .as_deref_mut()
        .expect("fixture exact row should carry composition fields");
    composition.result_value_digest = "thermograph:mismatched-result".to_owned();

    let issues = validate_dataset_label_row(&row).unwrap_err();
    assert!(issues.iter().any(|issue| {
        issue
            .message
            .contains("result_value_digest must equal exact.value.digest")
    }));
}

#[test]
fn composition_rows_require_exact_digest() {
    let mut row = composition_hard_target_shard(1).remove(0);
    let LabelPayload::Exact { exact, .. } = &mut row.label else {
        panic!("fixture row should be exact");
    };
    exact.value.remove("digest");

    let issues = validate_dataset_label_row(&row).unwrap_err();
    assert!(issues.iter().any(|issue| {
        issue
            .message
            .contains("exact.value.digest must be present for composition rows")
    }));
}

#[test]
fn composition_exact_rows_require_structured_certificate() {
    let mut row = composition_hard_target_shard(1).remove(0);
    let LabelPayload::Exact { provenance, .. } = &mut row.label else {
        panic!("fixture row should be exact");
    };
    provenance.certificate = LabelCertificate::legacy("legacy-composition", "legacy-digest");

    let issues = validate_dataset_label_row(&row).unwrap_err();
    assert!(issues.iter().any(|issue| {
        issue.message.contains(
            "composition exact rows must include structured composition certificate fields",
        )
    }));
}

#[test]
fn composition_shard_configs_separate_fixture_and_non_fixture_domains() {
    assert_eq!(
        COMPOSITION_FIXTURE_SHARD_CONFIG.kind,
        CompositionShardKind::Fixture
    );
    assert_eq!(
        NON_FIXTURE_COMPOSED_DOMAIN_SHARD_CONFIG.kind,
        CompositionShardKind::NonFixtureComposedDomain
    );
    assert_eq!(
        COMPOSITION_FIXTURE_SHARD_CONFIG.domain_id,
        COMPOSITION_FIXTURE_DOMAIN_ID
    );
    assert_eq!(
        NON_FIXTURE_COMPOSED_DOMAIN_SHARD_CONFIG.domain_id,
        NON_FIXTURE_COMPOSED_DOMAIN_ID
    );
    assert_eq!(
        NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG.domain_id,
        NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID
    );
    assert_ne!(
        COMPOSITION_FIXTURE_SHARD_CONFIG.domain_id,
        NON_FIXTURE_COMPOSED_DOMAIN_SHARD_CONFIG.domain_id
    );
    assert_ne!(
        NON_FIXTURE_COMPOSED_DOMAIN_SHARD_CONFIG.domain_id,
        NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG.domain_id
    );
    assert_ne!(
        COMPOSITION_FIXTURE_SHARD_CONFIG.generator_config_hash,
        NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG.generator_config_hash
    );
}

#[test]
fn composition_hard_target_shard_is_deterministic_and_mixed() {
    let rows = composition_hard_target_shard(DEFAULT_COMPOSITION_HARD_TARGET_SHARD_LIMIT);
    assert_eq!(rows.len(), DEFAULT_COMPOSITION_HARD_TARGET_SHARD_LIMIT);
    assert_eq!(rows[0].label_kind(), LabelKind::Exact);
    assert_eq!(
        rows.iter()
            .filter(|row| row.label_kind() == LabelKind::Exact)
            .count(),
        COMPOSITION_FIXTURE_EXACT_SPECS.len()
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.label_kind() == LabelKind::Rejected)
            .count(),
        COMPOSITION_FIXTURE_REJECTED_CONTROLS.len()
    );
    assert!(
        rows.iter()
            .any(|row| matches!(row.label, LabelPayload::Rejected { .. }))
    );

    for row in &rows {
        validate_dataset_label_row(row).unwrap();
    }

    assert_eq!(
        composition_hard_target_shard_jsonl(DEFAULT_COMPOSITION_HARD_TARGET_SHARD_LIMIT).unwrap(),
        composition_hard_target_shard_jsonl(DEFAULT_COMPOSITION_HARD_TARGET_SHARD_LIMIT).unwrap()
    );
}

#[test]
fn composition_hard_target_limit_four_keeps_original_smoke_shape() {
    let rows = composition_hard_target_shard(4);

    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].row_id, "astralbase-w17-composition-exact-wall-001");
    assert_eq!(rows[0].label_kind(), LabelKind::Exact);
    assert_eq!(
        rows.iter()
            .filter(|row| row.label_kind() == LabelKind::Rejected)
            .count(),
        3
    );
}

#[test]
fn fixture_and_non_fixture_composition_shards_are_distinct() {
    let fixture_rows = composition_hard_target_shard(4);
    let non_fixture_rows =
        non_fixture_composed_domain_shard(DEFAULT_NON_FIXTURE_COMPOSED_DOMAIN_SHARD_LIMIT);

    assert!(
        fixture_rows
            .iter()
            .all(|row| row.domain == COMPOSITION_FIXTURE_DOMAIN_ID)
    );
    assert!(
        fixture_rows
            .iter()
            .any(|row| row.label_kind() == LabelKind::Exact)
    );
    assert!(
        non_fixture_rows
            .iter()
            .any(|row| row.domain == NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID
                && row.label_kind() == LabelKind::Exact)
    );
    assert!(
        non_fixture_rows
            .iter()
            .any(|row| row.domain == NON_FIXTURE_COMPOSED_DOMAIN_ID
                && row.label_kind() == LabelKind::Rejected)
    );
    for non_fixture_row in &non_fixture_rows {
        assert!(
            non_fixture_row
                .row_id
                .starts_with(NON_FIXTURE_COMPOSED_DOMAIN_SHARD_CONFIG.row_id_prefix)
                || non_fixture_row
                    .row_id
                    .starts_with(NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG.row_id_prefix)
        );
        assert_ne!(
            non_fixture_row.position.text,
            COMPOSITION_FIXTURE_LOCKED_WALL_FEN
        );
        match &non_fixture_row.label {
            LabelPayload::Exact { exact, provenance } => {
                assert_eq!(non_fixture_row.domain, NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID);
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
                    Some(CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE)
                        | Some(PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE)
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
                assert_ne!(
                    composition_value_rule,
                    Some("component_index_integer_sum_fixture_v0")
                );
                match composition_value_rule {
                    Some("component_material_balance_sum_v0") => {
                        assert_eq!(
                            exact.value.get("solver_scope").map(String::as_str),
                            Some("composition_board_material_components")
                        );
                        assert_eq!(
                            provenance.verifier,
                            "bitmesh_conservative_board_material_bmcompose_verifier"
                        );
                    }
                    Some("component_agency_atom_sum_v0") => {
                        assert_eq!(
                            exact.value.get("solver_scope").map(String::as_str),
                            Some("composition_board_agency_atoms")
                        );
                        assert_eq!(exact.value_class, ExactValueClass::GameTree);
                        let component_value_classes = exact
                            .value
                            .get("component_value_classes")
                            .expect("agency atom row records component value classes");
                        assert!(component_value_classes.contains("=up"));
                        assert!(component_value_classes.contains("=down"));
                        assert_eq!(
                            provenance.verifier,
                            "bitmesh_conservative_board_agency_atom_bmcompose_verifier"
                        );
                    }
                    Some("component_local_move_game_v0") => {
                        assert_eq!(
                            exact.value.get("solver_scope").map(String::as_str),
                            Some("composition_board_component_local_moves")
                        );
                        assert_eq!(exact.value_class, ExactValueClass::GameTree);
                        let component_local_move_counts = exact
                            .value
                            .get("component_local_move_counts")
                            .expect("local move row records component move counts");
                        assert!(component_local_move_counts.contains("white:"));
                        assert!(component_local_move_counts.contains("black:"));
                        assert_eq!(
                            provenance.verifier,
                            "bitmesh_conservative_component_local_move_bmcompose_verifier"
                        );
                    }
                    Some("component_depth2_local_move_game_v0") => {
                        assert_eq!(
                            exact.value.get("solver_scope").map(String::as_str),
                            Some("composition_board_component_depth2_local_moves")
                        );
                        assert_eq!(
                            exact.value.get("solver_depth").map(String::as_str),
                            Some("2")
                        );
                        assert!(matches!(
                            exact
                                .value
                                .get("component_topology_family")
                                .map(String::as_str),
                            Some(DEPTH_TWO_LOCAL_MOVE_TOPOLOGY_FAMILY)
                                | Some(DEPTH_TWO_ASYMMETRIC_FAN_TOPOLOGY_FAMILY)
                                | Some(DEPTH_TWO_PHALANX_TOPOLOGY_FAMILY)
                        ));
                        assert_eq!(exact.value_class, ExactValueClass::GameTree);
                        assert_eq!(
                            exact.value.get("recursive_leaf_rule").map(String::as_str),
                            Some("component_material_balance_at_depth_cutoff_or_no_moves_v0")
                        );
                        let component_recursive_node_counts = exact
                            .value
                            .get("component_recursive_node_counts")
                            .expect("depth-2 row records recursive node counts");
                        assert!(component_recursive_node_counts.contains("depth:2"));
                        assert!(
                            exact
                                .value
                                .get("component_recursive_total_nodes")
                                .and_then(|nodes| nodes.parse::<usize>().ok())
                                .is_some_and(|nodes| nodes > 2)
                        );
                        assert_eq!(
                            provenance.verifier,
                            "bitmesh_conservative_component_depth2_local_move_bmcompose_verifier"
                        );
                    }
                    _ => unreachable!("validated composition value rule"),
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
                assert_eq!(non_fixture_row.domain, NON_FIXTURE_COMPOSED_DOMAIN_ID);
                assert_eq!(rejected.status, RejectedStatus::Unsupported);
                assert!(
                    rejected.reasons.iter().any(|reason| {
                        reason.starts_with("unsupported_non_fixture_composition:")
                    })
                );
            }
            _ => panic!("non-fixture composition shard contains unsupported label kind"),
        }
    }
}

#[test]
fn composition_hard_target_exact_fixtures_are_distinct() {
    let rows = composition_hard_target_shard(DEFAULT_COMPOSITION_HARD_TARGET_SHARD_LIMIT);
    let exact_rows = rows
        .iter()
        .filter(|row| row.label_kind() == LabelKind::Exact)
        .collect::<Vec<_>>();
    let mut positions = std::collections::BTreeSet::new();
    let mut result_digests = std::collections::BTreeSet::new();
    let mut decomposition_digests = std::collections::BTreeSet::new();
    let mut composition_digests = std::collections::BTreeSet::new();
    let mut component_counts = std::collections::BTreeSet::new();

    assert_eq!(exact_rows.len(), COMPOSITION_FIXTURE_EXACT_SPECS.len());
    for row in exact_rows {
        assert!(positions.insert(row.position.text.as_str()));
        let LabelPayload::Exact { exact, provenance } = &row.label else {
            unreachable!("filtered exact rows only");
        };
        let composition = provenance
            .certificate
            .composition
            .as_deref()
            .expect("composition exact row must carry structured certificate fields");
        component_counts.insert(
            exact
                .value
                .get("component_count")
                .expect("exact component count is present")
                .as_str(),
        );
        assert!(
            result_digests.insert(
                exact
                    .value
                    .get("digest")
                    .expect("exact result digest is present")
                    .as_str()
            )
        );
        assert!(decomposition_digests.insert(composition.decomposition_digest.as_str()));
        assert!(composition_digests.insert(composition.composition_digest.as_str()));
    }
    assert_eq!(
        component_counts,
        std::collections::BTreeSet::from(["2", "3"])
    );
}

#[test]
fn composition_hard_target_exact_row_carries_nested_certificate() {
    let rows = composition_hard_target_shard(1);
    let LabelPayload::Exact { exact, provenance } = &rows[0].label else {
        panic!("first composition fixture row should be exact");
    };

    assert_eq!(
        exact.value.get("solver_scope").map(String::as_str),
        Some("composition_certificate_fixture")
    );
    assert_eq!(
        exact
            .value
            .get("composition_value_rule")
            .map(String::as_str),
        Some("component_index_integer_sum_fixture_v0")
    );

    let composition = provenance
        .certificate
        .composition
        .as_deref()
        .expect("composition exact row must carry structured certificate fields");
    assert_eq!(composition.decomposition_digest.len(), 64);
    assert_eq!(composition.composition_digest.len(), 64);
    assert!(!composition.component_values.is_empty());
    assert_eq!(
        composition.result_value_digest,
        exact
            .value
            .get("digest")
            .expect("exact result digest is present")
            .as_str()
    );
}

#[test]
fn frontier_audited_shard_is_deterministic_and_mixed() {
    let rows = frontier_audited_shard(25);
    assert_eq!(rows.len(), 25);
    assert_eq!(rows, frontier_audited_shard(25));

    let exact_count = rows
        .iter()
        .filter(|row| row.label_kind() == LabelKind::Exact)
        .count();
    let rejected_count = rows
        .iter()
        .filter(|row| row.label_kind() == LabelKind::Rejected)
        .count();

    assert_eq!(exact_count, 5);
    assert_eq!(rejected_count, 20);
    assert!(
        rows.iter()
            .all(|row| row.row_id.starts_with("astralbase-w6-kqk-frontier-"))
    );
}

#[test]
fn frontier_exact_rows_use_kqk_generator_provenance() {
    let rows = frontier_audited_shard(10);
    let exact = rows
        .iter()
        .find_map(|row| match &row.label {
            LabelPayload::Exact { exact, provenance } => Some((exact, provenance)),
            _ => None,
        })
        .expect("frontier shard should include exact rows");

    assert_eq!(exact.0.value_class, ExactValueClass::Number);
    assert_eq!(exact.1.generator, "astralbase_kqk_frontier_generator");
    assert_eq!(
        exact.1.generator_config_hash,
        "astralbase:kqk_terminal_frontier:v1"
    );
}

#[test]
fn family_frontier_shard_includes_kqk_and_krk_families() {
    let rows = family_frontier_audited_shard(20);
    assert_eq!(rows.len(), 40);
    assert!(
        rows.iter()
            .any(|row| row.row_id.starts_with("astralbase-w7-kqk-frontier-"))
    );
    assert!(
        rows.iter()
            .any(|row| row.row_id.starts_with("astralbase-w7-krk-frontier-"))
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.label_kind() == LabelKind::Exact)
            .count(),
        8
    );
    assert!(rows.iter().any(|row| match &row.label {
        LabelPayload::Exact { provenance, .. } =>
            provenance.generator == "astralbase_krk_frontier_generator",
        _ => false,
    }));
}

#[test]
fn expanded_family_frontier_shard_includes_major_and_minor_families() {
    let rows = expanded_family_frontier_audited_shard(20);
    assert_eq!(rows.len(), 80);
    assert_eq!(rows, expanded_family_frontier_audited_shard(20));

    for prefix in [
        "astralbase-w7-kqk-frontier-",
        "astralbase-w7-krk-frontier-",
        "astralbase-w12-kbk-frontier-",
        "astralbase-w12-knk-frontier-",
    ] {
        assert!(
            rows.iter().any(|row| row.row_id.starts_with(prefix)),
            "missing expanded frontier family prefix {prefix}"
        );
    }

    assert_eq!(
        rows.iter()
            .filter(|row| row.label_kind() == LabelKind::Exact)
            .count(),
        16
    );
    for generator in [
        "astralbase_kbk_frontier_generator",
        "astralbase_knk_frontier_generator",
    ] {
        assert!(
            rows.iter().any(|row| match &row.label {
                LabelPayload::Exact { provenance, .. } => provenance.generator == generator,
                _ => false,
            }),
            "missing exact provenance for {generator}"
        );
    }
}

#[test]
fn exact_sample_uses_thermograph_payload_contract() {
    let rows = sample_audited_shard();
    let LabelPayload::Exact { exact, provenance } = &rows[0].label else {
        panic!("first sample row must be exact");
    };

    assert_eq!(exact.value_class, ExactValueClass::Number);
    assert_eq!(
        exact.value.get("solver_scope").unwrap(),
        "terminal_position"
    );
    assert_eq!(exact.value.get("value_class").unwrap(), "number");
    assert_eq!(
        exact.value.get("canonical_serialization").unwrap(),
        "Number(-1/2^0)"
    );
    assert!(
        exact
            .value
            .get("digest")
            .is_some_and(|digest| !digest.is_empty())
    );
    assert_eq!(exact.value.get("dyadic_numerator").unwrap(), "-1");
    assert_eq!(exact.value.get("dyadic_denominator_power").unwrap(), "0");
    assert!(provenance.certificate.digest.contains("bitmesh:"));
    assert!(provenance.certificate.digest.contains("thermograph:"));
}

#[test]
fn nonterminal_sample_uses_immediate_tactic_contract() {
    let rows = sample_audited_shard();
    let LabelPayload::Exact { exact, provenance } = &rows[1].label else {
        panic!("second sample row must be exact");
    };

    assert_eq!(exact.value_class, ExactValueClass::Number);
    assert_eq!(
        exact.value.get("solver_scope").unwrap(),
        "immediate_terminal_frontier"
    );
    assert_eq!(
        exact.value.get("canonical_serialization").unwrap(),
        "Number(1/2^0)"
    );
    assert_eq!(exact.value.get("dyadic_numerator").unwrap(), "1");
    assert_eq!(exact.value.get("terminal_distance_plies").unwrap(), "1");
    assert_eq!(exact.value.get("legal_move_count").unwrap(), "26");
    assert_eq!(exact.value.get("terminal_child_count").unwrap(), "14");
    assert_eq!(exact.value.get("checkmating_move_count").unwrap(), "4");
    assert_eq!(
        exact.value.get("checkmating_moves").unwrap(),
        "Qg6-g7,Qg6-g8,Qg6-h5,Qg6-h6"
    );
    assert_eq!(exact.value.get("stalemating_move_count").unwrap(), "10");
    assert_eq!(
        exact.value.get("stalemating_moves").unwrap(),
        "Kf7-e6,Kf7-e7,Kf7-e8,Kf7-f6,Kf7-f8,Qg6-b1,Qg6-c2,Qg6-d3,Qg6-e4,Qg6-f5"
    );
    assert_eq!(
        exact.value.get("frontier_value_class").unwrap(),
        "game_tree"
    );
    assert_eq!(
        exact.value.get("frontier_canonical_serialization").unwrap(),
        "GameTree(L[Number(0/2^0),Number(1/2^0)];R[])"
    );
    assert_eq!(exact.value.get("frontier_mean").unwrap(), "2");
    assert_eq!(exact.value.get("frontier_temperature").unwrap(), "-1");
    assert_eq!(
        exact.value.get("frontier_perspective").unwrap(),
        "parent_side_to_move"
    );
    assert!(
        exact
            .value
            .get("terminal_child_statuses")
            .unwrap()
            .contains("Qg6-g7=checkmate")
    );
    assert_eq!(
        provenance.verifier,
        "astralbase_immediate_terminal_tactic_solver"
    );
    assert!(provenance.certificate.digest.contains("frontier:"));
    assert!(
        provenance
            .certificate
            .digest
            .contains("frontier_thermograph:")
    );
}

#[test]
fn formal_switch_sample_uses_non_number_exact_contract() {
    let rows = sample_audited_shard();
    let switch_row = &rows[2];
    let LabelPayload::Exact { exact, provenance } = &switch_row.label else {
        panic!("third sample row must be exact");
    };

    assert_eq!(switch_row.domain, FORMAL_CGT_DOMAIN_ID);
    assert_eq!(switch_row.position.encoding, PositionEncoding::CgtCanonical);
    assert_eq!(exact.value_class, ExactValueClass::Switch);
    assert_eq!(
        exact.value.get("solver_scope").unwrap(),
        "formal_cgt_switch_fixture"
    );
    assert_eq!(exact.value.get("value_class").unwrap(), "switch");
    assert_eq!(
        exact.value.get("canonical_serialization").unwrap(),
        "GameTree(L[Number(1/2^0)];R[Number(-1/2^0)])"
    );
    assert_eq!(exact.value.get("digest").unwrap(), "0cc26090a9cea850");
    assert_eq!(exact.value.get("temperature").unwrap(), "1");
    assert_eq!(exact.value.get("mean").unwrap(), "0");
    assert!(!exact.value.contains_key("dyadic_numerator"));
    assert_eq!(provenance.verifier, "thermograph_switch_fixture_verifier");
    assert!(
        provenance
            .certificate
            .digest
            .contains("thermograph:0cc26090a9cea850")
    );
}

#[test]
fn unsupported_sample_candidates_are_rejected_not_exact() {
    let rows = sample_audited_shard();
    let rejected = rows
        .iter()
        .filter(|row| row.label_kind() == LabelKind::Rejected)
        .collect::<Vec<_>>();

    assert_eq!(rejected.len(), 2);
    assert!(rejected.iter().any(|row| {
        match &row.label {
            LabelPayload::Rejected { rejected } => rejected
                .reasons
                .iter()
                .any(|reason| reason.starts_with("castling_rights:")),
            _ => false,
        }
    }));
    assert!(rejected.iter().any(|row| {
        match &row.label {
            LabelPayload::Rejected { rejected } => rejected
                .reasons
                .iter()
                .any(|reason| reason.starts_with("no_strict_decomposition:")),
            _ => false,
        }
    }));
}

#[test]
fn rejected_rows_require_a_visible_reason() {
    let row = DatasetLabelRow::rejected(
        "missing-reason",
        "formal_domain:first_constrained_chess:v0",
        DatasetPosition::fen("8/8/8/8/8/8/8/4K2k w KQ - 0 1"),
        RejectedLabel::unsupported(Vec::new()),
    );

    let issues = validate_dataset_label_row(&row).unwrap_err();
    assert!(issues.iter().any(|issue| {
        issue
            .message
            .contains("rejected.reasons must be a non-empty list")
    }));
}

#[test]
fn parse_rejects_mixed_label_payloads() {
    let jsonl = "{\"schema_version\":\"partizan.dataset_label.v0\",\"row_id\":\"mixed\",\"domain\":\"formal_domain:first_constrained_chess:v0\",\"position\":{\"encoding\":\"fen\",\"text\":\"8/8/8/8/8/8/8/4K2k w - - 0 1\"},\"label_kind\":\"exact\",\"exact\":{\"status\":\"verified\",\"value\":{\"canonical\":\"0\"},\"value_class\":\"integer\"},\"prediction\":{\"model_id\":\"m\",\"model_version\":\"0\",\"checkpoint\":\"c\",\"outputs\":{\"mean\":\"0\"}},\"provenance\":{\"code_commit\":\"c\",\"generator\":\"g\",\"generator_config_hash\":\"h\",\"random_seed\":0,\"domain_definition\":\"d\",\"verifier\":\"v\",\"verifier_version\":\"0\",\"certificate\":{\"kind\":\"k\",\"digest\":\"d\"}}}\n";

    let issues = parse_and_validate_jsonl(jsonl).unwrap_err();
    assert!(issues.iter().any(|issue| {
        issue
            .message
            .contains("row must contain exactly one label payload object")
    }));
}
