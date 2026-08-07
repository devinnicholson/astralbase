use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonFixtureCompositionReplayReport {
    pub row_count: usize,
    pub checked_exact_rows: usize,
    pub skipped_rejected_rows: usize,
    pub skipped_non_target_rows: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureTargetReplayPreflightReport {
    pub row_count: usize,
    pub checked_heuristic_rows: usize,
    pub skipped_exact_rows: usize,
    pub skipped_rejected_rows: usize,
    pub skipped_non_target_rows: usize,
    pub required_output_field_missing_counts: BTreeMap<String, usize>,
    pub contract_field_mismatch_counts: BTreeMap<String, usize>,
    pub replay_failure_count: usize,
    pub replay_check_pass_counts: BTreeMap<String, usize>,
    pub replay_check_failure_counts: BTreeMap<String, usize>,
    pub mismatch_examples: Vec<SignatureTargetReplayMismatch>,
    pub promotion_blocker_counts: BTreeMap<String, usize>,
    pub replayability_status: String,
    pub promotion_gate_passed: bool,
    pub promotion_blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureTargetReplayMismatch {
    pub row_id: String,
    pub field: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoProfileSearchReport {
    pub rows_per_family_target: usize,
    pub left_profile_count: usize,
    pub right_profile_count: usize,
    pub candidate_pair_counts_by_topology_family: BTreeMap<String, usize>,
    pub selected_row_count: usize,
    pub selected_counts_by_topology_family: BTreeMap<String, usize>,
    pub rejection_counts: BTreeMap<String, usize>,
    pub candidates: Vec<GeneratedDepthTwoProfileCandidateReport>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoProfileSourceSearchReport {
    pub source: String,
    pub report: GeneratedDepthTwoProfileSearchReport,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoSignatureProfileSearchReport {
    pub source: String,
    pub component_signature_rule: String,
    pub rows_per_family_target: usize,
    #[serde(default)]
    pub target_row_count: usize,
    pub left_signature_profile_count: usize,
    pub right_signature_profile_count: usize,
    pub candidate_pair_counts_by_topology_family: BTreeMap<String, usize>,
    pub selected_row_count: usize,
    #[serde(default)]
    pub selected_gap_count: usize,
    pub selected_counts_by_topology_family: BTreeMap<String, usize>,
    #[serde(default)]
    pub remaining_gap_by_topology_family: BTreeMap<String, usize>,
    #[serde(default)]
    pub reached_target_by_topology_family: BTreeMap<String, bool>,
    pub rejection_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub rejection_counts_by_topology_family: BTreeMap<String, BTreeMap<String, usize>>,
    pub candidates: Vec<GeneratedDepthTwoSignatureProfileCandidateReport>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoSignatureBoundedSupportReport {
    pub source: String,
    pub component_signature_rule: String,
    pub rows_per_family_target: usize,
    pub candidate_pair_limit_per_family: usize,
    pub left_signature_profile_count: usize,
    pub right_signature_profile_count: usize,
    pub candidate_pair_counts_by_topology_family: BTreeMap<String, usize>,
    pub candidate_offsets_by_topology_family: BTreeMap<String, usize>,
    pub selected_row_count: usize,
    pub selected_counts_by_topology_family: BTreeMap<String, usize>,
    pub reached_target_by_topology_family: BTreeMap<String, bool>,
    pub candidate_pair_limit_hit_by_topology_family: BTreeMap<String, bool>,
    pub rejection_counts: BTreeMap<String, usize>,
    pub candidates: Vec<GeneratedDepthTwoSignatureProfileCandidateReport>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoValueUniqueSignatureUpperBoundReport {
    pub source: String,
    pub component_signature_rule: String,
    pub rows_per_family_target: usize,
    pub current_selection_evaluated: bool,
    pub left_signature_profile_count: usize,
    pub right_signature_profile_count: usize,
    pub left_unique_component_value_digest_count: usize,
    pub right_unique_component_value_digest_count: usize,
    pub shared_component_value_digest_count: usize,
    pub combined_unique_component_value_digest_count: usize,
    pub component_value_capacity_upper_bound: usize,
    pub target_row_count: usize,
    pub candidate_pair_counts_by_topology_family: BTreeMap<String, usize>,
    pub distinct_candidate_value_pair_counts_by_topology_family: BTreeMap<String, usize>,
    pub current_selected_row_count: usize,
    pub current_selected_counts_by_topology_family: BTreeMap<String, usize>,
    pub current_rejection_counts: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoValueUniqueSignatureSourceSweepReport {
    pub rows_per_family_target: usize,
    pub sources: Vec<GeneratedDepthTwoValueUniqueSignatureUpperBoundReport>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoSignatureProfileCandidateReport {
    pub row_number: usize,
    pub topology_family: String,
    pub left_profile_index: usize,
    pub right_profile_index: usize,
    pub total_recursive_nodes: usize,
    pub active_pieces: String,
    pub left_component_value_digest: String,
    pub right_component_value_digest: String,
    pub left_component_signature: String,
    pub right_component_signature: String,
    pub result_signature_key: String,
    pub current_result_value_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoProfileCandidateReport {
    pub row_number: usize,
    pub topology_family: String,
    pub left_profile_index: usize,
    pub right_profile_index: usize,
    pub total_recursive_nodes: usize,
    pub active_pieces: String,
    pub left_component_value_digest: String,
    pub right_component_value_digest: String,
    pub result_value_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoProfileInventoryReport {
    pub white: GeneratedDepthTwoProfileInventorySideReport,
    pub black: GeneratedDepthTwoProfileInventorySideReport,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoProfileSourceInventoryReport {
    pub sources: Vec<GeneratedDepthTwoNamedProfileInventoryReport>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoNamedProfileInventoryReport {
    pub source: String,
    pub white: GeneratedDepthTwoProfileInventorySideReport,
    pub black: GeneratedDepthTwoProfileInventorySideReport,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoDuplicateClusterReport {
    pub source: String,
    pub white: GeneratedDepthTwoDuplicateClusterSideReport,
    pub black: GeneratedDepthTwoDuplicateClusterSideReport,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoDuplicateClusterSideReport {
    pub pattern_count: usize,
    pub wall_safe_pattern_count: usize,
    pub budget_profile_count: usize,
    pub unique_value_digest_count: usize,
    pub duplicate_cluster_count: usize,
    pub duplicate_profile_count: usize,
    pub rejection_counts: BTreeMap<String, usize>,
    pub clusters: Vec<GeneratedDepthTwoDuplicateClusterSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoDuplicateClusterSummary {
    pub value_digest: String,
    pub profile_count: usize,
    pub distinct_signature_count: usize,
    pub signatures: Vec<String>,
    pub examples: Vec<GeneratedDepthTwoDuplicateClusterExample>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoDuplicateClusterExample {
    pub active_pieces: String,
    pub material_balance: i32,
    pub local_move_counts: String,
    pub recursive_nodes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedLocalMoveProfileInventoryReport {
    pub component_depth: u8,
    pub white: GeneratedDepthTwoProfileInventorySideReport,
    pub black: GeneratedDepthTwoProfileInventorySideReport,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoProfileInventorySideReport {
    pub pattern_count: usize,
    pub wall_safe_pattern_count: usize,
    pub accepted_profile_count: usize,
    pub rejection_counts: BTreeMap<String, usize>,
    pub profiles: Vec<GeneratedDepthTwoComponentProfileReport>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedDepthTwoComponentProfileReport {
    pub profile_index: usize,
    pub active_pieces: String,
    pub value_class: String,
    pub value_digest: String,
    pub recursive_nodes: usize,
}

pub fn replay_verify_non_fixture_composed_domain_jsonl(
    input: &str,
) -> LabelValidationResult<NonFixtureCompositionReplayReport> {
    let rows = parse_and_validate_jsonl(input)?;
    replay_verify_non_fixture_composed_domain_rows(&rows)
}

pub fn replay_verify_non_fixture_composed_domain_rows(
    rows: &[DatasetLabelRow],
) -> LabelValidationResult<NonFixtureCompositionReplayReport> {
    let mut report = NonFixtureCompositionReplayReport {
        row_count: rows.len(),
        checked_exact_rows: 0,
        skipped_rejected_rows: 0,
        skipped_non_target_rows: 0,
    };
    let mut issues = Vec::new();

    for row in rows {
        match &row.label {
            LabelPayload::Exact { exact, provenance }
                if row.domain == NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID =>
            {
                report.checked_exact_rows += 1;
                replay_verify_non_fixture_composed_board_exact_row(
                    row,
                    exact,
                    provenance,
                    &mut issues,
                );
            }
            LabelPayload::Rejected { .. } if row.domain == NON_FIXTURE_COMPOSED_DOMAIN_ID => {
                report.skipped_rejected_rows += 1;
            }
            _ => {
                report.skipped_non_target_rows += 1;
            }
        }
    }

    if issues.is_empty() {
        Ok(report)
    } else {
        Err(issues)
    }
}

pub fn signature_target_replay_preflight_jsonl(
    input: &str,
) -> LabelValidationResult<SignatureTargetReplayPreflightReport> {
    let rows = parse_and_validate_jsonl(input)?;
    Ok(signature_target_replay_preflight_rows(&rows))
}

#[must_use]
pub fn signature_target_replay_preflight_rows(
    rows: &[DatasetLabelRow],
) -> SignatureTargetReplayPreflightReport {
    let mut report = SignatureTargetReplayPreflightReport {
        row_count: rows.len(),
        checked_heuristic_rows: 0,
        skipped_exact_rows: 0,
        skipped_rejected_rows: 0,
        skipped_non_target_rows: 0,
        required_output_field_missing_counts: BTreeMap::new(),
        contract_field_mismatch_counts: BTreeMap::new(),
        replay_failure_count: 0,
        replay_check_pass_counts: BTreeMap::new(),
        replay_check_failure_counts: BTreeMap::new(),
        mismatch_examples: Vec::new(),
        promotion_blocker_counts: BTreeMap::new(),
        replayability_status: String::new(),
        promotion_gate_passed: false,
        promotion_blockers: signature_target_required_promotion_blockers(),
    };

    for row in rows {
        match &row.label {
            LabelPayload::Heuristic { heuristic }
                if row.domain == NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID
                    && heuristic.method == "signature_profile_target_diagnostic" =>
            {
                report.checked_heuristic_rows += 1;
                signature_target_replay_preflight_row(row, heuristic, &mut report);
            }
            LabelPayload::Exact { .. } => {
                report.skipped_exact_rows += 1;
            }
            LabelPayload::Rejected { .. } => {
                report.skipped_rejected_rows += 1;
            }
            _ => {
                report.skipped_non_target_rows += 1;
            }
        }
    }

    let replay_preflight_passed = report.required_output_field_missing_counts.is_empty()
        && report.contract_field_mismatch_counts.is_empty()
        && report.replay_failure_count == 0
        && report.replay_check_failure_counts.is_empty();
    report.replayability_status = if replay_preflight_passed {
        "replay_preflight_passed_promotion_blocked"
    } else {
        "replay_preflight_failed_promotion_blocked"
    }
    .to_owned();
    report
}

fn signature_target_replay_preflight_row(
    row: &DatasetLabelRow,
    heuristic: &HeuristicLabel,
    report: &mut SignatureTargetReplayPreflightReport,
) {
    for field in SIGNATURE_TARGET_REQUIRED_OUTPUT_FIELDS {
        if signature_output(heuristic, field).is_none() {
            increment_count(&mut report.required_output_field_missing_counts, field);
        }
    }

    record_signature_contract_check(
        report,
        "target_contract_id",
        signature_output(heuristic, "target_contract_id").as_deref()
            == Some(SIGNATURE_TARGET_CONTRACT_ID),
    );
    record_signature_contract_check(
        report,
        "target_status",
        signature_output(heuristic, "target_status").as_deref() == Some("diagnostic_only"),
    );
    record_signature_contract_check(
        report,
        "supervision_eligible",
        signature_output(heuristic, "supervision_eligible").as_deref() == Some("false"),
    );
    record_signature_contract_check(
        report,
        "component_signature_rule",
        signature_output(heuristic, "component_signature_rule").as_deref()
            == Some(SIGNATURE_TARGET_COMPONENT_RULE),
    );
    record_signature_contract_check(
        report,
        "composition_spec_source",
        signature_output(heuristic, "composition_spec_source").as_deref()
            == Some(SIGNATURE_TARGET_DIAGNOSTIC_SOURCE),
    );

    let blockers = signature_target_blocker_ids(heuristic);
    for blocker in &blockers {
        increment_count(&mut report.promotion_blocker_counts, blocker.as_str());
    }
    for required in signature_target_required_promotion_blockers() {
        record_signature_contract_check(
            report,
            format!("promotion_blocker:{required}").as_str(),
            blockers.iter().any(|blocker| blocker == &required),
        );
    }

    if row.position.encoding != PositionEncoding::Fen {
        report.replay_failure_count += 1;
        push_signature_replay_mismatch(
            report,
            row.row_id.as_str(),
            "position.encoding",
            Some("fen".to_owned()),
            Some(format!("{:?}", row.position.encoding)),
        );
        return;
    }
    let board = match board_from_fen_board_part(row.position.text.as_str()) {
        Ok(board) => board,
        Err(error) => {
            report.replay_failure_count += 1;
            push_signature_replay_mismatch(
                report,
                row.row_id.as_str(),
                "position.text",
                Some(format!("parseable FEN board part: {error}")),
                Some(row.position.text.clone()),
            );
            return;
        }
    };
    let replay = match replay_non_fixture_composed_board(
        &board,
        NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
    ) {
        Ok(replay) => replay,
        Err(error) => {
            report.replay_failure_count += 1;
            push_signature_replay_mismatch(
                report,
                row.row_id.as_str(),
                "replay_non_fixture_composed_board",
                Some("successful replay".to_owned()),
                Some(error),
            );
            return;
        }
    };

    let active_pieces = non_fixture_composed_board_active_piece_summary_from_board(&board);
    record_signature_replay_check(
        report,
        row.row_id.as_str(),
        "heuristic.outputs.active_pieces",
        Some(active_pieces),
        signature_output(heuristic, "active_pieces"),
    );
    record_signature_replay_check(
        report,
        row.row_id.as_str(),
        "heuristic.outputs.current_result_value_digest",
        Some(replay.result_value_digest.clone()),
        signature_output(heuristic, "current_result_value_digest"),
    );

    record_signature_replay_check(
        report,
        row.row_id.as_str(),
        "heuristic.outputs.left_component_value_digest",
        replay.component_value_digests.first().cloned(),
        signature_output(heuristic, "left_component_value_digest"),
    );
    record_signature_replay_check(
        report,
        row.row_id.as_str(),
        "heuristic.outputs.right_component_value_digest",
        replay.component_value_digests.get(1).cloned(),
        signature_output(heuristic, "right_component_value_digest"),
    );
    record_signature_replay_check(
        report,
        row.row_id.as_str(),
        "heuristic.outputs.left_component_signature",
        replay.component_signatures.first().cloned(),
        signature_output(heuristic, "left_component_signature"),
    );
    record_signature_replay_check(
        report,
        row.row_id.as_str(),
        "heuristic.outputs.right_component_signature",
        replay.component_signatures.get(1).cloned(),
        signature_output(heuristic, "right_component_signature"),
    );

    let expected_result_signature = signature_output(heuristic, "component_topology_family")
        .zip(replay.component_signatures.first().cloned())
        .zip(replay.component_signatures.get(1).cloned())
        .map(|((topology, left), right)| {
            generated_depth_two_result_signature_key(
                topology.as_str(),
                left.as_str(),
                right.as_str(),
            )
        });
    record_signature_replay_check(
        report,
        row.row_id.as_str(),
        "heuristic.outputs.result_signature_key",
        expected_result_signature,
        signature_output(heuristic, "result_signature_key"),
    );
    record_signature_replay_check(
        report,
        row.row_id.as_str(),
        "heuristic.outputs.total_recursive_nodes",
        replay
            .exact_values
            .get("component_recursive_total_nodes")
            .cloned(),
        signature_output(heuristic, "total_recursive_nodes"),
    );
}

fn signature_output(heuristic: &HeuristicLabel, field: &str) -> Option<String> {
    heuristic
        .outputs
        .get(field)
        .filter(|value| !value.trim().is_empty())
        .cloned()
}

fn signature_target_blocker_ids(heuristic: &HeuristicLabel) -> Vec<String> {
    signature_output(heuristic, "promotion_blockers")
        .unwrap_or_default()
        .split(';')
        .filter_map(|blocker| {
            let blocker = blocker.trim();
            (!blocker.is_empty()).then(|| blocker.to_owned())
        })
        .collect()
}

fn signature_target_required_promotion_blockers() -> Vec<String> {
    SIGNATURE_TARGET_PROMOTION_BLOCKERS
        .split(';')
        .map(str::to_owned)
        .collect()
}

fn record_signature_contract_check(
    report: &mut SignatureTargetReplayPreflightReport,
    field: &str,
    passed: bool,
) {
    if !passed {
        increment_count(&mut report.contract_field_mismatch_counts, field);
    }
}

fn record_signature_replay_check(
    report: &mut SignatureTargetReplayPreflightReport,
    row_id: &str,
    field: &str,
    expected: Option<String>,
    actual: Option<String>,
) {
    if expected == actual {
        increment_count(&mut report.replay_check_pass_counts, field);
    } else {
        increment_count(&mut report.replay_check_failure_counts, field);
        push_signature_replay_mismatch(report, row_id, field, expected, actual);
    }
}

fn push_signature_replay_mismatch(
    report: &mut SignatureTargetReplayPreflightReport,
    row_id: &str,
    field: &str,
    expected: Option<String>,
    actual: Option<String>,
) {
    if report.mismatch_examples.len() < SIGNATURE_TARGET_REPLAY_PREFLIGHT_MISMATCH_EXAMPLE_LIMIT {
        report
            .mismatch_examples
            .push(SignatureTargetReplayMismatch {
                row_id: row_id.to_owned(),
                field: field.to_owned(),
                expected,
                actual,
            });
    }
}
