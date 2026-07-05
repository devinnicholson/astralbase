use crate::domain::{
    self, FIRST_CONSTRAINED_DOMAIN_DEFINITION, FIRST_CONSTRAINED_DOMAIN_ID,
    ImmediateTerminalTactic, TerminalStatus, ValidatedDomainPosition,
};
use bitmesh::{
    self, CompositionCertificate as BitmeshCompositionCertificate, CompositionComponentValue,
    DecompositionStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shakmaty::{Board, CastlingMode, Chess, Color, Position, Square, fen::Fen};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};
use thermograph::{CGTValue, ExactValuePayload as ThermographExactValuePayload};

pub const DATASET_LABEL_SCHEMA_VERSION: &str = "partizan.dataset_label.v0";
pub const FORMAL_CGT_DOMAIN_ID: &str = "formal_domain:thermograph_golden_cgt:v0";
pub const FORMAL_CGT_DOMAIN_DEFINITION: &str = "thermograph:golden_values#hot_one_minus_one";
pub const DEFAULT_FRONTIER_SHARD_LIMIT: usize = 1_000;
pub const DEFAULT_FAMILY_FRONTIER_LIMIT_PER_FAMILY: usize = 1_000;
pub const DEFAULT_EXPANDED_FAMILY_FRONTIER_LIMIT_PER_FAMILY: usize = 1_000;
pub const DEFAULT_COMPOSITION_HARD_TARGET_SHARD_LIMIT: usize = 21;
pub const DEFAULT_NON_FIXTURE_COMPOSED_DOMAIN_SHARD_LIMIT: usize = 20;
pub const DEFAULT_EXPANDED_NON_FIXTURE_COMPOSED_DOMAIN_ROWS_PER_FAMILY: usize = 10;
pub const DEFAULT_EXPANDED_NON_FIXTURE_COMPOSED_DOMAIN_SHARD_LIMIT: usize = 44;
pub const DEFAULT_LEAKAGE_CLEAN_NON_FIXTURE_COMPOSED_DOMAIN_ROWS_PER_FAMILY: usize = 10;
pub const DEFAULT_SIGNATURE_TARGET_DIAGNOSTIC_ROWS_PER_FAMILY: usize = 10;
pub const COMPOSITION_FIXTURE_DOMAIN_ID: &str = "formal_domain:bitmesh_composition_fixture:v0";
pub const COMPOSITION_FIXTURE_DOMAIN_DEFINITION: &str =
    "docs/formal_domain.md#wave-17-composition-fixture";
pub const NON_FIXTURE_COMPOSED_DOMAIN_ID: &str = "formal_domain:bitmesh_composed_chess:v0";
pub const NON_FIXTURE_COMPOSED_DOMAIN_DEFINITION: &str =
    "docs/formal_domain.md#wave-18-non-fixture-composed-domain";
pub const NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID: &str =
    "formal_domain:bitmesh_composed_board_material:v0";
pub const NON_FIXTURE_COMPOSED_BOARD_DOMAIN_DEFINITION: &str =
    "docs/formal_domain.md#wave-18-board-material-composition";
pub const COMPOSITION_FIXTURE_LOCKED_WALL_FEN: &str = "7n/8/8/PPPPPPPP/PPPPPPPP/8/8/N7 w - - 0 1";
pub const COMPOSITION_FIXTURE_MISSING_COMPONENT_FEN: &str =
    "7n/8/8/PPPPPPPP/PPPPPPPP/8/8/N7 w - - 0 2";
pub const COMPOSITION_FIXTURE_STALE_COMPOSITION_FEN: &str =
    "7n/8/8/PPPPPPPP/PPPPPPPP/8/8/N7 w - - 0 3";
pub const COMPOSITION_FIXTURE_DUPLICATE_ROOT_FEN: &str =
    "7n/8/8/PPPPPPPP/PPPPPPPP/8/8/N7 w - - 0 4";
pub const COMPOSITION_FIXTURE_UNSUPPORTED_VALUE_FEN: &str =
    "7n/8/8/PPPPPPPP/PPPPPPPP/8/8/N7 w - - 0 5";
const COMMON_REQUIRED_FIELDS: [&str; 5] = [
    "schema_version",
    "row_id",
    "domain",
    "position",
    "label_kind",
];
const LABEL_PAYLOAD_KEYS: [&str; 4] = ["exact", "rejected", "heuristic", "prediction"];
const AMBIGUOUS_TOP_LEVEL_FIELDS: [&str; 6] = [
    "components",
    "error",
    "expanded_nodes",
    "label",
    "mean_value",
    "temperature",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionShardKind {
    Fixture,
    NonFixtureComposedDomain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositionShardConfig {
    pub kind: CompositionShardKind,
    pub shard_name: &'static str,
    pub domain_id: &'static str,
    pub domain_definition: &'static str,
    pub generator: &'static str,
    pub generator_config_hash: &'static str,
    pub row_id_prefix: &'static str,
}

pub const COMPOSITION_FIXTURE_SHARD_CONFIG: CompositionShardConfig = CompositionShardConfig {
    kind: CompositionShardKind::Fixture,
    shard_name: "composition_fixture_hard_target",
    domain_id: COMPOSITION_FIXTURE_DOMAIN_ID,
    domain_definition: COMPOSITION_FIXTURE_DOMAIN_DEFINITION,
    generator: "astralbase_composition_fixture_generator",
    generator_config_hash: "astralbase:composition_fixture:v1",
    row_id_prefix: "astralbase-w17-composition",
};

pub const NON_FIXTURE_COMPOSED_DOMAIN_SHARD_CONFIG: CompositionShardConfig =
    CompositionShardConfig {
        kind: CompositionShardKind::NonFixtureComposedDomain,
        shard_name: "non_fixture_composed_domain_mixed",
        domain_id: NON_FIXTURE_COMPOSED_DOMAIN_ID,
        domain_definition: NON_FIXTURE_COMPOSED_DOMAIN_DEFINITION,
        generator: "astralbase_non_fixture_composed_domain_generator",
        generator_config_hash: "astralbase:non_fixture_composed_domain:v3",
        row_id_prefix: "astralbase-w18-non-fixture-composed-domain",
    };

pub const NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG: CompositionShardConfig =
    CompositionShardConfig {
        kind: CompositionShardKind::NonFixtureComposedDomain,
        shard_name: "non_fixture_composed_board_material",
        domain_id: NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID,
        domain_definition: NON_FIXTURE_COMPOSED_BOARD_DOMAIN_DEFINITION,
        generator: "astralbase_non_fixture_composed_board_generator",
        generator_config_hash: "astralbase:non_fixture_composed_board:profiled_depth2_v2",
        row_id_prefix: "astralbase-w18-non-fixture-composed-board",
    };

#[derive(Clone, Copy, Debug)]
struct ExactGenerationContext {
    generator: &'static str,
    terminal_config_hash: &'static str,
    frontier_config_hash: &'static str,
}

const SAMPLE_EXACT_CONTEXT: ExactGenerationContext = ExactGenerationContext {
    generator: "astralbase_vertical_slice_generator",
    terminal_config_hash: "astralbase:first_constrained_sample:v1",
    frontier_config_hash: "astralbase:first_constrained_sample:v2",
};

const KQK_FRONTIER_EXACT_CONTEXT: ExactGenerationContext = ExactGenerationContext {
    generator: "astralbase_kqk_frontier_generator",
    terminal_config_hash: "astralbase:kqk_terminal_frontier:v1",
    frontier_config_hash: "astralbase:kqk_terminal_frontier:v1",
};

const KRK_FRONTIER_EXACT_CONTEXT: ExactGenerationContext = ExactGenerationContext {
    generator: "astralbase_krk_frontier_generator",
    terminal_config_hash: "astralbase:krk_terminal_frontier:v1",
    frontier_config_hash: "astralbase:krk_terminal_frontier:v1",
};

const KBK_FRONTIER_EXACT_CONTEXT: ExactGenerationContext = ExactGenerationContext {
    generator: "astralbase_kbk_frontier_generator",
    terminal_config_hash: "astralbase:kbk_terminal_frontier:v1",
    frontier_config_hash: "astralbase:kbk_terminal_frontier:v1",
};

const KNK_FRONTIER_EXACT_CONTEXT: ExactGenerationContext = ExactGenerationContext {
    generator: "astralbase_knk_frontier_generator",
    terminal_config_hash: "astralbase:knk_terminal_frontier:v1",
    frontier_config_hash: "astralbase:knk_terminal_frontier:v1",
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetLabelRow {
    pub schema_version: String,
    pub row_id: String,
    pub domain: String,
    pub position: DatasetPosition,
    #[serde(flatten)]
    pub label: LabelPayload,
}

impl DatasetLabelRow {
    #[must_use]
    pub fn exact(
        row_id: impl Into<String>,
        domain: impl Into<String>,
        position: DatasetPosition,
        exact: ExactLabel,
        provenance: ExactProvenance,
    ) -> Self {
        Self {
            schema_version: DATASET_LABEL_SCHEMA_VERSION.to_owned(),
            row_id: row_id.into(),
            domain: domain.into(),
            position,
            label: LabelPayload::Exact { exact, provenance },
        }
    }

    #[must_use]
    pub fn rejected(
        row_id: impl Into<String>,
        domain: impl Into<String>,
        position: DatasetPosition,
        rejected: RejectedLabel,
    ) -> Self {
        Self {
            schema_version: DATASET_LABEL_SCHEMA_VERSION.to_owned(),
            row_id: row_id.into(),
            domain: domain.into(),
            position,
            label: LabelPayload::Rejected { rejected },
        }
    }

    #[must_use]
    pub fn heuristic(
        row_id: impl Into<String>,
        domain: impl Into<String>,
        position: DatasetPosition,
        heuristic: HeuristicLabel,
    ) -> Self {
        Self {
            schema_version: DATASET_LABEL_SCHEMA_VERSION.to_owned(),
            row_id: row_id.into(),
            domain: domain.into(),
            position,
            label: LabelPayload::Heuristic { heuristic },
        }
    }

    #[must_use]
    pub fn prediction(
        row_id: impl Into<String>,
        domain: impl Into<String>,
        position: DatasetPosition,
        prediction: PredictionLabel,
    ) -> Self {
        Self {
            schema_version: DATASET_LABEL_SCHEMA_VERSION.to_owned(),
            row_id: row_id.into(),
            domain: domain.into(),
            position,
            label: LabelPayload::Prediction { prediction },
        }
    }

    #[must_use]
    pub fn label_kind(&self) -> LabelKind {
        self.label.kind()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetPosition {
    pub encoding: PositionEncoding,
    pub text: String,
}

impl DatasetPosition {
    #[must_use]
    pub fn fen(text: impl Into<String>) -> Self {
        Self {
            encoding: PositionEncoding::Fen,
            text: text.into(),
        }
    }

    #[must_use]
    pub fn cgt_canonical(text: impl Into<String>) -> Self {
        Self {
            encoding: PositionEncoding::CgtCanonical,
            text: text.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionEncoding {
    Fen,
    CgtCanonical,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "label_kind")]
pub enum LabelPayload {
    #[serde(rename = "exact")]
    Exact {
        exact: ExactLabel,
        provenance: ExactProvenance,
    },
    #[serde(rename = "rejected")]
    Rejected { rejected: RejectedLabel },
    #[serde(rename = "heuristic")]
    Heuristic { heuristic: HeuristicLabel },
    #[serde(rename = "prediction")]
    Prediction { prediction: PredictionLabel },
}

impl LabelPayload {
    #[must_use]
    pub fn kind(&self) -> LabelKind {
        match self {
            Self::Exact { .. } => LabelKind::Exact,
            Self::Rejected { .. } => LabelKind::Rejected,
            Self::Heuristic { .. } => LabelKind::Heuristic,
            Self::Prediction { .. } => LabelKind::Prediction,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelKind {
    Exact,
    Rejected,
    Heuristic,
    Prediction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactLabel {
    pub status: ExactStatus,
    pub value: BTreeMap<String, String>,
    pub value_class: ExactValueClass,
}

impl ExactLabel {
    #[must_use]
    pub fn verified(value: BTreeMap<String, String>, value_class: ExactValueClass) -> Self {
        Self {
            status: ExactStatus::Verified,
            value,
            value_class,
        }
    }

    #[must_use]
    pub fn from_thermograph_payload(payload: &ThermographExactValuePayload) -> Self {
        Self::verified(
            thermograph_exact_value_map(payload),
            payload.value_class.into(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExactStatus {
    Verified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExactValueClass {
    Number,
    Star,
    Up,
    Down,
    Switch,
    GameTree,
}

impl From<thermograph::ExactValueClass> for ExactValueClass {
    fn from(value_class: thermograph::ExactValueClass) -> Self {
        match value_class {
            thermograph::ExactValueClass::Number => Self::Number,
            thermograph::ExactValueClass::Star => Self::Star,
            thermograph::ExactValueClass::Up => Self::Up,
            thermograph::ExactValueClass::Down => Self::Down,
            thermograph::ExactValueClass::Switch => Self::Switch,
            thermograph::ExactValueClass::GameTree => Self::GameTree,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactProvenance {
    pub code_commit: String,
    pub generator: String,
    pub generator_config_hash: String,
    pub random_seed: u64,
    pub domain_definition: String,
    pub verifier: String,
    pub verifier_version: String,
    pub certificate: LabelCertificate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelCertificate {
    pub kind: String,
    pub digest: String,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub composition: Option<Box<CompositionCertificate>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionCertificate {
    pub decomposition_digest: String,
    pub composition_digest: String,
    pub component_values: BTreeMap<String, String>,
    pub result_value_digest: String,
}

impl LabelCertificate {
    #[must_use]
    pub fn legacy(kind: impl Into<String>, digest: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            digest: digest.into(),
            composition: None,
        }
    }

    #[must_use]
    pub fn composition(
        kind: impl Into<String>,
        digest: impl Into<String>,
        decomposition_digest: impl Into<String>,
        composition_digest: impl Into<String>,
        component_values: BTreeMap<String, String>,
        result_value_digest: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            digest: digest.into(),
            composition: Some(Box::new(CompositionCertificate {
                decomposition_digest: decomposition_digest.into(),
                composition_digest: composition_digest.into(),
                component_values,
                result_value_digest: result_value_digest.into(),
            })),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedLabel {
    pub status: RejectedStatus,
    pub reasons: Vec<String>,
}

impl RejectedLabel {
    #[must_use]
    pub fn unsupported(reasons: Vec<String>) -> Self {
        Self {
            status: RejectedStatus::Unsupported,
            reasons,
        }
    }

    #[must_use]
    pub fn excluded(reasons: Vec<String>) -> Self {
        Self {
            status: RejectedStatus::Excluded,
            reasons,
        }
    }

    #[must_use]
    pub fn error(reasons: Vec<String>) -> Self {
        Self {
            status: RejectedStatus::Error,
            reasons,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectedStatus {
    Unsupported,
    Error,
    Excluded,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeuristicLabel {
    pub method: String,
    pub method_version: String,
    pub outputs: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredictionLabel {
    pub model_id: String,
    pub model_version: String,
    pub checkpoint: String,
    pub outputs: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelValidationIssue {
    pub line_number: Option<usize>,
    pub message: String,
}

impl LabelValidationIssue {
    fn row(message: impl Into<String>) -> Self {
        Self {
            line_number: None,
            message: message.into(),
        }
    }

    fn line(line_number: usize, message: impl Into<String>) -> Self {
        Self {
            line_number: Some(line_number),
            message: message.into(),
        }
    }
}

pub type LabelValidationResult<T> = Result<T, Vec<LabelValidationIssue>>;

pub fn validate_dataset_label_row(row: &DatasetLabelRow) -> LabelValidationResult<()> {
    let mut issues = Vec::new();

    require_non_empty(row.schema_version.as_str(), "schema_version", &mut issues);
    if row.schema_version != DATASET_LABEL_SCHEMA_VERSION {
        issues.push(LabelValidationIssue::row(format!(
            "schema_version must be {DATASET_LABEL_SCHEMA_VERSION:?}"
        )));
    }
    require_non_empty(row.row_id.as_str(), "row_id", &mut issues);
    require_non_empty(row.domain.as_str(), "domain", &mut issues);
    require_non_empty(row.position.text.as_str(), "position.text", &mut issues);

    match &row.label {
        LabelPayload::Exact { exact, provenance } => {
            if exact.status != ExactStatus::Verified {
                issues.push(LabelValidationIssue::row("exact.status must be verified"));
            }
            require_non_empty_map(&exact.value, "exact.value", &mut issues);
            validate_exact_provenance(exact, provenance, &mut issues);
            if is_composition_domain(row.domain.as_str())
                && provenance.certificate.composition.is_none()
            {
                issues.push(LabelValidationIssue::row(
                    "composition exact rows must include structured composition certificate fields",
                ));
            }
        }
        LabelPayload::Rejected { rejected } => {
            if rejected.reasons.is_empty() {
                issues.push(LabelValidationIssue::row(
                    "rejected.reasons must be a non-empty list",
                ));
            }
            for reason in &rejected.reasons {
                require_non_empty(reason.as_str(), "rejected.reasons[]", &mut issues);
            }
        }
        LabelPayload::Heuristic { heuristic } => {
            require_non_empty(heuristic.method.as_str(), "heuristic.method", &mut issues);
            require_non_empty(
                heuristic.method_version.as_str(),
                "heuristic.method_version",
                &mut issues,
            );
            require_non_empty_map(&heuristic.outputs, "heuristic.outputs", &mut issues);
        }
        LabelPayload::Prediction { prediction } => {
            require_non_empty(
                prediction.model_id.as_str(),
                "prediction.model_id",
                &mut issues,
            );
            require_non_empty(
                prediction.model_version.as_str(),
                "prediction.model_version",
                &mut issues,
            );
            require_non_empty(
                prediction.checkpoint.as_str(),
                "prediction.checkpoint",
                &mut issues,
            );
            require_non_empty_map(&prediction.outputs, "prediction.outputs", &mut issues);
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

pub fn serialize_jsonl(rows: &[DatasetLabelRow]) -> Result<String, serde_json::Error> {
    let mut output = String::new();
    for row in rows {
        output.push_str(serde_json::to_string(row)?.as_str());
        output.push('\n');
    }
    Ok(output)
}

pub fn parse_and_validate_jsonl(input: &str) -> LabelValidationResult<Vec<DatasetLabelRow>> {
    let mut rows = Vec::new();
    let mut issues = Vec::new();

    for (index, line) in input.lines().enumerate() {
        let line_number = index + 1;
        if line.trim().is_empty() {
            issues.push(LabelValidationIssue::line(
                line_number,
                "blank JSONL row is ambiguous",
            ));
            continue;
        }

        let value = match serde_json::from_str::<Value>(line) {
            Ok(value) => value,
            Err(error) => {
                issues.push(LabelValidationIssue::line(
                    line_number,
                    format!("invalid JSON: {error}"),
                ));
                continue;
            }
        };

        let raw_issues = validate_raw_payload_shape(&value);
        if !raw_issues.is_empty() {
            issues.extend(
                raw_issues
                    .into_iter()
                    .map(|message| LabelValidationIssue::line(line_number, message)),
            );
            continue;
        }

        match serde_json::from_value::<DatasetLabelRow>(value) {
            Ok(row) => match validate_dataset_label_row(&row) {
                Ok(()) => rows.push(row),
                Err(row_issues) => {
                    issues.extend(row_issues.into_iter().map(|issue| LabelValidationIssue {
                        line_number: Some(line_number),
                        message: issue.message,
                    }));
                }
            },
            Err(error) => issues.push(LabelValidationIssue::line(
                line_number,
                format!("invalid dataset label row: {error}"),
            )),
        }
    }

    if issues.is_empty() {
        Ok(rows)
    } else {
        Err(issues)
    }
}

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
    pub left_signature_profile_count: usize,
    pub right_signature_profile_count: usize,
    pub candidate_pair_counts_by_topology_family: BTreeMap<String, usize>,
    pub selected_row_count: usize,
    pub selected_counts_by_topology_family: BTreeMap<String, usize>,
    pub rejection_counts: BTreeMap<String, usize>,
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

#[must_use]
pub fn generated_depth_two_profile_search_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoProfileSearchReport {
    let mut seed_rows = Vec::new();
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS
            .iter()
            .map(non_fixture_composed_board_exact_row),
    );
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_DOMAIN_CANDIDATES
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                non_fixture_composed_domain_rejected_row(index + 1, candidate)
            }),
    );
    generated_depth_two_profile_search_report_with_seed(&seed_rows, rows_per_family_target)
}

#[must_use]
pub fn generated_depth_two_combined_source_profile_search_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoProfileSourceSearchReport {
    let mut seed_rows = Vec::new();
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS
            .iter()
            .map(non_fixture_composed_board_exact_row),
    );
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_DOMAIN_CANDIDATES
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                non_fixture_composed_domain_rejected_row(index + 1, candidate)
            }),
    );
    GeneratedDepthTwoProfileSourceSearchReport {
        source: "corner_plus_edge_minor_ladder_v0".to_owned(),
        report: generated_depth_two_profile_search_report_with_seed_and_patterns(
            &seed_rows,
            rows_per_family_target,
            generated_combined_component_patterns(
                generated_white_component_patterns(),
                generated_white_edge_minor_ladder_component_patterns(),
            ),
            generated_combined_component_patterns(
                generated_black_component_patterns(),
                generated_black_edge_minor_ladder_component_patterns(),
            ),
        ),
    }
}

#[must_use]
pub fn generated_depth_two_signature_profile_search_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoSignatureProfileSearchReport {
    let mut seed_rows = Vec::new();
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS
            .iter()
            .map(non_fixture_composed_board_exact_row),
    );
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_DOMAIN_CANDIDATES
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                non_fixture_composed_domain_rejected_row(index + 1, candidate)
            }),
    );
    generated_depth_two_signature_profile_search_report_with_seed_and_patterns(
        &seed_rows,
        rows_per_family_target,
        "corner_plus_edge_minor_ladder_v0",
        generated_combined_component_patterns(
            generated_white_component_patterns(),
            generated_white_edge_minor_ladder_component_patterns(),
        ),
        generated_combined_component_patterns(
            generated_black_component_patterns(),
            generated_black_edge_minor_ladder_component_patterns(),
        ),
    )
}

#[must_use]
pub fn generated_depth_two_value_unique_signature_profile_search_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoSignatureProfileSearchReport {
    let mut seed_rows = Vec::new();
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS
            .iter()
            .map(non_fixture_composed_board_exact_row),
    );
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_DOMAIN_CANDIDATES
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                non_fixture_composed_domain_rejected_row(index + 1, candidate)
            }),
    );
    generated_depth_two_value_unique_signature_profile_search_report_with_seed_and_patterns(
        &seed_rows,
        rows_per_family_target,
        "corner_plus_edge_minor_ladder_value_unique_exact_metadata_v0",
        generated_combined_component_patterns(
            generated_white_component_patterns(),
            generated_white_edge_minor_ladder_component_patterns(),
        ),
        generated_combined_component_patterns(
            generated_black_component_patterns(),
            generated_black_edge_minor_ladder_component_patterns(),
        ),
    )
}

#[must_use]
pub fn generated_depth_two_value_unique_signature_upper_bound_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoValueUniqueSignatureUpperBoundReport {
    let mut seed_rows = Vec::new();
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS
            .iter()
            .map(non_fixture_composed_board_exact_row),
    );
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_DOMAIN_CANDIDATES
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                non_fixture_composed_domain_rejected_row(index + 1, candidate)
            }),
    );
    generated_depth_two_value_unique_signature_upper_bound_report_with_patterns(
        &seed_rows,
        rows_per_family_target,
        "corner_plus_edge_minor_ladder_value_unique_exact_metadata_v0",
        generated_combined_component_patterns(
            generated_white_component_patterns(),
            generated_white_edge_minor_ladder_component_patterns(),
        ),
        generated_combined_component_patterns(
            generated_black_component_patterns(),
            generated_black_edge_minor_ladder_component_patterns(),
        ),
    )
}

#[must_use]
pub fn generated_depth_two_value_unique_signature_source_sweep_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoValueUniqueSignatureSourceSweepReport {
    let seed_rows = non_fixture_composed_domain_seed_rows();
    let corner_white_patterns = generated_white_component_patterns();
    let corner_black_patterns = generated_black_component_patterns();
    let edge_white_patterns = generated_white_edge_minor_ladder_component_patterns();
    let edge_black_patterns = generated_black_edge_minor_ladder_component_patterns();
    let rank4_white_patterns = generated_white_rank4_minor_ladder_component_patterns();
    let rank5_black_patterns = generated_black_rank5_minor_ladder_component_patterns();

    let sources = vec![
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "corner_baseline_v0",
            corner_white_patterns.clone(),
            corner_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "edge_minor_ladder_v0",
            edge_white_patterns.clone(),
            edge_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "corner_plus_edge_minor_ladder_v0",
            generated_combined_component_patterns(
                corner_white_patterns.clone(),
                edge_white_patterns.clone(),
            ),
            generated_combined_component_patterns(
                corner_black_patterns.clone(),
                edge_black_patterns.clone(),
            ),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "rank4_minor_ladder_v0",
            rank4_white_patterns.clone(),
            rank5_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "corner_plus_edge_plus_rank4_minor_ladder_v0",
            generated_combined_component_patterns(
                generated_combined_component_patterns(corner_white_patterns, edge_white_patterns),
                rank4_white_patterns,
            ),
            generated_combined_component_patterns(
                generated_combined_component_patterns(corner_black_patterns, edge_black_patterns),
                rank5_black_patterns,
            ),
        ),
    ];

    GeneratedDepthTwoValueUniqueSignatureSourceSweepReport {
        rows_per_family_target,
        sources,
    }
}

#[must_use]
pub fn generated_depth_two_value_unique_signature_source_atlas_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoValueUniqueSignatureSourceSweepReport {
    let seed_rows = non_fixture_composed_domain_seed_rows();
    let corner_white_patterns = generated_white_component_patterns();
    let corner_black_patterns = generated_black_component_patterns();
    let edge_white_patterns = generated_white_edge_minor_ladder_component_patterns();
    let edge_black_patterns = generated_black_edge_minor_ladder_component_patterns();
    let current_white_patterns =
        generated_combined_component_patterns(corner_white_patterns, edge_white_patterns);
    let current_black_patterns =
        generated_combined_component_patterns(corner_black_patterns, edge_black_patterns);
    let cfile_bridge_white_patterns = generated_white_cfile_minor_bridge_component_patterns();
    let ffile_bridge_black_patterns = generated_black_ffile_minor_bridge_component_patterns();
    let wide_shelf_white_patterns = generated_white_wide_pawn_shelf_component_patterns();
    let wide_shelf_black_patterns = generated_black_wide_pawn_shelf_component_patterns();
    let mixed_hook_white_patterns = generated_white_mixed_color_hook_component_patterns();
    let mixed_hook_black_patterns = generated_black_mixed_color_hook_component_patterns();

    let sources = vec![
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "corner_plus_edge_minor_ladder_v0",
            current_white_patterns.clone(),
            current_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "cfile_minor_bridge_v0",
            cfile_bridge_white_patterns.clone(),
            ffile_bridge_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "corner_plus_edge_plus_cfile_minor_bridge_v0",
            generated_combined_component_patterns(
                current_white_patterns.clone(),
                cfile_bridge_white_patterns.clone(),
            ),
            generated_combined_component_patterns(
                current_black_patterns.clone(),
                ffile_bridge_black_patterns.clone(),
            ),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "wide_pawn_shelf_v0",
            wide_shelf_white_patterns.clone(),
            wide_shelf_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "corner_plus_edge_plus_wide_pawn_shelf_v0",
            generated_combined_component_patterns(
                current_white_patterns.clone(),
                wide_shelf_white_patterns.clone(),
            ),
            generated_combined_component_patterns(
                current_black_patterns.clone(),
                wide_shelf_black_patterns.clone(),
            ),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "corner_plus_edge_plus_cfile_bridge_plus_wide_shelf_v0",
            generated_combined_component_patterns(
                generated_combined_component_patterns(
                    current_white_patterns.clone(),
                    cfile_bridge_white_patterns.clone(),
                ),
                wide_shelf_white_patterns.clone(),
            ),
            generated_combined_component_patterns(
                generated_combined_component_patterns(
                    current_black_patterns.clone(),
                    ffile_bridge_black_patterns.clone(),
                ),
                wide_shelf_black_patterns.clone(),
            ),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "mixed_color_hook_v0",
            mixed_hook_white_patterns.clone(),
            mixed_hook_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "corner_plus_edge_plus_mixed_color_hook_v0",
            generated_combined_component_patterns(
                current_white_patterns.clone(),
                mixed_hook_white_patterns.clone(),
            ),
            generated_combined_component_patterns(
                current_black_patterns.clone(),
                mixed_hook_black_patterns.clone(),
            ),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "ordered_all_atlas_truncation_probe_v0",
            generated_combined_component_patterns(
                generated_combined_component_patterns(
                    generated_combined_component_patterns(
                        current_white_patterns,
                        cfile_bridge_white_patterns,
                    ),
                    wide_shelf_white_patterns,
                ),
                mixed_hook_white_patterns,
            ),
            generated_combined_component_patterns(
                generated_combined_component_patterns(
                    generated_combined_component_patterns(
                        current_black_patterns,
                        ffile_bridge_black_patterns,
                    ),
                    wide_shelf_black_patterns,
                ),
                mixed_hook_black_patterns,
            ),
        ),
    ];

    GeneratedDepthTwoValueUniqueSignatureSourceSweepReport {
        rows_per_family_target,
        sources,
    }
}

#[must_use]
pub fn generated_depth_two_value_unique_signature_mixed_hook_upper_bound_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoValueUniqueSignatureUpperBoundReport {
    let seed_rows = non_fixture_composed_domain_seed_rows();
    generated_depth_two_value_unique_signature_upper_bound_report_with_patterns(
        &seed_rows,
        rows_per_family_target,
        "corner_plus_edge_plus_mixed_color_hook_v0",
        generated_combined_component_patterns(
            generated_combined_component_patterns(
                generated_white_component_patterns(),
                generated_white_edge_minor_ladder_component_patterns(),
            ),
            generated_white_mixed_color_hook_component_patterns(),
        ),
        generated_combined_component_patterns(
            generated_combined_component_patterns(
                generated_black_component_patterns(),
                generated_black_edge_minor_ladder_component_patterns(),
            ),
            generated_black_mixed_color_hook_component_patterns(),
        ),
    )
}

#[must_use]
pub fn generated_depth_two_value_unique_signature_expanded_mixed_hook_upper_bound_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoValueUniqueSignatureUpperBoundReport {
    let seed_rows = non_fixture_composed_domain_seed_rows();
    generated_depth_two_value_unique_signature_upper_bound_report_with_patterns(
        &seed_rows,
        rows_per_family_target,
        "corner_plus_edge_plus_expanded_mixed_color_hook_v0",
        generated_combined_component_patterns(
            generated_combined_component_patterns(
                generated_combined_component_patterns(
                    generated_white_component_patterns(),
                    generated_white_edge_minor_ladder_component_patterns(),
                ),
                generated_white_mixed_color_hook_component_patterns(),
            ),
            generated_white_expanded_mixed_color_hook_component_patterns(),
        ),
        generated_combined_component_patterns(
            generated_combined_component_patterns(
                generated_combined_component_patterns(
                    generated_black_component_patterns(),
                    generated_black_edge_minor_ladder_component_patterns(),
                ),
                generated_black_mixed_color_hook_component_patterns(),
            ),
            generated_black_expanded_mixed_color_hook_component_patterns(),
        ),
    )
}

#[must_use]
pub fn generated_depth_two_value_unique_signature_interior_mixed_hook_source_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoValueUniqueSignatureSourceSweepReport {
    let seed_rows = non_fixture_composed_domain_seed_rows();
    let interior_white_patterns = generated_white_interior_mixed_color_hook_component_patterns();
    let interior_black_patterns = generated_black_interior_mixed_color_hook_component_patterns();
    let current_white_patterns = generated_combined_component_patterns(
        generated_combined_component_patterns(
            generated_white_component_patterns(),
            generated_white_edge_minor_ladder_component_patterns(),
        ),
        generated_white_mixed_color_hook_component_patterns(),
    );
    let current_black_patterns = generated_combined_component_patterns(
        generated_combined_component_patterns(
            generated_black_component_patterns(),
            generated_black_edge_minor_ladder_component_patterns(),
        ),
        generated_black_mixed_color_hook_component_patterns(),
    );
    let expanded_white_patterns = generated_combined_component_patterns(
        current_white_patterns.clone(),
        generated_white_expanded_mixed_color_hook_component_patterns(),
    );
    let expanded_black_patterns = generated_combined_component_patterns(
        current_black_patterns.clone(),
        generated_black_expanded_mixed_color_hook_component_patterns(),
    );

    let sources = vec![
        generated_depth_two_value_unique_signature_upper_bound_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "interior_mixed_color_hook_v0",
            interior_white_patterns.clone(),
            interior_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "interior_first_plus_current_mixed_color_hook_v0",
            generated_combined_component_patterns(
                interior_white_patterns.clone(),
                current_white_patterns,
            ),
            generated_combined_component_patterns(
                interior_black_patterns.clone(),
                current_black_patterns,
            ),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "interior_first_plus_expanded_mixed_color_hook_v0",
            generated_combined_component_patterns(interior_white_patterns, expanded_white_patterns),
            generated_combined_component_patterns(interior_black_patterns, expanded_black_patterns),
        ),
    ];

    GeneratedDepthTwoValueUniqueSignatureSourceSweepReport {
        rows_per_family_target,
        sources,
    }
}

#[must_use]
pub fn generated_depth_two_value_unique_signature_pattern_limit_atlas_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoValueUniqueSignatureSourceSweepReport {
    let seed_rows = non_fixture_composed_domain_seed_rows();
    let current_white_patterns = generated_combined_component_patterns(
        generated_combined_component_patterns(
            generated_white_component_patterns(),
            generated_white_edge_minor_ladder_component_patterns(),
        ),
        generated_white_mixed_color_hook_component_patterns(),
    );
    let current_black_patterns = generated_combined_component_patterns(
        generated_combined_component_patterns(
            generated_black_component_patterns(),
            generated_black_edge_minor_ladder_component_patterns(),
        ),
        generated_black_mixed_color_hook_component_patterns(),
    );
    let expanded_white_patterns = generated_combined_component_patterns(
        current_white_patterns.clone(),
        generated_white_expanded_mixed_color_hook_component_patterns(),
    );
    let expanded_black_patterns = generated_combined_component_patterns(
        current_black_patterns.clone(),
        generated_black_expanded_mixed_color_hook_component_patterns(),
    );
    let interior_white_patterns = generated_white_interior_mixed_color_hook_component_patterns();
    let interior_black_patterns = generated_black_interior_mixed_color_hook_component_patterns();
    let unbounded_expanded_interior_white_patterns =
        generated_unbounded_combined_component_patterns(
            generated_unbounded_combined_component_patterns(
                current_white_patterns.clone(),
                generated_white_expanded_mixed_color_hook_component_patterns(),
            ),
            interior_white_patterns.clone(),
        );
    let unbounded_expanded_interior_black_patterns =
        generated_unbounded_combined_component_patterns(
            generated_unbounded_combined_component_patterns(
                current_black_patterns.clone(),
                generated_black_expanded_mixed_color_hook_component_patterns(),
            ),
            interior_black_patterns.clone(),
        );

    let sources = vec![
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "bounded_expanded_mixed_color_hook_v0",
            expanded_white_patterns.clone(),
            expanded_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "bounded_expanded_plus_interior_mixed_color_hook_v0",
            generated_combined_component_patterns(
                expanded_white_patterns.clone(),
                interior_white_patterns.clone(),
            ),
            generated_combined_component_patterns(
                expanded_black_patterns.clone(),
                interior_black_patterns.clone(),
            ),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "bounded_interior_first_plus_expanded_mixed_color_hook_v0",
            generated_combined_component_patterns(interior_white_patterns, expanded_white_patterns),
            generated_combined_component_patterns(interior_black_patterns, expanded_black_patterns),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "unbounded_expanded_plus_interior_mixed_color_hook_v0",
            unbounded_expanded_interior_white_patterns,
            unbounded_expanded_interior_black_patterns,
        ),
    ];

    GeneratedDepthTwoValueUniqueSignatureSourceSweepReport {
        rows_per_family_target,
        sources,
    }
}

#[must_use]
pub fn generated_depth_two_value_unique_signature_left_supply_atlas_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoValueUniqueSignatureSourceSweepReport {
    let seed_rows = non_fixture_composed_domain_seed_rows();
    let current_white_patterns = generated_combined_component_patterns(
        generated_combined_component_patterns(
            generated_white_component_patterns(),
            generated_white_edge_minor_ladder_component_patterns(),
        ),
        generated_white_mixed_color_hook_component_patterns(),
    );
    let current_black_patterns = generated_combined_component_patterns(
        generated_combined_component_patterns(
            generated_black_component_patterns(),
            generated_black_edge_minor_ladder_component_patterns(),
        ),
        generated_black_mixed_color_hook_component_patterns(),
    );
    let expanded_white_patterns = generated_combined_component_patterns(
        current_white_patterns.clone(),
        generated_white_expanded_mixed_color_hook_component_patterns(),
    );
    let expanded_black_patterns = generated_combined_component_patterns(
        current_black_patterns,
        generated_black_expanded_mixed_color_hook_component_patterns(),
    );
    let interior_white_patterns = generated_white_interior_mixed_color_hook_component_patterns();
    let near_wall_white_patterns = generated_white_near_wall_mixed_color_hook_component_patterns();
    let outer_white_patterns = generated_white_outer_mixed_color_hook_component_patterns();
    let diagonal_white_patterns = generated_white_diagonal_mixed_color_hook_component_patterns();
    let unbounded_expanded_interior_white_patterns =
        generated_unbounded_combined_component_patterns(
            generated_unbounded_combined_component_patterns(
                current_white_patterns.clone(),
                generated_white_expanded_mixed_color_hook_component_patterns(),
            ),
            interior_white_patterns.clone(),
        );
    let unbounded_expanded_near_wall_white_patterns =
        generated_unbounded_combined_component_patterns(
            generated_unbounded_combined_component_patterns(
                current_white_patterns.clone(),
                generated_white_expanded_mixed_color_hook_component_patterns(),
            ),
            near_wall_white_patterns.clone(),
        );
    let unbounded_expanded_outer_white_patterns = generated_unbounded_combined_component_patterns(
        generated_unbounded_combined_component_patterns(
            current_white_patterns.clone(),
            generated_white_expanded_mixed_color_hook_component_patterns(),
        ),
        outer_white_patterns.clone(),
    );
    let unbounded_expanded_diagonal_white_patterns =
        generated_unbounded_combined_component_patterns(
            generated_unbounded_combined_component_patterns(
                current_white_patterns.clone(),
                generated_white_expanded_mixed_color_hook_component_patterns(),
            ),
            diagonal_white_patterns.clone(),
        );
    let unbounded_all_left_supply_white_patterns = generated_unbounded_combined_component_patterns(
        generated_unbounded_combined_component_patterns(
            generated_unbounded_combined_component_patterns(
                unbounded_expanded_interior_white_patterns.clone(),
                near_wall_white_patterns,
            ),
            outer_white_patterns,
        ),
        diagonal_white_patterns,
    );

    let sources = vec![
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "bounded_expanded_left_vs_expanded_right_v0",
            expanded_white_patterns,
            expanded_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "unbounded_expanded_plus_interior_left_vs_expanded_right_v0",
            unbounded_expanded_interior_white_patterns,
            expanded_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "near_wall_left_supply_vs_expanded_right_v0",
            generated_white_near_wall_mixed_color_hook_component_patterns(),
            expanded_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "outer_left_supply_vs_expanded_right_v0",
            generated_white_outer_mixed_color_hook_component_patterns(),
            expanded_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "diagonal_left_supply_vs_expanded_right_v0",
            generated_white_diagonal_mixed_color_hook_component_patterns(),
            expanded_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "unbounded_expanded_plus_near_wall_left_vs_expanded_right_v0",
            unbounded_expanded_near_wall_white_patterns,
            expanded_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "unbounded_expanded_plus_outer_left_vs_expanded_right_v0",
            unbounded_expanded_outer_white_patterns,
            expanded_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "unbounded_expanded_plus_diagonal_left_vs_expanded_right_v0",
            unbounded_expanded_diagonal_white_patterns,
            expanded_black_patterns.clone(),
        ),
        generated_depth_two_value_unique_signature_capacity_report_with_patterns(
            &seed_rows,
            rows_per_family_target,
            "unbounded_all_left_supply_vs_expanded_right_v0",
            unbounded_all_left_supply_white_patterns,
            expanded_black_patterns,
        ),
    ];

    GeneratedDepthTwoValueUniqueSignatureSourceSweepReport {
        rows_per_family_target,
        sources,
    }
}

#[must_use]
pub fn generated_depth_two_value_unique_signature_left_supply_bounded_selection_report(
    rows_per_family_target: usize,
    candidate_pair_limit_per_family: usize,
) -> GeneratedDepthTwoSignatureBoundedSupportReport {
    let seed_rows = non_fixture_composed_domain_seed_rows();
    let (white_patterns, black_patterns) =
        generated_left_supply_outer_vs_expanded_right_component_patterns();
    generated_depth_two_value_unique_signature_bounded_support_report_with_seed_and_patterns(
        &seed_rows,
        rows_per_family_target,
        candidate_pair_limit_per_family,
        "unbounded_expanded_plus_outer_left_vs_expanded_right_v0",
        white_patterns,
        black_patterns,
        GeneratedDepthTwoCandidatePairOrder::ProfileIndexSpread,
    )
}

#[must_use]
pub fn generated_depth_two_value_unique_signature_left_supply_value_spread_bounded_selection_report(
    rows_per_family_target: usize,
    candidate_pair_limit_per_family: usize,
) -> GeneratedDepthTwoSignatureBoundedSupportReport {
    let seed_rows = non_fixture_composed_domain_seed_rows();
    let (white_patterns, black_patterns) =
        generated_left_supply_outer_vs_expanded_right_component_patterns();
    generated_depth_two_value_unique_signature_bounded_support_report_with_seed_and_patterns(
        &seed_rows,
        rows_per_family_target,
        candidate_pair_limit_per_family,
        "unbounded_expanded_plus_outer_left_vs_expanded_right_value_spread_v0",
        white_patterns,
        black_patterns,
        GeneratedDepthTwoCandidatePairOrder::ComponentValueDigestSpread,
    )
}

#[must_use]
pub fn generated_depth_two_value_unique_signature_left_supply_dynamic_pairing_preflight_report(
    rows_per_family_target: usize,
) -> GeneratedDepthTwoSignatureProfileSearchReport {
    let seed_rows = non_fixture_composed_domain_seed_rows();
    let (white_patterns, black_patterns) =
        generated_left_supply_outer_vs_expanded_right_component_patterns();
    generated_depth_two_value_unique_signature_dynamic_pairing_preflight_report_with_patterns(
        &seed_rows,
        rows_per_family_target,
        "unbounded_expanded_plus_outer_left_vs_expanded_right_dynamic_pairing_preflight_v0",
        white_patterns,
        black_patterns,
    )
}

#[must_use]
pub fn generated_depth_two_signature_bounded_support_report(
    rows_per_family_target: usize,
    candidate_pair_limit_per_family: usize,
) -> GeneratedDepthTwoSignatureBoundedSupportReport {
    let mut seed_rows = Vec::new();
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS
            .iter()
            .map(non_fixture_composed_board_exact_row),
    );
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_DOMAIN_CANDIDATES
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                non_fixture_composed_domain_rejected_row(index + 1, candidate)
            }),
    );
    generated_depth_two_signature_bounded_support_report_with_seed_and_patterns(
        &seed_rows,
        rows_per_family_target,
        candidate_pair_limit_per_family,
        "corner_plus_edge_minor_ladder_v0",
        generated_combined_component_patterns(
            generated_white_component_patterns(),
            generated_white_edge_minor_ladder_component_patterns(),
        ),
        generated_combined_component_patterns(
            generated_black_component_patterns(),
            generated_black_edge_minor_ladder_component_patterns(),
        ),
    )
}

#[must_use]
pub fn generated_depth_two_profile_inventory_report() -> GeneratedDepthTwoProfileInventoryReport {
    GeneratedDepthTwoProfileInventoryReport {
        white: generated_depth_two_profile_inventory_side(generated_white_component_patterns()),
        black: generated_depth_two_profile_inventory_side(generated_black_component_patterns()),
    }
}

#[must_use]
pub fn generated_depth_two_profile_source_inventory_report()
-> GeneratedDepthTwoProfileSourceInventoryReport {
    GeneratedDepthTwoProfileSourceInventoryReport {
        sources: vec![
            generated_depth_two_named_profile_inventory_report(
                "corner_baseline_v0",
                generated_white_component_patterns(),
                generated_black_component_patterns(),
            ),
            generated_depth_two_named_profile_inventory_report(
                "edge_minor_ladder_v0",
                generated_white_edge_minor_ladder_component_patterns(),
                generated_black_edge_minor_ladder_component_patterns(),
            ),
            generated_depth_two_named_profile_inventory_report(
                "corner_plus_edge_minor_ladder_v0",
                generated_combined_component_patterns(
                    generated_white_component_patterns(),
                    generated_white_edge_minor_ladder_component_patterns(),
                ),
                generated_combined_component_patterns(
                    generated_black_component_patterns(),
                    generated_black_edge_minor_ladder_component_patterns(),
                ),
            ),
        ],
    }
}

#[must_use]
pub fn generated_depth_two_duplicate_cluster_report() -> GeneratedDepthTwoDuplicateClusterReport {
    GeneratedDepthTwoDuplicateClusterReport {
        source: "corner_plus_edge_minor_ladder_v0".to_owned(),
        white: generated_depth_two_duplicate_cluster_side(generated_combined_component_patterns(
            generated_white_component_patterns(),
            generated_white_edge_minor_ladder_component_patterns(),
        )),
        black: generated_depth_two_duplicate_cluster_side(generated_combined_component_patterns(
            generated_black_component_patterns(),
            generated_black_edge_minor_ladder_component_patterns(),
        )),
    }
}

#[must_use]
pub fn generated_depth_three_profile_inventory_report() -> GeneratedLocalMoveProfileInventoryReport
{
    GeneratedLocalMoveProfileInventoryReport {
        component_depth: 3,
        white: generated_local_move_profile_inventory_side(generated_white_component_patterns(), 3),
        black: generated_local_move_profile_inventory_side(generated_black_component_patterns(), 3),
    }
}

pub fn sample_audited_shard_jsonl() -> Result<String, serde_json::Error> {
    serialize_jsonl(&sample_audited_shard())
}

pub fn frontier_audited_shard_jsonl(limit: usize) -> Result<String, serde_json::Error> {
    serialize_jsonl(&frontier_audited_shard(limit))
}

pub fn family_frontier_audited_shard_jsonl(
    limit_per_family: usize,
) -> Result<String, serde_json::Error> {
    serialize_jsonl(&family_frontier_audited_shard(limit_per_family))
}

pub fn expanded_family_frontier_audited_shard_jsonl(
    limit_per_family: usize,
) -> Result<String, serde_json::Error> {
    serialize_jsonl(&expanded_family_frontier_audited_shard(limit_per_family))
}

pub fn composition_hard_target_shard_jsonl(limit: usize) -> Result<String, serde_json::Error> {
    serialize_jsonl(&composition_hard_target_shard(limit))
}

pub fn non_fixture_composed_domain_shard_jsonl(limit: usize) -> Result<String, serde_json::Error> {
    serialize_jsonl(&non_fixture_composed_domain_shard(limit))
}

pub fn expanded_non_fixture_composed_domain_shard_jsonl(
    rows_per_family: usize,
) -> Result<String, serde_json::Error> {
    serialize_jsonl(&expanded_non_fixture_composed_domain_shard(rows_per_family))
}

pub fn leakage_clean_non_fixture_composed_domain_shard_jsonl(
    rows_per_family: usize,
) -> Result<String, serde_json::Error> {
    serialize_jsonl(&leakage_clean_non_fixture_composed_domain_shard(
        rows_per_family,
    ))
}

pub fn signature_target_diagnostic_shard_jsonl(
    rows_per_family: usize,
) -> Result<String, serde_json::Error> {
    serialize_jsonl(&signature_target_diagnostic_shard(rows_per_family))
}

pub fn signature_target_exact_shard_jsonl(
    rows_per_family: usize,
) -> Result<String, serde_json::Error> {
    serialize_jsonl(&signature_target_exact_shard(rows_per_family))
}

pub fn signature_target_mixed_hook_exact_shard_jsonl(
    rows_per_family: usize,
) -> Result<String, serde_json::Error> {
    serialize_jsonl(&signature_target_mixed_hook_exact_shard(rows_per_family))
}

#[must_use]
pub fn sample_audited_shard() -> Vec<DatasetLabelRow> {
    let mut rows = SAMPLE_LABEL_CANDIDATES
        .iter()
        .map(|candidate| candidate.to_row())
        .collect::<Vec<_>>();
    rows.insert(
        2,
        formal_switch_exact_row("astralbase-w5-exact-formal-switch-001"),
    );
    rows
}

#[must_use]
pub fn frontier_audited_shard(limit: usize) -> Vec<DatasetLabelRow> {
    frontier_family_rows(
        "KQK",
        'Q',
        KQK_FRONTIER_EXACT_CONTEXT,
        "astralbase-w6-kqk-frontier",
        limit,
    )
}

#[must_use]
pub fn family_frontier_audited_shard(limit_per_family: usize) -> Vec<DatasetLabelRow> {
    let mut rows = frontier_family_rows(
        "KQK",
        'Q',
        KQK_FRONTIER_EXACT_CONTEXT,
        "astralbase-w7-kqk-frontier",
        limit_per_family,
    );
    rows.extend(frontier_family_rows(
        "KRK",
        'R',
        KRK_FRONTIER_EXACT_CONTEXT,
        "astralbase-w7-krk-frontier",
        limit_per_family,
    ));
    rows
}

#[must_use]
pub fn expanded_family_frontier_audited_shard(limit_per_family: usize) -> Vec<DatasetLabelRow> {
    let mut rows = family_frontier_audited_shard(limit_per_family);
    rows.extend(frontier_family_rows(
        "KBK",
        'B',
        KBK_FRONTIER_EXACT_CONTEXT,
        "astralbase-w12-kbk-frontier",
        limit_per_family,
    ));
    rows.extend(frontier_family_rows(
        "KNK",
        'N',
        KNK_FRONTIER_EXACT_CONTEXT,
        "astralbase-w12-knk-frontier",
        limit_per_family,
    ));
    rows
}

#[must_use]
pub fn composition_hard_target_shard(limit: usize) -> Vec<DatasetLabelRow> {
    let mut rows = Vec::with_capacity(DEFAULT_COMPOSITION_HARD_TARGET_SHARD_LIMIT);

    if let Some(first_exact) = COMPOSITION_FIXTURE_EXACT_SPECS.first() {
        rows.push(composition_fixture_exact_row(first_exact));
    }
    rows.extend(
        COMPOSITION_FIXTURE_REJECTED_CONTROLS
            .iter()
            .take(3)
            .map(|control| {
                composition_fixture_rejected_row(control.row_id, control.fen, control.reason)
            }),
    );
    rows.extend(
        COMPOSITION_FIXTURE_EXACT_SPECS
            .iter()
            .skip(1)
            .map(composition_fixture_exact_row),
    );
    rows.extend(
        COMPOSITION_FIXTURE_REJECTED_CONTROLS
            .iter()
            .skip(3)
            .map(|control| {
                composition_fixture_rejected_row(control.row_id, control.fen, control.reason)
            }),
    );
    rows.truncate(limit);
    rows
}

#[must_use]
pub fn non_fixture_composed_domain_shard(limit: usize) -> Vec<DatasetLabelRow> {
    let mut rows = non_fixture_composed_domain_seed_rows();
    if rows.len() < limit {
        let requested_generated_rows = limit.saturating_sub(rows.len());
        let generated_topology_family_count = generated_depth_two_topology_families().len();
        let requested_rows_per_family =
            requested_generated_rows.div_ceil(generated_topology_family_count);
        rows.extend(generated_depth_two_composed_board_exact_rows(
            &rows,
            requested_rows_per_family.min(GENERATED_DEPTH_TWO_ROWS_PER_TOPOLOGY_FAMILY),
        ));
    }
    rows.truncate(limit);
    rows
}

#[must_use]
pub fn expanded_non_fixture_composed_domain_shard(rows_per_family: usize) -> Vec<DatasetLabelRow> {
    let mut rows = non_fixture_composed_domain_seed_rows();
    rows.extend(generated_depth_two_profiled_composed_board_exact_rows(
        &rows,
        rows_per_family,
    ));
    rows
}

#[must_use]
pub fn leakage_clean_non_fixture_composed_domain_shard(
    rows_per_family: usize,
) -> Vec<DatasetLabelRow> {
    let mut rows = non_fixture_composed_domain_seed_rows();
    rows.extend(generated_depth_two_leakage_clean_composed_board_exact_rows(
        &rows,
        rows_per_family,
    ));
    rows
}

#[must_use]
pub fn signature_target_diagnostic_shard(rows_per_family: usize) -> Vec<DatasetLabelRow> {
    let mut seed_rows = Vec::new();
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS
            .iter()
            .map(non_fixture_composed_board_exact_row),
    );
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_DOMAIN_CANDIDATES
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                non_fixture_composed_domain_rejected_row(index + 1, candidate)
            }),
    );
    let selection = generated_depth_two_signature_profile_selection_with_patterns(
        &seed_rows,
        rows_per_family,
        generated_combined_component_patterns(
            generated_white_component_patterns(),
            generated_white_edge_minor_ladder_component_patterns(),
        ),
        generated_combined_component_patterns(
            generated_black_component_patterns(),
            generated_black_edge_minor_ladder_component_patterns(),
        ),
        None,
    );

    selection
        .candidates
        .iter()
        .map(signature_target_diagnostic_row)
        .collect()
}

#[must_use]
pub fn signature_target_exact_shard(rows_per_family: usize) -> Vec<DatasetLabelRow> {
    let mut seed_rows = Vec::new();
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS
            .iter()
            .map(non_fixture_composed_board_exact_row),
    );
    seed_rows.extend(
        NON_FIXTURE_COMPOSED_DOMAIN_CANDIDATES
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                non_fixture_composed_domain_rejected_row(index + 1, candidate)
            }),
    );
    let selection = generated_depth_two_value_unique_signature_profile_selection_with_patterns(
        &seed_rows,
        rows_per_family,
        generated_combined_component_patterns(
            generated_white_component_patterns(),
            generated_white_edge_minor_ladder_component_patterns(),
        ),
        generated_combined_component_patterns(
            generated_black_component_patterns(),
            generated_black_edge_minor_ladder_component_patterns(),
        ),
        None,
    );
    selection
        .candidates
        .iter()
        .map(signature_target_exact_row)
        .collect()
}

#[must_use]
pub fn signature_target_mixed_hook_exact_shard(rows_per_family: usize) -> Vec<DatasetLabelRow> {
    let seed_rows = non_fixture_composed_domain_seed_rows();
    let selection = generated_depth_two_value_unique_signature_profile_selection_with_patterns(
        &seed_rows,
        rows_per_family,
        generated_combined_component_patterns(
            generated_combined_component_patterns(
                generated_white_component_patterns(),
                generated_white_edge_minor_ladder_component_patterns(),
            ),
            generated_white_mixed_color_hook_component_patterns(),
        ),
        generated_combined_component_patterns(
            generated_combined_component_patterns(
                generated_black_component_patterns(),
                generated_black_edge_minor_ladder_component_patterns(),
            ),
            generated_black_mixed_color_hook_component_patterns(),
        ),
        None,
    );
    selection
        .candidates
        .iter()
        .map(signature_target_mixed_hook_exact_row)
        .collect()
}

fn non_fixture_composed_domain_seed_rows() -> Vec<DatasetLabelRow> {
    let mut rows = Vec::new();
    rows.extend(
        NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS
            .iter()
            .map(non_fixture_composed_board_exact_row),
    );
    rows.extend(
        NON_FIXTURE_COMPOSED_DOMAIN_CANDIDATES
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                non_fixture_composed_domain_rejected_row(index + 1, candidate)
            }),
    );
    rows
}

fn frontier_family_rows(
    family: &'static str,
    white_piece: char,
    context: ExactGenerationContext,
    row_id_prefix: &'static str,
    limit: usize,
) -> Vec<DatasetLabelRow> {
    if limit == 0 {
        return Vec::new();
    }

    let exact_target = frontier_exact_target(limit);
    let rejected_target = limit.saturating_sub(exact_target);
    let mut exact_rows = Vec::with_capacity(exact_target);
    let mut rejected_rows = Vec::with_capacity(rejected_target);

    for (candidate_index, fen) in material_candidate_fens(white_piece).into_iter().enumerate() {
        if exact_rows.len() >= exact_target && rejected_rows.len() >= rejected_target {
            break;
        }

        let row_id = format!("{row_id_prefix}-{candidate_index:06}");
        let Some(row) = generated_material_row(&row_id, fen.as_str(), context, family) else {
            continue;
        };

        match row.label_kind() {
            LabelKind::Exact if exact_rows.len() < exact_target => exact_rows.push(row),
            LabelKind::Rejected if rejected_rows.len() < rejected_target => rejected_rows.push(row),
            _ => {}
        }
    }

    exact_rows.extend(rejected_rows);
    exact_rows.truncate(limit);
    exact_rows
}

fn frontier_exact_target(limit: usize) -> usize {
    if limit == 0 {
        0
    } else {
        limit.min((limit / 5).max(1))
    }
}

fn material_candidate_fens(white_piece: char) -> Vec<String> {
    let mut fens = Vec::new();

    for side_to_move in ["w", "b"] {
        for white_king in 0..64 {
            for black_king in 0..64 {
                if black_king == white_king {
                    continue;
                }
                for white_piece_square in 0..64 {
                    if white_piece_square == white_king || white_piece_square == black_king {
                        continue;
                    }

                    let board = board_fen(&[
                        (white_king, 'K'),
                        (black_king, 'k'),
                        (white_piece_square, white_piece),
                    ]);
                    fens.push(format!("{board} {side_to_move} - - 0 1"));
                }
            }
        }
    }

    fens
}

fn board_fen(pieces: &[(usize, char)]) -> String {
    let mut board = [' '; 64];
    for (square, piece) in pieces {
        board[*square] = *piece;
    }

    let mut ranks = Vec::new();
    for rank in (0..8).rev() {
        let mut rank_text = String::new();
        let mut empty_run = 0;

        for file in 0..8 {
            let square = rank * 8 + file;
            let piece = board[square];
            if piece == ' ' {
                empty_run += 1;
            } else {
                if empty_run > 0 {
                    rank_text.push_str(empty_run.to_string().as_str());
                    empty_run = 0;
                }
                rank_text.push(piece);
            }
        }

        if empty_run > 0 {
            rank_text.push_str(empty_run.to_string().as_str());
        }
        ranks.push(rank_text);
    }

    ranks.join("/")
}

const SAMPLE_LABEL_CANDIDATES: [SampleLabelCandidate; 4] = [
    SampleLabelCandidate {
        row_id: "astralbase-w3-exact-terminal-checkmate-001",
        fen: "7k/5KQ1/8/8/8/8/8/8 b - - 0 1",
    },
    SampleLabelCandidate {
        row_id: "astralbase-w4-exact-mate-in-one-001",
        fen: "7k/5K2/6Q1/8/8/8/8/8 w - - 0 1",
    },
    SampleLabelCandidate {
        row_id: "astralbase-w3-rejected-castling-rights-001",
        fen: "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
    },
    SampleLabelCandidate {
        row_id: "astralbase-w3-rejected-no-strict-decomposition-001",
        fen: "8/8/8/8/8/8/8/4K2k w - - 0 1",
    },
];

#[derive(Clone, Copy, Debug)]
struct SampleLabelCandidate {
    row_id: &'static str,
    fen: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct CompositionFixtureSpec {
    row_id: &'static str,
    topology: CompositionFixtureTopology,
    bottom_square: Square,
    bottom_piece: char,
    top_square: Square,
    top_piece: char,
    value_offset: i32,
    fullmove_number: u32,
    expected_component_count: usize,
}

#[derive(Clone, Copy, Debug)]
struct CompositionFixtureRejectedControl {
    row_id: &'static str,
    fen: &'static str,
    reason: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct NonFixtureComposedDomainCandidate {
    fen: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct NonFixtureComposedBoardSpec<'a> {
    row_id: &'a str,
    active_pieces: &'a [(Square, char)],
    fullmove_number: u32,
    value_rule: NonFixtureComposedBoardValueRule,
    topology_family: &'static str,
    spec_source: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NonFixtureComposedBoardValueRule {
    MaterialBalanceSum,
    AgencyAtomSum,
    LocalMoveGame,
    DepthTwoLocalMoveGame,
}

impl NonFixtureComposedBoardValueRule {
    fn from_composition_value_rule(value: &str) -> Option<Self> {
        match value {
            "component_material_balance_sum_v0" => Some(Self::MaterialBalanceSum),
            "component_agency_atom_sum_v0" => Some(Self::AgencyAtomSum),
            "component_local_move_game_v0" => Some(Self::LocalMoveGame),
            "component_depth2_local_move_game_v0" => Some(Self::DepthTwoLocalMoveGame),
            _ => None,
        }
    }

    fn solver_scope(self) -> &'static str {
        match self {
            Self::MaterialBalanceSum => "composition_board_material_components",
            Self::AgencyAtomSum => "composition_board_agency_atoms",
            Self::LocalMoveGame => "composition_board_component_local_moves",
            Self::DepthTwoLocalMoveGame => "composition_board_component_depth2_local_moves",
        }
    }

    fn composition_value_rule(self) -> &'static str {
        match self {
            Self::MaterialBalanceSum => "component_material_balance_sum_v0",
            Self::AgencyAtomSum => "component_agency_atom_sum_v0",
            Self::LocalMoveGame => "component_local_move_game_v0",
            Self::DepthTwoLocalMoveGame => "component_depth2_local_move_game_v0",
        }
    }

    fn verifier(self) -> &'static str {
        match self {
            Self::MaterialBalanceSum => "bitmesh_conservative_board_material_bmcompose_verifier",
            Self::AgencyAtomSum => "bitmesh_conservative_board_agency_atom_bmcompose_verifier",
            Self::LocalMoveGame => "bitmesh_conservative_component_local_move_bmcompose_verifier",
            Self::DepthTwoLocalMoveGame => {
                "bitmesh_conservative_component_depth2_local_move_bmcompose_verifier"
            }
        }
    }

    fn certificate_kind(self) -> &'static str {
        match self {
            Self::MaterialBalanceSum => {
                "bitmesh-bmcompose-v1+thermograph-exact-value+board-material-v0"
            }
            Self::AgencyAtomSum => {
                "bitmesh-bmcompose-v1+thermograph-exact-value+board-agency-atom-v0"
            }
            Self::LocalMoveGame => {
                "bitmesh-bmcompose-v1+thermograph-exact-value+component-local-move-v0"
            }
            Self::DepthTwoLocalMoveGame => {
                "bitmesh-bmcompose-v1+thermograph-exact-value+component-depth2-local-move-v0"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ComponentLocalMoveCounts {
    white: usize,
    black: usize,
}

#[derive(Clone, Debug)]
struct ComponentValueEvaluation {
    value: CGTValue,
    recursive_depth: Option<u8>,
    recursive_nodes: Option<usize>,
}

#[derive(Clone, Debug)]
struct NonFixtureCompositionReplay {
    exact_values: BTreeMap<String, String>,
    exact_value_class: ExactValueClass,
    verifier: &'static str,
    certificate_kind: &'static str,
    certificate_digest: String,
    decomposition_digest: String,
    composition_digest: String,
    component_values: BTreeMap<String, String>,
    component_value_digests: Vec<String>,
    component_signatures: Vec<String>,
    result_value_digest: String,
}

#[derive(Clone, Debug)]
struct CompositionIdentitySummary {
    decomposition_digest: String,
    composition_digest: String,
    component_identities: Vec<String>,
    component_value_digests: Vec<String>,
    component_value_identities: Vec<String>,
    result_value_digest: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ComponentLocalMove {
    from: Square,
    to: Square,
}

const COMPONENT_DEPTH_TWO_LOCAL_MOVE_DEPTH: u8 = 2;
const MATERIAL_BALANCE_TOPOLOGY_FAMILY: &str = "dfile_two_component_material_balance_v0";
const AGENCY_ATOM_TOPOLOGY_FAMILY: &str = "dfile_two_component_agency_atom_v0";
const LOCAL_MOVE_TOPOLOGY_FAMILY: &str = "dfile_two_component_local_move_v0";
const DEPTH_TWO_LOCAL_MOVE_TOPOLOGY_FAMILY: &str = "dfile_two_component_depth2_local_move_v0";
const DEPTH_TWO_ASYMMETRIC_FAN_TOPOLOGY_FAMILY: &str =
    "dfile_two_component_depth2_asymmetric_fan_v0";
const DEPTH_TWO_PHALANX_TOPOLOGY_FAMILY: &str = "dfile_two_component_depth2_pawn_phalanx_v0";
const CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE: &str = "curated_non_fixture_board_spec_v0";
const PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE: &str =
    "profiled_depth2_component_pair_generator_v0";
const SIGNATURE_TARGET_DIAGNOSTIC_SOURCE: &str = "signature_profile_target_diagnostic_v0";
const SIGNATURE_TARGET_DIAGNOSTIC_ROW_ID_PREFIX: &str =
    "astralbase-w39-signature-target-diagnostic";
const SIGNATURE_TARGET_EXACT_SOURCE: &str = "signature_target_replay_exact_metadata_v0";
const SIGNATURE_TARGET_EXACT_ROW_ID_PREFIX: &str = "astralbase-w47-signature-target-exact";
const SIGNATURE_TARGET_MIXED_HOOK_EXACT_SOURCE: &str =
    "signature_target_mixed_hook_replay_exact_metadata_v0";
const SIGNATURE_TARGET_MIXED_HOOK_DIAGNOSTIC_ROW_ID_PREFIX: &str =
    "astralbase-w52-mixed-hook-signature-target-diagnostic";
const SIGNATURE_TARGET_MIXED_HOOK_EXACT_ROW_ID_PREFIX: &str =
    "astralbase-w52-mixed-hook-signature-target-exact";
const SIGNATURE_TARGET_EXACT_RULE: &str = "depth2_material_mobility_signature_exact_metadata_v0";
const SIGNATURE_TARGET_CONTRACT_ID: &str = "depth2_material_mobility_signature_target_contract_v0";
const SIGNATURE_TARGET_COMPONENT_RULE: &str =
    "depth2_value_digest_plus_material_balance_plus_local_move_counts_v0";
const SIGNATURE_TARGET_PROMOTION_BLOCKERS: &str = "versioned_exact_value_rule_missing;replay_compatible_provenance_missing;split_semantics_missing;deterministic_and_model_baselines_missing";
const SIGNATURE_TARGET_REPLAY_PREFLIGHT_MISMATCH_EXAMPLE_LIMIT: usize = 12;
const SIGNATURE_TARGET_REQUIRED_OUTPUT_FIELDS: [&str; 18] = [
    "active_pieces",
    "component_signature_rule",
    "component_topology_family",
    "composition_spec_source",
    "current_result_value_digest",
    "left_component_signature",
    "left_component_value_digest",
    "left_profile_index",
    "promotion_blockers",
    "result_signature_key",
    "right_component_signature",
    "right_component_value_digest",
    "right_profile_index",
    "row_number",
    "supervision_eligible",
    "target_contract_id",
    "target_status",
    "total_recursive_nodes",
];
const GENERATED_DEPTH_TWO_ROWS_PER_TOPOLOGY_FAMILY: usize = 2;
const GENERATED_DEPTH_TWO_MAX_COMPONENT_RECURSIVE_NODES: usize = 220;
const GENERATED_DEPTH_TWO_MAX_RECURSIVE_NODES: usize = 1_000;
const GENERATED_DEPTH_TWO_START_FULLMOVE: u32 = 200;
const GENERATED_DEPTH_TWO_COMPONENT_PATTERN_LIMIT: usize = 1_536;
const GENERATED_DEPTH_TWO_COMPONENT_PATTERN_GROUP_LIMIT: usize = 512;
const GENERATED_DEPTH_TWO_DUPLICATE_CLUSTER_REPORT_LIMIT: usize = 16;
const GENERATED_DEPTH_TWO_DUPLICATE_CLUSTER_EXAMPLE_LIMIT: usize = 5;

fn generated_depth_two_topology_families() -> [&'static str; 3] {
    [
        DEPTH_TWO_LOCAL_MOVE_TOPOLOGY_FAMILY,
        DEPTH_TWO_ASYMMETRIC_FAN_TOPOLOGY_FAMILY,
        DEPTH_TWO_PHALANX_TOPOLOGY_FAMILY,
    ]
}

#[derive(Clone, Debug)]
struct GeneratedDepthTwoComponentProfile {
    active_pieces: Vec<(Square, char)>,
    value: CGTValue,
    value_digest: String,
    recursive_nodes: usize,
}

#[derive(Clone, Debug)]
struct GeneratedDepthTwoSignatureProfile {
    active_pieces: Vec<(Square, char)>,
    value: CGTValue,
    value_digest: String,
    component_signature: String,
    recursive_nodes: usize,
}

#[derive(Clone, Debug)]
struct GeneratedDepthTwoSelectedProfileCandidate {
    row_number: usize,
    topology_family: &'static str,
    left_profile_index: usize,
    right_profile_index: usize,
    total_recursive_nodes: usize,
    active_pieces: Vec<(Square, char)>,
    left_component_value_digest: String,
    right_component_value_digest: String,
    result_value_digest: String,
}

#[derive(Clone, Debug)]
struct GeneratedDepthTwoSelectedSignatureCandidate {
    row_number: usize,
    topology_family: &'static str,
    left_profile_index: usize,
    right_profile_index: usize,
    total_recursive_nodes: usize,
    active_pieces: Vec<(Square, char)>,
    left_component_value_digest: String,
    right_component_value_digest: String,
    left_component_signature: String,
    right_component_signature: String,
    result_signature_key: String,
    current_result_value_digest: String,
}

#[derive(Clone, Debug)]
struct GeneratedDepthTwoProfileSelection {
    left_profile_count: usize,
    right_profile_count: usize,
    candidate_pair_counts: Vec<usize>,
    family_counts: Vec<usize>,
    rejection_counts: BTreeMap<String, usize>,
    candidates: Vec<GeneratedDepthTwoSelectedProfileCandidate>,
}

#[derive(Clone, Debug)]
struct GeneratedDepthTwoSignatureProfileSelection {
    left_signature_profile_count: usize,
    right_signature_profile_count: usize,
    candidate_pair_counts: Vec<usize>,
    candidate_offsets: Vec<usize>,
    family_counts: Vec<usize>,
    rejection_counts: BTreeMap<String, usize>,
    candidates: Vec<GeneratedDepthTwoSelectedSignatureCandidate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GeneratedDepthTwoCandidatePairOrder {
    ProfileIndexSpread,
    ComponentValueDigestSpread,
}

#[derive(Clone, Copy, Debug)]
enum CompositionFixtureTopology {
    SingleHorizontalWall,
    DoubleHorizontalWall,
}

const COMPOSITION_FIXTURE_EXACT_SPECS: [CompositionFixtureSpec; 16] = [
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-001",
        topology: CompositionFixtureTopology::SingleHorizontalWall,
        bottom_square: Square::A1,
        bottom_piece: 'N',
        top_square: Square::H8,
        top_piece: 'n',
        value_offset: 0,
        fullmove_number: 1,
        expected_component_count: 2,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-002",
        topology: CompositionFixtureTopology::SingleHorizontalWall,
        bottom_square: Square::B1,
        bottom_piece: 'B',
        top_square: Square::G8,
        top_piece: 'r',
        value_offset: 2,
        fullmove_number: 2,
        expected_component_count: 2,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-003",
        topology: CompositionFixtureTopology::SingleHorizontalWall,
        bottom_square: Square::C1,
        bottom_piece: 'R',
        top_square: Square::F8,
        top_piece: 'b',
        value_offset: 4,
        fullmove_number: 3,
        expected_component_count: 2,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-004",
        topology: CompositionFixtureTopology::SingleHorizontalWall,
        bottom_square: Square::D1,
        bottom_piece: 'Q',
        top_square: Square::E8,
        top_piece: 'q',
        value_offset: 6,
        fullmove_number: 4,
        expected_component_count: 2,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-005",
        topology: CompositionFixtureTopology::SingleHorizontalWall,
        bottom_square: Square::A2,
        bottom_piece: 'N',
        top_square: Square::H7,
        top_piece: 'n',
        value_offset: 8,
        fullmove_number: 5,
        expected_component_count: 2,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-006",
        topology: CompositionFixtureTopology::SingleHorizontalWall,
        bottom_square: Square::B2,
        bottom_piece: 'B',
        top_square: Square::G7,
        top_piece: 'r',
        value_offset: 10,
        fullmove_number: 6,
        expected_component_count: 2,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-007",
        topology: CompositionFixtureTopology::SingleHorizontalWall,
        bottom_square: Square::C2,
        bottom_piece: 'R',
        top_square: Square::F7,
        top_piece: 'b',
        value_offset: 12,
        fullmove_number: 7,
        expected_component_count: 2,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-008",
        topology: CompositionFixtureTopology::SingleHorizontalWall,
        bottom_square: Square::D2,
        bottom_piece: 'Q',
        top_square: Square::E7,
        top_piece: 'q',
        value_offset: 14,
        fullmove_number: 8,
        expected_component_count: 2,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-009",
        topology: CompositionFixtureTopology::SingleHorizontalWall,
        bottom_square: Square::A3,
        bottom_piece: 'N',
        top_square: Square::H6,
        top_piece: 'n',
        value_offset: 16,
        fullmove_number: 9,
        expected_component_count: 2,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-010",
        topology: CompositionFixtureTopology::SingleHorizontalWall,
        bottom_square: Square::B3,
        bottom_piece: 'B',
        top_square: Square::G6,
        top_piece: 'r',
        value_offset: 18,
        fullmove_number: 10,
        expected_component_count: 2,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-011",
        topology: CompositionFixtureTopology::SingleHorizontalWall,
        bottom_square: Square::C3,
        bottom_piece: 'R',
        top_square: Square::F6,
        top_piece: 'b',
        value_offset: 20,
        fullmove_number: 11,
        expected_component_count: 2,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-012",
        topology: CompositionFixtureTopology::SingleHorizontalWall,
        bottom_square: Square::D3,
        bottom_piece: 'Q',
        top_square: Square::E6,
        top_piece: 'q',
        value_offset: 22,
        fullmove_number: 12,
        expected_component_count: 2,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-013",
        topology: CompositionFixtureTopology::DoubleHorizontalWall,
        bottom_square: Square::A1,
        bottom_piece: 'N',
        top_square: Square::H8,
        top_piece: 'n',
        value_offset: 24,
        fullmove_number: 13,
        expected_component_count: 3,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-014",
        topology: CompositionFixtureTopology::DoubleHorizontalWall,
        bottom_square: Square::B1,
        bottom_piece: 'B',
        top_square: Square::G8,
        top_piece: 'r',
        value_offset: 27,
        fullmove_number: 14,
        expected_component_count: 3,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-015",
        topology: CompositionFixtureTopology::DoubleHorizontalWall,
        bottom_square: Square::C1,
        bottom_piece: 'R',
        top_square: Square::F8,
        top_piece: 'b',
        value_offset: 30,
        fullmove_number: 15,
        expected_component_count: 3,
    },
    CompositionFixtureSpec {
        row_id: "astralbase-w17-composition-exact-wall-016",
        topology: CompositionFixtureTopology::DoubleHorizontalWall,
        bottom_square: Square::D1,
        bottom_piece: 'Q',
        top_square: Square::E8,
        top_piece: 'q',
        value_offset: 33,
        fullmove_number: 16,
        expected_component_count: 3,
    },
];

const COMPOSITION_FIXTURE_REJECTED_CONTROLS: [CompositionFixtureRejectedControl; 5] = [
    CompositionFixtureRejectedControl {
        row_id: "astralbase-w17-composition-rejected-weak-decomposition-001",
        fen: "8/8/8/8/8/8/8/4K2k w - - 0 1",
        reason: "weak_decomposition: no strict decomposition certificate is available",
    },
    CompositionFixtureRejectedControl {
        row_id: "astralbase-w17-composition-rejected-missing-component-value-001",
        fen: COMPOSITION_FIXTURE_MISSING_COMPONENT_FEN,
        reason: "missing_component_value_digest: every strict component root needs a verified value digest",
    },
    CompositionFixtureRejectedControl {
        row_id: "astralbase-w17-composition-rejected-stale-composition-001",
        fen: COMPOSITION_FIXTURE_STALE_COMPOSITION_FEN,
        reason: "stale_composition_digest: BMCOMPOSE digest does not match the referenced decomposition and component values",
    },
    CompositionFixtureRejectedControl {
        row_id: "astralbase-w17-composition-rejected-duplicate-root-001",
        fen: COMPOSITION_FIXTURE_DUPLICATE_ROOT_FEN,
        reason: "duplicate_component_root: component roots must be unique and match strict decomposition coverage",
    },
    CompositionFixtureRejectedControl {
        row_id: "astralbase-w17-composition-rejected-unsupported-value-001",
        fen: COMPOSITION_FIXTURE_UNSUPPORTED_VALUE_FEN,
        reason: "unsupported_composition_value: approximate or unsupported values cannot produce exact BMCOMPOSE labels",
    },
];

const NON_FIXTURE_COMPOSED_DOMAIN_CANDIDATES: [NonFixtureComposedDomainCandidate; 3] = [
    NonFixtureComposedDomainCandidate {
        fen: "7k/5K2/6Q1/8/8/8/8/8 w - - 0 1",
    },
    NonFixtureComposedDomainCandidate {
        fen: "8/8/8/8/8/8/8/4K2k w - - 0 1",
    },
    NonFixtureComposedDomainCandidate {
        fen: "8/8/8/8/4Q3/8/8/4K2k w - - 0 1",
    },
];

const NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS: [NonFixtureComposedBoardSpec<'static>; 11] = [
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-001",
        active_pieces: &[(Square::A1, 'N'), (Square::H8, 'n'), (Square::G7, 'p')],
        fullmove_number: 101,
        value_rule: NonFixtureComposedBoardValueRule::MaterialBalanceSum,
        topology_family: MATERIAL_BALANCE_TOPOLOGY_FAMILY,
        spec_source: CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-002",
        active_pieces: &[
            (Square::A1, 'N'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::H8, 'p'),
            (Square::G7, 'p'),
        ],
        fullmove_number: 102,
        value_rule: NonFixtureComposedBoardValueRule::MaterialBalanceSum,
        topology_family: MATERIAL_BALANCE_TOPOLOGY_FAMILY,
        spec_source: CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-003",
        active_pieces: &[(Square::A2, 'n'), (Square::H7, 'N'), (Square::G6, 'P')],
        fullmove_number: 103,
        value_rule: NonFixtureComposedBoardValueRule::MaterialBalanceSum,
        topology_family: MATERIAL_BALANCE_TOPOLOGY_FAMILY,
        spec_source: CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-004",
        active_pieces: &[(Square::A1, 'N'), (Square::A2, 'P'), (Square::H8, 'n')],
        fullmove_number: 104,
        value_rule: NonFixtureComposedBoardValueRule::AgencyAtomSum,
        topology_family: AGENCY_ATOM_TOPOLOGY_FAMILY,
        spec_source: CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-005",
        active_pieces: &[
            (Square::A1, 'N'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::H8, 'n'),
        ],
        fullmove_number: 105,
        value_rule: NonFixtureComposedBoardValueRule::LocalMoveGame,
        topology_family: LOCAL_MOVE_TOPOLOGY_FAMILY,
        spec_source: CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-006",
        active_pieces: &[
            (Square::A1, 'N'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::H8, 'n'),
            (Square::H7, 'p'),
        ],
        fullmove_number: 106,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_LOCAL_MOVE_TOPOLOGY_FAMILY,
        spec_source: CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-007",
        active_pieces: &[
            (Square::A1, 'N'),
            (Square::A2, 'P'),
            (Square::H8, 'n'),
            (Square::H7, 'p'),
            (Square::G7, 'p'),
        ],
        fullmove_number: 107,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_ASYMMETRIC_FAN_TOPOLOGY_FAMILY,
        spec_source: CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-008",
        active_pieces: &[
            (Square::A1, 'N'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
            (Square::H8, 'n'),
            (Square::H7, 'p'),
            (Square::G7, 'p'),
            (Square::F7, 'p'),
        ],
        fullmove_number: 108,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_PHALANX_TOPOLOGY_FAMILY,
        spec_source: CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-009",
        active_pieces: &[
            (Square::A1, 'N'),
            (Square::B1, 'B'),
            (Square::C2, 'P'),
            (Square::H8, 'n'),
            (Square::G8, 'b'),
            (Square::F7, 'p'),
        ],
        fullmove_number: 109,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_LOCAL_MOVE_TOPOLOGY_FAMILY,
        spec_source: CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-010",
        active_pieces: &[
            (Square::A1, 'N'),
            (Square::B1, 'B'),
            (Square::A2, 'P'),
            (Square::C2, 'P'),
            (Square::H8, 'n'),
            (Square::G8, 'b'),
            (Square::H7, 'p'),
            (Square::F7, 'p'),
        ],
        fullmove_number: 110,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_ASYMMETRIC_FAN_TOPOLOGY_FAMILY,
        spec_source: CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-011",
        active_pieces: &[
            (Square::A1, 'N'),
            (Square::B1, 'B'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
            (Square::H8, 'n'),
            (Square::G8, 'b'),
            (Square::H7, 'p'),
            (Square::G7, 'p'),
            (Square::F7, 'p'),
        ],
        fullmove_number: 111,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_PHALANX_TOPOLOGY_FAMILY,
        spec_source: CURATED_NON_FIXTURE_BOARD_SPEC_SOURCE,
    },
];

const LEGACY_GENERATED_DEPTH_TWO_EXACT_SPECS: [NonFixtureComposedBoardSpec<'static>; 6] = [
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-012",
        active_pieces: &[
            (Square::A1, 'R'),
            (Square::B1, 'B'),
            (Square::A2, 'P'),
            (Square::C2, 'P'),
            (Square::F7, 'p'),
            (Square::G7, 'p'),
            (Square::G8, 'b'),
            (Square::H8, 'q'),
        ],
        fullmove_number: 212,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_LOCAL_MOVE_TOPOLOGY_FAMILY,
        spec_source: PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-013",
        active_pieces: &[
            (Square::A1, 'Q'),
            (Square::B1, 'B'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
            (Square::F7, 'p'),
            (Square::G7, 'p'),
            (Square::H7, 'p'),
            (Square::G8, 'b'),
            (Square::H8, 'q'),
        ],
        fullmove_number: 213,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_ASYMMETRIC_FAN_TOPOLOGY_FAMILY,
        spec_source: PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-014",
        active_pieces: &[
            (Square::A1, 'Q'),
            (Square::B1, 'B'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
            (Square::H7, 'n'),
            (Square::H8, 'q'),
        ],
        fullmove_number: 214,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_PHALANX_TOPOLOGY_FAMILY,
        spec_source: PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-015",
        active_pieces: &[
            (Square::A1, 'R'),
            (Square::B1, 'B'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
            (Square::G7, 'p'),
            (Square::H7, 'p'),
            (Square::G8, 'r'),
            (Square::H8, 'q'),
        ],
        fullmove_number: 215,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_LOCAL_MOVE_TOPOLOGY_FAMILY,
        spec_source: PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-016",
        active_pieces: &[
            (Square::A1, 'R'),
            (Square::B1, 'R'),
            (Square::A2, 'P'),
            (Square::C2, 'P'),
            (Square::G7, 'p'),
            (Square::H7, 'p'),
            (Square::H8, 'q'),
        ],
        fullmove_number: 216,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_ASYMMETRIC_FAN_TOPOLOGY_FAMILY,
        spec_source: PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-017",
        active_pieces: &[
            (Square::A1, 'Q'),
            (Square::B1, 'R'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
            (Square::F7, 'p'),
            (Square::H7, 'p'),
            (Square::G8, 'b'),
            (Square::H8, 'r'),
        ],
        fullmove_number: 217,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_PHALANX_TOPOLOGY_FAMILY,
        spec_source: PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE,
    },
];

fn generated_depth_two_composed_board_exact_rows(
    seed_rows: &[DatasetLabelRow],
    rows_per_family: usize,
) -> Vec<DatasetLabelRow> {
    let topology_families = generated_depth_two_topology_families();

    let mut rows = Vec::with_capacity(rows_per_family * topology_families.len());
    let mut seen_positions = BTreeSet::new();
    let mut seen_decomposition_digests = BTreeSet::new();
    let mut seen_component_digests = BTreeSet::new();
    let mut seen_result_digests = BTreeSet::new();
    for row in seed_rows {
        seen_positions.insert(row.position.text.clone());
        if let Some((decomposition_digest, result_digest, component_digests)) =
            composition_digest_summary_for_row(row)
        {
            seen_decomposition_digests.insert(decomposition_digest);
            seen_result_digests.insert(result_digest);
            for digest in component_digests {
                seen_component_digests.insert(digest);
            }
        }
    }

    let mut family_counts = vec![0usize; topology_families.len()];
    for spec in LEGACY_GENERATED_DEPTH_TWO_EXACT_SPECS {
        let topology_family = spec.topology_family;
        let Some(family_index) = topology_families
            .iter()
            .position(|family| *family == topology_family)
        else {
            continue;
        };
        if family_counts[family_index] == rows_per_family {
            continue;
        }
        let Ok(row) = try_non_fixture_composed_board_exact_row(&spec) else {
            continue;
        };
        if !generated_depth_two_row_within_node_budget(&row) {
            continue;
        }
        let Some((decomposition_digest, result_digest, component_digests)) =
            composition_digest_summary_for_row(&row)
        else {
            continue;
        };
        if component_digests.iter().collect::<BTreeSet<_>>().len() != component_digests.len() {
            continue;
        }
        if seen_decomposition_digests.contains(&decomposition_digest)
            || seen_result_digests.contains(&result_digest)
            || component_digests
                .iter()
                .any(|digest| seen_component_digests.contains(digest))
            || seen_positions.contains(&row.position.text)
        {
            continue;
        }

        seen_positions.insert(row.position.text.clone());
        seen_decomposition_digests.insert(decomposition_digest);
        seen_result_digests.insert(result_digest);
        for digest in component_digests {
            seen_component_digests.insert(digest);
        }
        rows.push(row);
        family_counts[family_index] += 1;
    }

    rows
}

fn generated_depth_two_profiled_composed_board_exact_rows(
    seed_rows: &[DatasetLabelRow],
    rows_per_family: usize,
) -> Vec<DatasetLabelRow> {
    let selection = generated_depth_two_profile_selection(seed_rows, rows_per_family, true);

    let mut rows = Vec::with_capacity(selection.candidates.len());
    let mut seen_positions = BTreeSet::new();
    let mut seed_component_digests = BTreeSet::new();
    let mut seen_result_digests = BTreeSet::new();
    for row in seed_rows {
        seen_positions.insert(row.position.text.clone());
        if let Some((_decomposition_digest, result_digest, component_digests)) =
            composition_digest_summary_for_row(row)
        {
            seen_result_digests.insert(result_digest);
            for digest in component_digests {
                seed_component_digests.insert(digest);
            }
        }
    }

    for candidate in selection.candidates {
        let row_id = format!(
            "{}-{:03}",
            NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG.row_id_prefix, candidate.row_number
        );
        let spec = NonFixtureComposedBoardSpec {
            row_id: &row_id,
            active_pieces: &candidate.active_pieces,
            fullmove_number: GENERATED_DEPTH_TWO_START_FULLMOVE
                + u32::try_from(candidate.row_number)
                    .expect("generated row number fits fullmove u32"),
            value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
            topology_family: candidate.topology_family,
            spec_source: PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE,
        };
        let Ok(row) = try_non_fixture_composed_board_exact_row(&spec) else {
            continue;
        };
        if !generated_depth_two_row_within_node_budget(&row) {
            continue;
        }
        let Some((_decomposition_digest, result_digest, component_digests)) =
            composition_digest_summary_for_row(&row)
        else {
            continue;
        };
        if component_digests.iter().collect::<BTreeSet<_>>().len() != component_digests.len() {
            continue;
        }
        if seen_result_digests.contains(&result_digest)
            || component_digests
                .iter()
                .any(|digest| seed_component_digests.contains(digest))
            || seen_positions.contains(&row.position.text)
        {
            continue;
        }

        seen_positions.insert(row.position.text.clone());
        seen_result_digests.insert(result_digest);
        rows.push(row);
    }

    rows
}

fn generated_depth_two_leakage_clean_composed_board_exact_rows(
    seed_rows: &[DatasetLabelRow],
    rows_per_family: usize,
) -> Vec<DatasetLabelRow> {
    let selection = generated_depth_two_leakage_clean_profile_selection(seed_rows, rows_per_family);

    let mut rows = Vec::with_capacity(selection.candidates.len());
    let mut seen_positions = BTreeSet::new();
    let mut seen_decomposition_digests = BTreeSet::new();
    let mut seen_composition_digests = BTreeSet::new();
    let mut seen_component_identities = BTreeSet::new();
    let mut seen_component_value_digests = BTreeSet::new();
    let mut seen_component_value_identities = BTreeSet::new();
    let mut seen_result_digests = BTreeSet::new();
    for row in seed_rows {
        seen_positions.insert(row.position.text.clone());
        if let Some(summary) = composition_identity_summary_for_row(row) {
            seen_decomposition_digests.insert(summary.decomposition_digest);
            seen_composition_digests.insert(summary.composition_digest);
            seen_result_digests.insert(summary.result_value_digest);
            seen_component_identities.extend(summary.component_identities);
            seen_component_value_digests.extend(summary.component_value_digests);
            seen_component_value_identities.extend(summary.component_value_identities);
        }
    }

    for candidate in selection.candidates {
        let row_id = format!(
            "{}-{:03}",
            NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG.row_id_prefix, candidate.row_number
        );
        let spec = NonFixtureComposedBoardSpec {
            row_id: &row_id,
            active_pieces: &candidate.active_pieces,
            fullmove_number: GENERATED_DEPTH_TWO_START_FULLMOVE
                + u32::try_from(candidate.row_number)
                    .expect("generated row number fits fullmove u32"),
            value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
            topology_family: candidate.topology_family,
            spec_source: PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE,
        };
        let Ok(row) = try_non_fixture_composed_board_exact_row(&spec) else {
            continue;
        };
        if !generated_depth_two_row_within_node_budget(&row) {
            continue;
        }
        let Some(summary) = composition_identity_summary_for_row(&row) else {
            continue;
        };
        if has_duplicates(&summary.component_identities)
            || has_duplicates(&summary.component_value_digests)
            || has_duplicates(&summary.component_value_identities)
        {
            continue;
        }
        if seen_positions.contains(&row.position.text)
            || seen_decomposition_digests.contains(&summary.decomposition_digest)
            || seen_composition_digests.contains(&summary.composition_digest)
            || seen_result_digests.contains(&summary.result_value_digest)
            || summary
                .component_identities
                .iter()
                .any(|identity| seen_component_identities.contains(identity))
            || summary
                .component_value_digests
                .iter()
                .any(|digest| seen_component_value_digests.contains(digest))
            || summary
                .component_value_identities
                .iter()
                .any(|identity| seen_component_value_identities.contains(identity))
        {
            continue;
        }

        seen_positions.insert(row.position.text.clone());
        seen_decomposition_digests.insert(summary.decomposition_digest);
        seen_composition_digests.insert(summary.composition_digest);
        seen_result_digests.insert(summary.result_value_digest);
        seen_component_identities.extend(summary.component_identities);
        seen_component_value_digests.extend(summary.component_value_digests);
        seen_component_value_identities.extend(summary.component_value_identities);
        rows.push(row);
    }

    rows
}

fn signature_target_diagnostic_row(
    candidate: &GeneratedDepthTwoSelectedSignatureCandidate,
) -> DatasetLabelRow {
    let row_id = format!(
        "{}-{:03}",
        SIGNATURE_TARGET_DIAGNOSTIC_ROW_ID_PREFIX, candidate.row_number
    );
    let spec = NonFixtureComposedBoardSpec {
        row_id: &row_id,
        active_pieces: &candidate.active_pieces,
        fullmove_number: GENERATED_DEPTH_TWO_START_FULLMOVE
            + u32::try_from(candidate.row_number).expect("generated row number fits fullmove u32"),
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: candidate.topology_family,
        spec_source: SIGNATURE_TARGET_DIAGNOSTIC_SOURCE,
    };
    let fen = non_fixture_composed_board_fen(&spec);
    DatasetLabelRow::heuristic(
        row_id,
        NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID,
        DatasetPosition::fen(fen),
        HeuristicLabel {
            method: "signature_profile_target_diagnostic".to_owned(),
            method_version: "v0".to_owned(),
            outputs: BTreeMap::from([
                (
                    "target_contract_id".to_owned(),
                    SIGNATURE_TARGET_CONTRACT_ID.to_owned(),
                ),
                ("target_status".to_owned(), "diagnostic_only".to_owned()),
                ("supervision_eligible".to_owned(), "false".to_owned()),
                (
                    "component_signature_rule".to_owned(),
                    SIGNATURE_TARGET_COMPONENT_RULE.to_owned(),
                ),
                (
                    "promotion_blockers".to_owned(),
                    SIGNATURE_TARGET_PROMOTION_BLOCKERS.to_owned(),
                ),
                (
                    "component_topology_family".to_owned(),
                    candidate.topology_family.to_owned(),
                ),
                (
                    "composition_spec_source".to_owned(),
                    SIGNATURE_TARGET_DIAGNOSTIC_SOURCE.to_owned(),
                ),
                ("row_number".to_owned(), candidate.row_number.to_string()),
                (
                    "left_profile_index".to_owned(),
                    candidate.left_profile_index.to_string(),
                ),
                (
                    "right_profile_index".to_owned(),
                    candidate.right_profile_index.to_string(),
                ),
                (
                    "total_recursive_nodes".to_owned(),
                    candidate.total_recursive_nodes.to_string(),
                ),
                (
                    "active_pieces".to_owned(),
                    generated_depth_two_active_piece_summary(&candidate.active_pieces),
                ),
                (
                    "left_component_value_digest".to_owned(),
                    candidate.left_component_value_digest.clone(),
                ),
                (
                    "right_component_value_digest".to_owned(),
                    candidate.right_component_value_digest.clone(),
                ),
                (
                    "current_result_value_digest".to_owned(),
                    candidate.current_result_value_digest.clone(),
                ),
                (
                    "left_component_signature".to_owned(),
                    candidate.left_component_signature.clone(),
                ),
                (
                    "right_component_signature".to_owned(),
                    candidate.right_component_signature.clone(),
                ),
                (
                    "result_signature_key".to_owned(),
                    candidate.result_signature_key.clone(),
                ),
            ]),
        },
    )
}

fn signature_target_exact_row(
    candidate: &GeneratedDepthTwoSelectedSignatureCandidate,
) -> DatasetLabelRow {
    signature_target_exact_row_with_context(
        candidate,
        SIGNATURE_TARGET_EXACT_ROW_ID_PREFIX,
        SIGNATURE_TARGET_DIAGNOSTIC_ROW_ID_PREFIX,
        SIGNATURE_TARGET_EXACT_SOURCE,
    )
}

fn signature_target_mixed_hook_exact_row(
    candidate: &GeneratedDepthTwoSelectedSignatureCandidate,
) -> DatasetLabelRow {
    signature_target_exact_row_with_context(
        candidate,
        SIGNATURE_TARGET_MIXED_HOOK_EXACT_ROW_ID_PREFIX,
        SIGNATURE_TARGET_MIXED_HOOK_DIAGNOSTIC_ROW_ID_PREFIX,
        SIGNATURE_TARGET_MIXED_HOOK_EXACT_SOURCE,
    )
}

fn signature_target_exact_row_with_context(
    candidate: &GeneratedDepthTwoSelectedSignatureCandidate,
    row_id_prefix: &str,
    diagnostic_row_id_prefix: &str,
    spec_source: &'static str,
) -> DatasetLabelRow {
    let row_id = format!("{}-{:03}", row_id_prefix, candidate.row_number);
    let spec = NonFixtureComposedBoardSpec {
        row_id: &row_id,
        active_pieces: &candidate.active_pieces,
        fullmove_number: GENERATED_DEPTH_TWO_START_FULLMOVE
            + u32::try_from(candidate.row_number).expect("generated row number fits fullmove u32"),
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: candidate.topology_family,
        spec_source,
    };
    let mut row = try_non_fixture_composed_board_exact_row(&spec)
        .expect("signature target exact row must replay");
    let LabelPayload::Exact { exact, .. } = &mut row.label else {
        panic!("signature target exact row generator must emit exact rows");
    };
    exact.value.extend(BTreeMap::from([
        (
            "signature_target_rule".to_owned(),
            SIGNATURE_TARGET_EXACT_RULE.to_owned(),
        ),
        (
            "signature_target_contract_id".to_owned(),
            SIGNATURE_TARGET_CONTRACT_ID.to_owned(),
        ),
        (
            "signature_target_status".to_owned(),
            "replay_exact_metadata".to_owned(),
        ),
        (
            "source_signature_diagnostic_row_id".to_owned(),
            format!("{}-{:03}", diagnostic_row_id_prefix, candidate.row_number),
        ),
        (
            "component_signature_rule".to_owned(),
            SIGNATURE_TARGET_COMPONENT_RULE.to_owned(),
        ),
        ("row_number".to_owned(), candidate.row_number.to_string()),
        (
            "left_profile_index".to_owned(),
            candidate.left_profile_index.to_string(),
        ),
        (
            "right_profile_index".to_owned(),
            candidate.right_profile_index.to_string(),
        ),
        (
            "total_recursive_nodes".to_owned(),
            candidate.total_recursive_nodes.to_string(),
        ),
        (
            "active_pieces".to_owned(),
            generated_depth_two_active_piece_summary(&candidate.active_pieces),
        ),
        (
            "left_component_value_digest".to_owned(),
            candidate.left_component_value_digest.clone(),
        ),
        (
            "right_component_value_digest".to_owned(),
            candidate.right_component_value_digest.clone(),
        ),
        (
            "current_result_value_digest".to_owned(),
            candidate.current_result_value_digest.clone(),
        ),
        (
            "left_component_signature".to_owned(),
            candidate.left_component_signature.clone(),
        ),
        (
            "right_component_signature".to_owned(),
            candidate.right_component_signature.clone(),
        ),
        (
            "result_signature_key".to_owned(),
            candidate.result_signature_key.clone(),
        ),
    ]));
    row
}

fn generated_depth_two_profile_search_report_with_seed(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
) -> GeneratedDepthTwoProfileSearchReport {
    let selection =
        generated_depth_two_leakage_clean_profile_selection(seed_rows, rows_per_family_target);
    generated_depth_two_profile_search_report_from_selection(rows_per_family_target, selection)
}

fn generated_depth_two_profile_search_report_with_seed_and_patterns(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
) -> GeneratedDepthTwoProfileSearchReport {
    let selection = generated_depth_two_leakage_clean_profile_selection_with_patterns(
        seed_rows,
        rows_per_family_target,
        white_patterns,
        black_patterns,
    );
    generated_depth_two_profile_search_report_from_selection(rows_per_family_target, selection)
}

fn generated_depth_two_profile_search_report_from_selection(
    rows_per_family_target: usize,
    selection: GeneratedDepthTwoProfileSelection,
) -> GeneratedDepthTwoProfileSearchReport {
    let topology_families = generated_depth_two_topology_families();
    let selected_counts_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.family_counts[index]))
        .collect::<BTreeMap<_, _>>();
    let candidate_pair_counts_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.candidate_pair_counts[index]))
        .collect::<BTreeMap<_, _>>();

    let candidates = selection
        .candidates
        .iter()
        .map(|candidate| GeneratedDepthTwoProfileCandidateReport {
            row_number: candidate.row_number,
            topology_family: candidate.topology_family.to_owned(),
            left_profile_index: candidate.left_profile_index,
            right_profile_index: candidate.right_profile_index,
            total_recursive_nodes: candidate.total_recursive_nodes,
            active_pieces: generated_depth_two_active_piece_summary(&candidate.active_pieces),
            left_component_value_digest: candidate.left_component_value_digest.clone(),
            right_component_value_digest: candidate.right_component_value_digest.clone(),
            result_value_digest: candidate.result_value_digest.clone(),
        })
        .collect::<Vec<_>>();

    GeneratedDepthTwoProfileSearchReport {
        rows_per_family_target,
        left_profile_count: selection.left_profile_count,
        right_profile_count: selection.right_profile_count,
        candidate_pair_counts_by_topology_family,
        selected_row_count: candidates.len(),
        selected_counts_by_topology_family,
        rejection_counts: selection.rejection_counts,
        candidates,
    }
}

fn generated_depth_two_signature_profile_search_report_with_seed_and_patterns(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    source: &str,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
) -> GeneratedDepthTwoSignatureProfileSearchReport {
    let selection = generated_depth_two_signature_profile_selection_with_patterns(
        seed_rows,
        rows_per_family_target,
        white_patterns,
        black_patterns,
        None,
    );
    let topology_families = generated_depth_two_topology_families();
    let selected_counts_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.family_counts[index]))
        .collect::<BTreeMap<_, _>>();
    let candidate_pair_counts_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.candidate_pair_counts[index]))
        .collect::<BTreeMap<_, _>>();
    let candidates = generated_depth_two_signature_candidate_reports(&selection);

    GeneratedDepthTwoSignatureProfileSearchReport {
        source: source.to_owned(),
        component_signature_rule:
            "depth2_value_digest_plus_material_balance_plus_local_move_counts_v0".to_owned(),
        rows_per_family_target,
        left_signature_profile_count: selection.left_signature_profile_count,
        right_signature_profile_count: selection.right_signature_profile_count,
        candidate_pair_counts_by_topology_family,
        selected_row_count: candidates.len(),
        selected_counts_by_topology_family,
        rejection_counts: selection.rejection_counts,
        candidates,
    }
}

fn generated_depth_two_value_unique_signature_profile_search_report_with_seed_and_patterns(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    source: &str,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
) -> GeneratedDepthTwoSignatureProfileSearchReport {
    let selection = generated_depth_two_value_unique_signature_profile_selection_with_patterns(
        seed_rows,
        rows_per_family_target,
        white_patterns,
        black_patterns,
        None,
    );
    let topology_families = generated_depth_two_topology_families();
    let selected_counts_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.family_counts[index]))
        .collect::<BTreeMap<_, _>>();
    let candidate_pair_counts_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.candidate_pair_counts[index]))
        .collect::<BTreeMap<_, _>>();
    let candidates = generated_depth_two_signature_candidate_reports(&selection);

    GeneratedDepthTwoSignatureProfileSearchReport {
        source: source.to_owned(),
        component_signature_rule:
            "depth2_value_digest_plus_material_balance_plus_local_move_counts_v0".to_owned(),
        rows_per_family_target,
        left_signature_profile_count: selection.left_signature_profile_count,
        right_signature_profile_count: selection.right_signature_profile_count,
        candidate_pair_counts_by_topology_family,
        selected_row_count: candidates.len(),
        selected_counts_by_topology_family,
        rejection_counts: selection.rejection_counts,
        candidates,
    }
}

fn generated_depth_two_value_unique_signature_upper_bound_report_with_patterns(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    source: &str,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
) -> GeneratedDepthTwoValueUniqueSignatureUpperBoundReport {
    generated_depth_two_value_unique_signature_upper_bound_report_with_patterns_and_selection(
        seed_rows,
        rows_per_family_target,
        source,
        white_patterns,
        black_patterns,
        true,
    )
}

fn generated_depth_two_value_unique_signature_capacity_report_with_patterns(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    source: &str,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
) -> GeneratedDepthTwoValueUniqueSignatureUpperBoundReport {
    generated_depth_two_value_unique_signature_upper_bound_report_with_patterns_and_selection(
        seed_rows,
        rows_per_family_target,
        source,
        white_patterns,
        black_patterns,
        false,
    )
}

fn generated_depth_two_value_unique_signature_upper_bound_report_with_patterns_and_selection(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    source: &str,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
    evaluate_current_selection: bool,
) -> GeneratedDepthTwoValueUniqueSignatureUpperBoundReport {
    let white_profiles = generated_depth_two_signature_profiles(white_patterns.clone());
    let black_profiles = generated_depth_two_signature_profiles(black_patterns.clone());
    let left_value_digests = white_profiles
        .iter()
        .map(|profile| profile.value_digest.clone())
        .collect::<BTreeSet<_>>();
    let right_value_digests = black_profiles
        .iter()
        .map(|profile| profile.value_digest.clone())
        .collect::<BTreeSet<_>>();
    let combined_value_digests = left_value_digests
        .union(&right_value_digests)
        .cloned()
        .collect::<BTreeSet<_>>();
    let shared_component_value_digest_count = left_value_digests
        .intersection(&right_value_digests)
        .count();
    let target_row_count = rows_per_family_target * generated_depth_two_topology_families().len();
    let component_value_capacity_upper_bound = [
        left_value_digests.len(),
        right_value_digests.len(),
        combined_value_digests.len() / 2,
        target_row_count,
    ]
    .into_iter()
    .min()
    .unwrap_or(0);

    let topology_families = generated_depth_two_topology_families();
    let mut candidate_pair_counts_by_topology_family = BTreeMap::new();
    let mut distinct_candidate_value_pair_counts_by_topology_family = BTreeMap::new();
    for (family_index, topology_family) in topology_families.iter().enumerate() {
        let candidate_pairs = generated_depth_two_signature_profile_candidate_pairs(
            family_index,
            &white_profiles,
            &black_profiles,
        );
        candidate_pair_counts_by_topology_family
            .insert((*topology_family).to_owned(), candidate_pairs.len());

        let mut value_pairs = BTreeSet::new();
        for (left_index, right_index, _total_recursive_nodes) in candidate_pairs {
            let left = &white_profiles[left_index];
            let right = &black_profiles[right_index];
            if left.value_digest == right.value_digest {
                continue;
            }
            value_pairs.insert(format!("{}|{}", left.value_digest, right.value_digest));
        }
        distinct_candidate_value_pair_counts_by_topology_family
            .insert((*topology_family).to_owned(), value_pairs.len());
    }

    let (
        current_selected_counts_by_topology_family,
        current_selected_row_count,
        current_rejection_counts,
    ) = if evaluate_current_selection {
        let selection = generated_depth_two_value_unique_signature_profile_selection_with_patterns(
            seed_rows,
            rows_per_family_target,
            white_patterns,
            black_patterns,
            None,
        );
        (
            topology_families
                .iter()
                .enumerate()
                .map(|(index, family)| ((*family).to_owned(), selection.family_counts[index]))
                .collect::<BTreeMap<_, _>>(),
            selection.candidates.len(),
            selection.rejection_counts,
        )
    } else {
        (
            topology_families
                .iter()
                .map(|family| ((*family).to_owned(), 0))
                .collect::<BTreeMap<_, _>>(),
            0,
            BTreeMap::new(),
        )
    };

    GeneratedDepthTwoValueUniqueSignatureUpperBoundReport {
        source: source.to_owned(),
        component_signature_rule:
            "depth2_value_digest_plus_material_balance_plus_local_move_counts_v0".to_owned(),
        rows_per_family_target,
        current_selection_evaluated: evaluate_current_selection,
        left_signature_profile_count: white_profiles.len(),
        right_signature_profile_count: black_profiles.len(),
        left_unique_component_value_digest_count: left_value_digests.len(),
        right_unique_component_value_digest_count: right_value_digests.len(),
        shared_component_value_digest_count,
        combined_unique_component_value_digest_count: combined_value_digests.len(),
        component_value_capacity_upper_bound,
        target_row_count,
        candidate_pair_counts_by_topology_family,
        distinct_candidate_value_pair_counts_by_topology_family,
        current_selected_row_count,
        current_selected_counts_by_topology_family,
        current_rejection_counts,
    }
}

fn generated_depth_two_signature_bounded_support_report_with_seed_and_patterns(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    candidate_pair_limit_per_family: usize,
    source: &str,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
) -> GeneratedDepthTwoSignatureBoundedSupportReport {
    let selection = generated_depth_two_signature_profile_selection_with_patterns(
        seed_rows,
        rows_per_family_target,
        white_patterns,
        black_patterns,
        Some(candidate_pair_limit_per_family),
    );
    let topology_families = generated_depth_two_topology_families();
    let selected_counts_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.family_counts[index]))
        .collect::<BTreeMap<_, _>>();
    let candidate_pair_counts_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.candidate_pair_counts[index]))
        .collect::<BTreeMap<_, _>>();
    let candidate_offsets_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.candidate_offsets[index]))
        .collect::<BTreeMap<_, _>>();
    let reached_target_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| {
            (
                (*family).to_owned(),
                selection.family_counts[index] >= rows_per_family_target,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let candidate_pair_limit_hit_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| {
            (
                (*family).to_owned(),
                selection.candidate_offsets[index] >= candidate_pair_limit_per_family
                    && candidate_pair_limit_per_family < selection.candidate_pair_counts[index],
            )
        })
        .collect::<BTreeMap<_, _>>();
    let candidates = generated_depth_two_signature_candidate_reports(&selection);

    GeneratedDepthTwoSignatureBoundedSupportReport {
        source: source.to_owned(),
        component_signature_rule:
            "depth2_value_digest_plus_material_balance_plus_local_move_counts_v0".to_owned(),
        rows_per_family_target,
        candidate_pair_limit_per_family,
        left_signature_profile_count: selection.left_signature_profile_count,
        right_signature_profile_count: selection.right_signature_profile_count,
        candidate_pair_counts_by_topology_family,
        candidate_offsets_by_topology_family,
        selected_row_count: candidates.len(),
        selected_counts_by_topology_family,
        reached_target_by_topology_family,
        candidate_pair_limit_hit_by_topology_family,
        rejection_counts: selection.rejection_counts,
        candidates,
    }
}

fn generated_depth_two_value_unique_signature_bounded_support_report_with_seed_and_patterns(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    candidate_pair_limit_per_family: usize,
    source: &str,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
    candidate_pair_order: GeneratedDepthTwoCandidatePairOrder,
) -> GeneratedDepthTwoSignatureBoundedSupportReport {
    let selection =
        generated_depth_two_signature_profile_selection_with_patterns_internal_and_order(
            seed_rows,
            rows_per_family_target,
            white_patterns,
            black_patterns,
            Some(candidate_pair_limit_per_family),
            true,
            candidate_pair_order,
        );
    let topology_families = generated_depth_two_topology_families();
    let selected_counts_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.family_counts[index]))
        .collect::<BTreeMap<_, _>>();
    let candidate_pair_counts_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.candidate_pair_counts[index]))
        .collect::<BTreeMap<_, _>>();
    let candidate_offsets_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.candidate_offsets[index]))
        .collect::<BTreeMap<_, _>>();
    let reached_target_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| {
            (
                (*family).to_owned(),
                selection.family_counts[index] >= rows_per_family_target,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let candidate_pair_limit_hit_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| {
            (
                (*family).to_owned(),
                selection.candidate_offsets[index] >= candidate_pair_limit_per_family
                    && candidate_pair_limit_per_family < selection.candidate_pair_counts[index],
            )
        })
        .collect::<BTreeMap<_, _>>();
    let candidates = generated_depth_two_signature_candidate_reports(&selection);

    GeneratedDepthTwoSignatureBoundedSupportReport {
        source: source.to_owned(),
        component_signature_rule:
            "depth2_value_digest_plus_material_balance_plus_local_move_counts_v0".to_owned(),
        rows_per_family_target,
        candidate_pair_limit_per_family,
        left_signature_profile_count: selection.left_signature_profile_count,
        right_signature_profile_count: selection.right_signature_profile_count,
        candidate_pair_counts_by_topology_family,
        candidate_offsets_by_topology_family,
        selected_row_count: candidates.len(),
        selected_counts_by_topology_family,
        reached_target_by_topology_family,
        candidate_pair_limit_hit_by_topology_family,
        rejection_counts: selection.rejection_counts,
        candidates,
    }
}

fn generated_depth_two_value_unique_signature_dynamic_pairing_preflight_report_with_patterns(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    source: &str,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
) -> GeneratedDepthTwoSignatureProfileSearchReport {
    let selection = generated_depth_two_value_unique_signature_dynamic_pairing_preflight_selection(
        seed_rows,
        rows_per_family_target,
        white_patterns,
        black_patterns,
    );
    let topology_families = generated_depth_two_topology_families();
    let selected_counts_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.family_counts[index]))
        .collect::<BTreeMap<_, _>>();
    let candidate_pair_counts_by_topology_family = topology_families
        .iter()
        .enumerate()
        .map(|(index, family)| ((*family).to_owned(), selection.candidate_pair_counts[index]))
        .collect::<BTreeMap<_, _>>();
    let candidates = generated_depth_two_signature_candidate_reports(&selection);

    GeneratedDepthTwoSignatureProfileSearchReport {
        source: source.to_owned(),
        component_signature_rule:
            "depth2_value_digest_plus_material_balance_plus_local_move_counts_v0".to_owned(),
        rows_per_family_target,
        left_signature_profile_count: selection.left_signature_profile_count,
        right_signature_profile_count: selection.right_signature_profile_count,
        candidate_pair_counts_by_topology_family,
        selected_row_count: candidates.len(),
        selected_counts_by_topology_family,
        rejection_counts: selection.rejection_counts,
        candidates,
    }
}

fn generated_depth_two_signature_candidate_reports(
    selection: &GeneratedDepthTwoSignatureProfileSelection,
) -> Vec<GeneratedDepthTwoSignatureProfileCandidateReport> {
    selection
        .candidates
        .iter()
        .map(
            |candidate| GeneratedDepthTwoSignatureProfileCandidateReport {
                row_number: candidate.row_number,
                topology_family: candidate.topology_family.to_owned(),
                left_profile_index: candidate.left_profile_index,
                right_profile_index: candidate.right_profile_index,
                total_recursive_nodes: candidate.total_recursive_nodes,
                active_pieces: generated_depth_two_active_piece_summary(&candidate.active_pieces),
                left_component_value_digest: candidate.left_component_value_digest.clone(),
                right_component_value_digest: candidate.right_component_value_digest.clone(),
                left_component_signature: candidate.left_component_signature.clone(),
                right_component_signature: candidate.right_component_signature.clone(),
                result_signature_key: candidate.result_signature_key.clone(),
                current_result_value_digest: candidate.current_result_value_digest.clone(),
            },
        )
        .collect()
}

fn generated_depth_two_leakage_clean_profile_selection(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
) -> GeneratedDepthTwoProfileSelection {
    generated_depth_two_leakage_clean_profile_selection_with_patterns(
        seed_rows,
        rows_per_family_target,
        generated_white_component_patterns(),
        generated_black_component_patterns(),
    )
}

fn generated_depth_two_leakage_clean_profile_selection_with_patterns(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
) -> GeneratedDepthTwoProfileSelection {
    let white_profiles = generated_depth_two_wall_safe_component_profiles(white_patterns);
    let black_profiles = generated_depth_two_wall_safe_component_profiles(black_patterns);
    let topology_families = generated_depth_two_topology_families();

    let mut seen_positions = BTreeSet::new();
    let mut seen_decomposition_digests = BTreeSet::new();
    let mut seen_composition_digests = BTreeSet::new();
    let mut seen_component_identities = BTreeSet::new();
    let mut seen_component_value_digests = BTreeSet::new();
    let mut seen_component_value_identities = BTreeSet::new();
    let mut seen_result_digests = BTreeSet::new();
    for row in seed_rows {
        seen_positions.insert(row.position.text.clone());
        if let Some(summary) = composition_identity_summary_for_row(row) {
            seen_decomposition_digests.insert(summary.decomposition_digest);
            seen_composition_digests.insert(summary.composition_digest);
            seen_result_digests.insert(summary.result_value_digest);
            seen_component_identities.extend(summary.component_identities);
            seen_component_value_digests.extend(summary.component_value_digests);
            seen_component_value_identities.extend(summary.component_value_identities);
        }
    }

    let mut candidate_offsets = vec![0usize; topology_families.len()];
    let candidate_pairs = topology_families
        .iter()
        .enumerate()
        .map(|(family_index, _family)| {
            generated_depth_two_profile_candidate_pairs(
                family_index,
                &white_profiles,
                &black_profiles,
            )
        })
        .collect::<Vec<_>>();
    let candidate_pair_counts = candidate_pairs
        .iter()
        .map(std::vec::Vec::len)
        .collect::<Vec<_>>();
    let mut family_counts = vec![0usize; topology_families.len()];
    let mut rejection_counts = BTreeMap::new();
    let mut candidates = Vec::new();
    let mut next_row_number = NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS.len() + 1;

    while family_counts
        .iter()
        .any(|count| *count < rows_per_family_target)
    {
        let mut made_progress = false;
        for (family_index, topology_family) in topology_families.iter().enumerate() {
            if family_counts[family_index] == rows_per_family_target {
                continue;
            }

            while candidate_offsets[family_index] < candidate_pairs[family_index].len() {
                let (left_index, right_index, total_recursive_nodes) =
                    candidate_pairs[family_index][candidate_offsets[family_index]];
                candidate_offsets[family_index] += 1;
                let left = &white_profiles[left_index];
                let right = &black_profiles[right_index];
                if left.value_digest == right.value_digest {
                    increment_count(&mut rejection_counts, "same_component_value_digest");
                    continue;
                }
                if seen_component_value_digests.contains(&left.value_digest)
                    || seen_component_value_digests.contains(&right.value_digest)
                {
                    increment_count(
                        &mut rejection_counts,
                        "component_value_digest_reuse_before_materialization",
                    );
                    continue;
                }
                let result_value = CGTValue::sum_all(&[left.value.clone(), right.value.clone()]);
                let result_value_digest = result_value.exact_value_payload().digest;
                if seen_result_digests.contains(&result_value_digest) {
                    increment_count(
                        &mut rejection_counts,
                        "result_value_digest_reuse_before_materialization",
                    );
                    continue;
                }

                let mut active_pieces = left.active_pieces.clone();
                active_pieces.extend(right.active_pieces.iter().copied());
                active_pieces.sort_by_key(|(square, piece)| (usize::from(*square), *piece));
                let board_position_key = generated_depth_two_board_position_key(&active_pieces);
                if seen_positions.contains(&board_position_key) {
                    increment_count(
                        &mut rejection_counts,
                        "position_reuse_before_materialization",
                    );
                    continue;
                }

                let row_id = format!(
                    "{}-{:03}",
                    NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG.row_id_prefix, next_row_number
                );
                let spec = NonFixtureComposedBoardSpec {
                    row_id: &row_id,
                    active_pieces: &active_pieces,
                    fullmove_number: GENERATED_DEPTH_TWO_START_FULLMOVE
                        + u32::try_from(next_row_number)
                            .expect("generated row number fits fullmove u32"),
                    value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
                    topology_family,
                    spec_source: PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE,
                };
                let Ok(row) = try_non_fixture_composed_board_exact_row(&spec) else {
                    increment_count(&mut rejection_counts, "materialization_failure");
                    continue;
                };
                if !generated_depth_two_row_within_node_budget(&row) {
                    increment_count(&mut rejection_counts, "row_recursive_node_budget");
                    continue;
                }
                let Some(summary) = composition_identity_summary_for_row(&row) else {
                    increment_count(&mut rejection_counts, "missing_identity_summary");
                    continue;
                };
                if has_duplicates(&summary.component_identities)
                    || has_duplicates(&summary.component_value_digests)
                    || has_duplicates(&summary.component_value_identities)
                {
                    increment_count(&mut rejection_counts, "intra_row_identity_duplicate");
                    continue;
                }
                if seen_positions.contains(&row.position.text) {
                    increment_count(&mut rejection_counts, "row_position_reuse");
                    continue;
                }
                if seen_decomposition_digests.contains(&summary.decomposition_digest) {
                    increment_count(&mut rejection_counts, "decomposition_digest_reuse");
                    continue;
                }
                if seen_composition_digests.contains(&summary.composition_digest) {
                    increment_count(&mut rejection_counts, "composition_digest_reuse");
                    continue;
                }
                if seen_result_digests.contains(&summary.result_value_digest) {
                    increment_count(
                        &mut rejection_counts,
                        "result_value_digest_reuse_after_materialization",
                    );
                    continue;
                }
                if summary
                    .component_identities
                    .iter()
                    .any(|identity| seen_component_identities.contains(identity))
                {
                    increment_count(&mut rejection_counts, "component_identity_reuse");
                    continue;
                }
                if summary
                    .component_value_digests
                    .iter()
                    .any(|digest| seen_component_value_digests.contains(digest))
                {
                    increment_count(
                        &mut rejection_counts,
                        "component_value_digest_reuse_after_materialization",
                    );
                    continue;
                }
                if summary
                    .component_value_identities
                    .iter()
                    .any(|identity| seen_component_value_identities.contains(identity))
                {
                    increment_count(&mut rejection_counts, "component_value_identity_reuse");
                    continue;
                }

                seen_positions.insert(board_position_key);
                seen_positions.insert(row.position.text.clone());
                seen_decomposition_digests.insert(summary.decomposition_digest);
                seen_composition_digests.insert(summary.composition_digest);
                seen_result_digests.insert(summary.result_value_digest);
                seen_component_identities.extend(summary.component_identities);
                seen_component_value_digests.extend(summary.component_value_digests);
                seen_component_value_identities.extend(summary.component_value_identities);
                candidates.push(GeneratedDepthTwoSelectedProfileCandidate {
                    row_number: next_row_number,
                    topology_family,
                    left_profile_index: left_index,
                    right_profile_index: right_index,
                    total_recursive_nodes,
                    active_pieces,
                    left_component_value_digest: left.value_digest.clone(),
                    right_component_value_digest: right.value_digest.clone(),
                    result_value_digest,
                });
                next_row_number += 1;
                family_counts[family_index] += 1;
                made_progress = true;
                break;
            }
        }

        if !made_progress {
            break;
        }
    }

    GeneratedDepthTwoProfileSelection {
        left_profile_count: white_profiles.len(),
        right_profile_count: black_profiles.len(),
        candidate_pair_counts,
        family_counts,
        rejection_counts,
        candidates,
    }
}

fn generated_depth_two_signature_profile_selection_with_patterns(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
    candidate_pair_limit_per_family: Option<usize>,
) -> GeneratedDepthTwoSignatureProfileSelection {
    generated_depth_two_signature_profile_selection_with_patterns_internal(
        seed_rows,
        rows_per_family_target,
        white_patterns,
        black_patterns,
        candidate_pair_limit_per_family,
        false,
    )
}

fn generated_depth_two_value_unique_signature_profile_selection_with_patterns(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
    candidate_pair_limit_per_family: Option<usize>,
) -> GeneratedDepthTwoSignatureProfileSelection {
    generated_depth_two_signature_profile_selection_with_patterns_internal(
        seed_rows,
        rows_per_family_target,
        white_patterns,
        black_patterns,
        candidate_pair_limit_per_family,
        true,
    )
}

fn generated_depth_two_signature_profile_selection_with_patterns_internal(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
    candidate_pair_limit_per_family: Option<usize>,
    require_value_digest_uniqueness: bool,
) -> GeneratedDepthTwoSignatureProfileSelection {
    generated_depth_two_signature_profile_selection_with_patterns_internal_and_order(
        seed_rows,
        rows_per_family_target,
        white_patterns,
        black_patterns,
        candidate_pair_limit_per_family,
        require_value_digest_uniqueness,
        GeneratedDepthTwoCandidatePairOrder::ProfileIndexSpread,
    )
}

fn generated_depth_two_signature_profile_selection_with_patterns_internal_and_order(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
    candidate_pair_limit_per_family: Option<usize>,
    require_value_digest_uniqueness: bool,
    candidate_pair_order: GeneratedDepthTwoCandidatePairOrder,
) -> GeneratedDepthTwoSignatureProfileSelection {
    let white_profiles = generated_depth_two_signature_profiles(white_patterns);
    let black_profiles = generated_depth_two_signature_profiles(black_patterns);
    let topology_families = generated_depth_two_topology_families();

    let mut seen_positions = BTreeSet::new();
    let mut seen_decomposition_digests = BTreeSet::new();
    let mut seen_composition_digests = BTreeSet::new();
    let mut seen_component_identities = BTreeSet::new();
    let mut seen_component_value_digests = BTreeSet::new();
    let mut seen_component_value_identities = BTreeSet::new();
    let mut seen_result_digests = BTreeSet::new();
    for row in seed_rows {
        seen_positions.insert(row.position.text.clone());
        if let Some(summary) = composition_identity_summary_for_row(row) {
            seen_decomposition_digests.insert(summary.decomposition_digest);
            seen_composition_digests.insert(summary.composition_digest);
            seen_component_identities.extend(summary.component_identities);
        }
    }

    let mut seen_component_signatures = BTreeSet::new();
    let mut seen_result_signature_keys = BTreeSet::new();
    let mut candidate_offsets = vec![0usize; topology_families.len()];
    let candidate_pairs = topology_families
        .iter()
        .enumerate()
        .map(|(family_index, _family)| {
            generated_depth_two_signature_profile_candidate_pairs_with_order(
                family_index,
                &white_profiles,
                &black_profiles,
                candidate_pair_order,
            )
        })
        .collect::<Vec<_>>();
    let candidate_pair_counts = candidate_pairs
        .iter()
        .map(std::vec::Vec::len)
        .collect::<Vec<_>>();
    let mut family_counts = vec![0usize; topology_families.len()];
    let mut rejection_counts = BTreeMap::new();
    let mut candidates = Vec::new();
    let mut next_row_number = NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS.len() + 1;

    while family_counts
        .iter()
        .any(|count| *count < rows_per_family_target)
    {
        let mut made_progress = false;
        for (family_index, topology_family) in topology_families.iter().enumerate() {
            if family_counts[family_index] == rows_per_family_target {
                continue;
            }

            let candidate_pair_limit = candidate_pair_limit_per_family
                .unwrap_or(candidate_pairs[family_index].len())
                .min(candidate_pairs[family_index].len());
            while candidate_offsets[family_index] < candidate_pair_limit {
                let (left_index, right_index, total_recursive_nodes) =
                    candidate_pairs[family_index][candidate_offsets[family_index]];
                candidate_offsets[family_index] += 1;
                let left = &white_profiles[left_index];
                let right = &black_profiles[right_index];
                if left.component_signature == right.component_signature {
                    increment_count(&mut rejection_counts, "same_component_signature");
                    continue;
                }
                if require_value_digest_uniqueness && left.value_digest == right.value_digest {
                    increment_count(&mut rejection_counts, "same_component_value_digest");
                    continue;
                }
                if seen_component_signatures.contains(&left.component_signature)
                    || seen_component_signatures.contains(&right.component_signature)
                {
                    increment_count(
                        &mut rejection_counts,
                        "component_signature_reuse_before_materialization",
                    );
                    continue;
                }
                if require_value_digest_uniqueness
                    && (seen_component_value_digests.contains(&left.value_digest)
                        || seen_component_value_digests.contains(&right.value_digest))
                {
                    increment_count(
                        &mut rejection_counts,
                        "component_value_digest_reuse_before_materialization",
                    );
                    continue;
                }
                let result_signature_key = generated_depth_two_result_signature_key(
                    topology_family,
                    &left.component_signature,
                    &right.component_signature,
                );
                if seen_result_signature_keys.contains(&result_signature_key) {
                    increment_count(
                        &mut rejection_counts,
                        "result_signature_reuse_before_materialization",
                    );
                    continue;
                }
                let current_result_value =
                    CGTValue::sum_all(&[left.value.clone(), right.value.clone()]);
                let current_result_value_digest = current_result_value.exact_value_payload().digest;
                if require_value_digest_uniqueness
                    && seen_result_digests.contains(&current_result_value_digest)
                {
                    increment_count(
                        &mut rejection_counts,
                        "result_value_digest_reuse_before_materialization",
                    );
                    continue;
                }

                let mut active_pieces = left.active_pieces.clone();
                active_pieces.extend(right.active_pieces.iter().copied());
                active_pieces.sort_by_key(|(square, piece)| (usize::from(*square), *piece));
                let board_position_key = generated_depth_two_board_position_key(&active_pieces);
                if seen_positions.contains(&board_position_key) {
                    increment_count(
                        &mut rejection_counts,
                        "position_reuse_before_materialization",
                    );
                    continue;
                }

                let row_id = format!(
                    "{}-{:03}",
                    NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG.row_id_prefix, next_row_number
                );
                let spec = NonFixtureComposedBoardSpec {
                    row_id: &row_id,
                    active_pieces: &active_pieces,
                    fullmove_number: GENERATED_DEPTH_TWO_START_FULLMOVE
                        + u32::try_from(next_row_number)
                            .expect("generated row number fits fullmove u32"),
                    value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
                    topology_family,
                    spec_source: PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE,
                };
                let Ok(row) = try_non_fixture_composed_board_exact_row(&spec) else {
                    increment_count(&mut rejection_counts, "materialization_failure");
                    continue;
                };
                if !generated_depth_two_row_within_node_budget(&row) {
                    increment_count(&mut rejection_counts, "row_recursive_node_budget");
                    continue;
                }
                let Some(summary) = composition_identity_summary_for_row(&row) else {
                    increment_count(&mut rejection_counts, "missing_identity_summary");
                    continue;
                };
                if has_duplicates(&summary.component_identities) {
                    increment_count(
                        &mut rejection_counts,
                        "intra_row_component_identity_duplicate",
                    );
                    continue;
                }
                if require_value_digest_uniqueness
                    && (has_duplicates(&summary.component_value_digests)
                        || has_duplicates(&summary.component_value_identities))
                {
                    increment_count(&mut rejection_counts, "intra_row_component_value_duplicate");
                    continue;
                }
                if seen_positions.contains(&row.position.text) {
                    increment_count(&mut rejection_counts, "row_position_reuse");
                    continue;
                }
                if seen_decomposition_digests.contains(&summary.decomposition_digest) {
                    increment_count(&mut rejection_counts, "decomposition_digest_reuse");
                    continue;
                }
                if seen_composition_digests.contains(&summary.composition_digest) {
                    increment_count(&mut rejection_counts, "composition_digest_reuse");
                    continue;
                }
                if summary
                    .component_identities
                    .iter()
                    .any(|identity| seen_component_identities.contains(identity))
                {
                    increment_count(&mut rejection_counts, "component_identity_reuse");
                    continue;
                }
                if require_value_digest_uniqueness
                    && seen_result_digests.contains(&summary.result_value_digest)
                {
                    increment_count(
                        &mut rejection_counts,
                        "result_value_digest_reuse_after_materialization",
                    );
                    continue;
                }
                if require_value_digest_uniqueness
                    && summary
                        .component_value_digests
                        .iter()
                        .any(|digest| seen_component_value_digests.contains(digest))
                {
                    increment_count(
                        &mut rejection_counts,
                        "component_value_digest_reuse_after_materialization",
                    );
                    continue;
                }
                if require_value_digest_uniqueness
                    && summary
                        .component_value_identities
                        .iter()
                        .any(|identity| seen_component_value_identities.contains(identity))
                {
                    increment_count(&mut rejection_counts, "component_value_identity_reuse");
                    continue;
                }

                seen_positions.insert(board_position_key);
                seen_positions.insert(row.position.text.clone());
                seen_decomposition_digests.insert(summary.decomposition_digest);
                seen_composition_digests.insert(summary.composition_digest);
                seen_component_identities.extend(summary.component_identities);
                if require_value_digest_uniqueness {
                    seen_result_digests.insert(summary.result_value_digest);
                    seen_component_value_digests.extend(summary.component_value_digests);
                    seen_component_value_identities.extend(summary.component_value_identities);
                }
                seen_component_signatures.insert(left.component_signature.clone());
                seen_component_signatures.insert(right.component_signature.clone());
                seen_result_signature_keys.insert(result_signature_key.clone());
                candidates.push(GeneratedDepthTwoSelectedSignatureCandidate {
                    row_number: next_row_number,
                    topology_family,
                    left_profile_index: left_index,
                    right_profile_index: right_index,
                    total_recursive_nodes,
                    active_pieces,
                    left_component_value_digest: left.value_digest.clone(),
                    right_component_value_digest: right.value_digest.clone(),
                    left_component_signature: left.component_signature.clone(),
                    right_component_signature: right.component_signature.clone(),
                    result_signature_key,
                    current_result_value_digest,
                });
                next_row_number += 1;
                family_counts[family_index] += 1;
                made_progress = true;
                break;
            }
        }

        if !made_progress {
            break;
        }
    }

    GeneratedDepthTwoSignatureProfileSelection {
        left_signature_profile_count: white_profiles.len(),
        right_signature_profile_count: black_profiles.len(),
        candidate_pair_counts,
        candidate_offsets,
        family_counts,
        rejection_counts,
        candidates,
    }
}

fn generated_depth_two_value_unique_signature_dynamic_pairing_preflight_selection(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
) -> GeneratedDepthTwoSignatureProfileSelection {
    let white_profiles = generated_depth_two_signature_profiles(white_patterns);
    let black_profiles = generated_depth_two_signature_profiles(black_patterns);
    let topology_families = generated_depth_two_topology_families();

    let mut seen_positions = BTreeSet::new();
    let mut seen_component_value_digests = BTreeSet::new();
    let mut seen_result_digests = BTreeSet::new();
    for row in seed_rows {
        seen_positions.insert(row.position.text.clone());
        if let Some(summary) = composition_identity_summary_for_row(row) {
            seen_result_digests.insert(summary.result_value_digest);
            seen_component_value_digests.extend(summary.component_value_digests);
        }
    }

    let mut seen_component_signatures = BTreeSet::new();
    let mut seen_result_signature_keys = BTreeSet::new();
    let candidate_pairs = topology_families
        .iter()
        .enumerate()
        .map(|(family_index, _family)| {
            generated_depth_two_signature_profile_candidate_pairs(
                family_index,
                &white_profiles,
                &black_profiles,
            )
        })
        .collect::<Vec<_>>();
    let candidate_pair_counts = candidate_pairs
        .iter()
        .map(std::vec::Vec::len)
        .collect::<Vec<_>>();
    let mut candidate_visits = vec![0usize; topology_families.len()];
    let mut family_counts = vec![0usize; topology_families.len()];
    let mut rejection_counts = BTreeMap::new();
    let mut candidates = Vec::new();
    let mut next_row_number = NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS.len() + 1;

    while family_counts
        .iter()
        .any(|count| *count < rows_per_family_target)
    {
        let mut made_progress = false;
        for (family_index, topology_family) in topology_families.iter().enumerate() {
            if family_counts[family_index] == rows_per_family_target {
                continue;
            }

            for (left_index, right_index, total_recursive_nodes) in &candidate_pairs[family_index] {
                candidate_visits[family_index] += 1;
                let left = &white_profiles[*left_index];
                let right = &black_profiles[*right_index];
                if left.component_signature == right.component_signature {
                    increment_count(&mut rejection_counts, "same_component_signature");
                    continue;
                }
                if left.value_digest == right.value_digest {
                    increment_count(&mut rejection_counts, "same_component_value_digest");
                    continue;
                }
                if seen_component_signatures.contains(&left.component_signature)
                    || seen_component_signatures.contains(&right.component_signature)
                {
                    increment_count(
                        &mut rejection_counts,
                        "component_signature_reuse_before_materialization",
                    );
                    continue;
                }
                if seen_component_value_digests.contains(&left.value_digest)
                    || seen_component_value_digests.contains(&right.value_digest)
                {
                    increment_count(
                        &mut rejection_counts,
                        "component_value_digest_reuse_before_materialization",
                    );
                    continue;
                }
                let result_signature_key = generated_depth_two_result_signature_key(
                    topology_family,
                    &left.component_signature,
                    &right.component_signature,
                );
                if seen_result_signature_keys.contains(&result_signature_key) {
                    increment_count(
                        &mut rejection_counts,
                        "result_signature_reuse_before_materialization",
                    );
                    continue;
                }
                let current_result_value =
                    CGTValue::sum_all(&[left.value.clone(), right.value.clone()]);
                let current_result_value_digest = current_result_value.exact_value_payload().digest;
                if seen_result_digests.contains(&current_result_value_digest) {
                    increment_count(
                        &mut rejection_counts,
                        "result_value_digest_reuse_before_materialization",
                    );
                    continue;
                }

                let mut active_pieces = left.active_pieces.clone();
                active_pieces.extend(right.active_pieces.iter().copied());
                active_pieces.sort_by_key(|(square, piece)| (usize::from(*square), *piece));
                let board_position_key = generated_depth_two_board_position_key(&active_pieces);
                if seen_positions.contains(&board_position_key) {
                    increment_count(
                        &mut rejection_counts,
                        "position_reuse_before_materialization",
                    );
                    continue;
                }

                seen_positions.insert(board_position_key);
                seen_component_value_digests.insert(left.value_digest.clone());
                seen_component_value_digests.insert(right.value_digest.clone());
                seen_result_digests.insert(current_result_value_digest.clone());
                seen_component_signatures.insert(left.component_signature.clone());
                seen_component_signatures.insert(right.component_signature.clone());
                seen_result_signature_keys.insert(result_signature_key.clone());
                candidates.push(GeneratedDepthTwoSelectedSignatureCandidate {
                    row_number: next_row_number,
                    topology_family,
                    left_profile_index: *left_index,
                    right_profile_index: *right_index,
                    total_recursive_nodes: *total_recursive_nodes,
                    active_pieces,
                    left_component_value_digest: left.value_digest.clone(),
                    right_component_value_digest: right.value_digest.clone(),
                    left_component_signature: left.component_signature.clone(),
                    right_component_signature: right.component_signature.clone(),
                    result_signature_key,
                    current_result_value_digest,
                });
                next_row_number += 1;
                family_counts[family_index] += 1;
                made_progress = true;
                break;
            }
        }

        if !made_progress {
            break;
        }
    }

    GeneratedDepthTwoSignatureProfileSelection {
        left_signature_profile_count: white_profiles.len(),
        right_signature_profile_count: black_profiles.len(),
        candidate_pair_counts,
        candidate_offsets: candidate_visits,
        family_counts,
        rejection_counts,
        candidates,
    }
}

fn generated_depth_two_profile_selection(
    seed_rows: &[DatasetLabelRow],
    rows_per_family_target: usize,
    reuse_generated_component_values: bool,
) -> GeneratedDepthTwoProfileSelection {
    let white_profiles =
        generated_depth_two_wall_safe_component_profiles(generated_white_component_patterns());
    let black_profiles =
        generated_depth_two_wall_safe_component_profiles(generated_black_component_patterns());
    let topology_families = generated_depth_two_topology_families();

    let mut seen_positions = BTreeSet::new();
    let mut seen_component_digests = BTreeSet::new();
    let mut seen_result_digests = BTreeSet::new();
    let mut next_row_number = NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS.len() + 1;
    for row in seed_rows {
        seen_positions.insert(row.position.text.clone());
        if let Some((_decomposition_digest, result_digest, component_digests)) =
            composition_digest_summary_for_row(row)
        {
            seen_result_digests.insert(result_digest);
            for digest in component_digests {
                seen_component_digests.insert(digest);
            }
        }
    }

    let mut candidate_offsets = vec![0usize; topology_families.len()];
    let candidate_pairs = topology_families
        .iter()
        .enumerate()
        .map(|(family_index, _family)| {
            generated_depth_two_profile_candidate_pairs(
                family_index,
                &white_profiles,
                &black_profiles,
            )
        })
        .collect::<Vec<_>>();
    let candidate_pair_counts = candidate_pairs
        .iter()
        .map(std::vec::Vec::len)
        .collect::<Vec<_>>();
    let mut family_counts = vec![0usize; topology_families.len()];
    let mut candidates = Vec::new();

    while family_counts
        .iter()
        .any(|count| *count < rows_per_family_target)
    {
        let mut made_progress = false;
        for (family_index, topology_family) in topology_families.iter().enumerate() {
            if family_counts[family_index] == rows_per_family_target {
                continue;
            }

            while candidate_offsets[family_index] < candidate_pairs[family_index].len() {
                let (left_index, right_index, total_recursive_nodes) =
                    candidate_pairs[family_index][candidate_offsets[family_index]];
                candidate_offsets[family_index] += 1;
                let left = &white_profiles[left_index];
                let right = &black_profiles[right_index];
                if left.value_digest == right.value_digest
                    || seen_component_digests.contains(&left.value_digest)
                    || seen_component_digests.contains(&right.value_digest)
                {
                    continue;
                }
                let result_value = CGTValue::sum_all(&[left.value.clone(), right.value.clone()]);
                let result_value_digest = result_value.exact_value_payload().digest;
                if seen_result_digests.contains(&result_value_digest) {
                    continue;
                }

                let mut active_pieces = left.active_pieces.clone();
                active_pieces.extend(right.active_pieces.iter().copied());
                active_pieces.sort_by_key(|(square, piece)| (usize::from(*square), *piece));
                let board_position_key = generated_depth_two_board_position_key(&active_pieces);
                if seen_positions.contains(&board_position_key) {
                    continue;
                }

                seen_positions.insert(board_position_key);
                seen_result_digests.insert(result_value_digest.clone());
                if !reuse_generated_component_values {
                    seen_component_digests.insert(left.value_digest.clone());
                    seen_component_digests.insert(right.value_digest.clone());
                }
                candidates.push(GeneratedDepthTwoSelectedProfileCandidate {
                    row_number: next_row_number,
                    topology_family,
                    left_profile_index: left_index,
                    right_profile_index: right_index,
                    total_recursive_nodes,
                    active_pieces,
                    left_component_value_digest: left.value_digest.clone(),
                    right_component_value_digest: right.value_digest.clone(),
                    result_value_digest,
                });
                next_row_number += 1;
                family_counts[family_index] += 1;
                made_progress = true;
                break;
            }
        }

        if !made_progress {
            break;
        }
    }

    GeneratedDepthTwoProfileSelection {
        left_profile_count: white_profiles.len(),
        right_profile_count: black_profiles.len(),
        candidate_pair_counts,
        family_counts,
        rejection_counts: BTreeMap::new(),
        candidates,
    }
}

fn generated_depth_two_profile_candidate_pairs(
    family_index: usize,
    white_profiles: &[GeneratedDepthTwoComponentProfile],
    black_profiles: &[GeneratedDepthTwoComponentProfile],
) -> Vec<(usize, usize, usize)> {
    let mut pairs = Vec::new();
    for (left_index, left) in white_profiles.iter().enumerate() {
        for (right_index, right) in black_profiles.iter().enumerate() {
            let total_recursive_nodes = left.recursive_nodes + right.recursive_nodes;
            if total_recursive_nodes > GENERATED_DEPTH_TWO_MAX_RECURSIVE_NODES
                || left.value_digest == right.value_digest
            {
                continue;
            }
            pairs.push((left_index, right_index, total_recursive_nodes));
        }
    }

    let product = white_profiles
        .len()
        .saturating_mul(black_profiles.len())
        .max(1);
    pairs.sort_by_key(|(left_index, right_index, total_recursive_nodes)| {
        (
            (left_index * 37 + right_index * 53 + family_index * 97) % product,
            *total_recursive_nodes,
            *left_index,
            *right_index,
        )
    });
    pairs
}

fn generated_depth_two_board_position_key(active_pieces: &[(Square, char)]) -> String {
    let spec = NonFixtureComposedBoardSpec {
        row_id: "generated-depth-two-profile-search",
        active_pieces,
        fullmove_number: GENERATED_DEPTH_TWO_START_FULLMOVE,
        value_rule: NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        topology_family: DEPTH_TWO_LOCAL_MOVE_TOPOLOGY_FAMILY,
        spec_source: PROFILED_DEPTH_TWO_GENERATED_SPEC_SOURCE,
    };
    non_fixture_composed_board_fen(&spec)
}

fn generated_depth_two_active_piece_summary(active_pieces: &[(Square, char)]) -> String {
    active_pieces
        .iter()
        .map(|(square, piece)| format!("{square}{piece}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn generated_depth_two_component_profiles(
    patterns: Vec<Vec<(Square, char)>>,
) -> Vec<GeneratedDepthTwoComponentProfile> {
    let mut profiles = Vec::new();
    let mut seen_value_digests = BTreeSet::new();
    for active_pieces in patterns {
        let Some(profile) = generated_depth_two_component_profile(active_pieces) else {
            continue;
        };
        if profile.recursive_nodes <= GENERATED_DEPTH_TWO_MAX_COMPONENT_RECURSIVE_NODES
            && seen_value_digests.insert(profile.value_digest.clone())
        {
            profiles.push(profile);
        }
    }
    profiles
}

fn generated_depth_two_profile_inventory_side(
    patterns: Vec<Vec<(Square, char)>>,
) -> GeneratedDepthTwoProfileInventorySideReport {
    let pattern_count = patterns.len();
    let mut wall_safe_pattern_count = 0usize;
    let mut profiles = Vec::new();
    let mut seen_value_digests = BTreeSet::new();
    let mut rejection_counts = BTreeMap::new();

    for active_pieces in patterns {
        let board = non_fixture_component_profile_board(&active_pieces);
        if !generated_component_pattern_respects_wall(&board, &active_pieces) {
            increment_count(&mut rejection_counts, "wall_safety");
            continue;
        }
        wall_safe_pattern_count += 1;

        let Some(profile) = generated_depth_two_component_profile(active_pieces) else {
            increment_count(&mut rejection_counts, "materialization_failure");
            continue;
        };
        if profile.recursive_nodes > GENERATED_DEPTH_TWO_MAX_COMPONENT_RECURSIVE_NODES {
            increment_count(&mut rejection_counts, "component_recursive_node_budget");
            continue;
        }
        if !seen_value_digests.insert(profile.value_digest.clone()) {
            increment_count(&mut rejection_counts, "duplicate_value_digest");
            continue;
        }

        let payload = profile.value.exact_value_payload();
        profiles.push(GeneratedDepthTwoComponentProfileReport {
            profile_index: profiles.len(),
            active_pieces: generated_depth_two_active_piece_summary(&profile.active_pieces),
            value_class: payload.value_class.as_str().to_owned(),
            value_digest: payload.digest,
            recursive_nodes: profile.recursive_nodes,
        });
    }

    GeneratedDepthTwoProfileInventorySideReport {
        pattern_count,
        wall_safe_pattern_count,
        accepted_profile_count: profiles.len(),
        rejection_counts,
        profiles,
    }
}

fn generated_depth_two_named_profile_inventory_report(
    source: &str,
    white_patterns: Vec<Vec<(Square, char)>>,
    black_patterns: Vec<Vec<(Square, char)>>,
) -> GeneratedDepthTwoNamedProfileInventoryReport {
    GeneratedDepthTwoNamedProfileInventoryReport {
        source: source.to_owned(),
        white: generated_depth_two_profile_inventory_side(white_patterns),
        black: generated_depth_two_profile_inventory_side(black_patterns),
    }
}

fn generated_depth_two_duplicate_cluster_side(
    patterns: Vec<Vec<(Square, char)>>,
) -> GeneratedDepthTwoDuplicateClusterSideReport {
    let pattern_count = patterns.len();
    let mut wall_safe_pattern_count = 0usize;
    let mut rejection_counts = BTreeMap::new();
    let mut profiles_by_digest: BTreeMap<String, Vec<GeneratedDepthTwoDuplicateClusterExample>> =
        BTreeMap::new();

    for active_pieces in patterns {
        let board = non_fixture_component_profile_board(&active_pieces);
        if !generated_component_pattern_respects_wall(&board, &active_pieces) {
            increment_count(&mut rejection_counts, "wall_safety");
            continue;
        }
        wall_safe_pattern_count += 1;

        let Some(profile) = generated_depth_two_component_profile(active_pieces) else {
            increment_count(&mut rejection_counts, "materialization_failure");
            continue;
        };
        if profile.recursive_nodes > GENERATED_DEPTH_TWO_MAX_COMPONENT_RECURSIVE_NODES {
            increment_count(&mut rejection_counts, "component_recursive_node_budget");
            continue;
        }
        let Some((material_balance, local_move_counts)) =
            generated_depth_two_component_signature(&profile.active_pieces)
        else {
            increment_count(&mut rejection_counts, "signature_failure");
            continue;
        };

        profiles_by_digest
            .entry(profile.value_digest.clone())
            .or_default()
            .push(GeneratedDepthTwoDuplicateClusterExample {
                active_pieces: generated_depth_two_active_piece_summary(&profile.active_pieces),
                material_balance,
                local_move_counts: format!(
                    "white:{},black:{}",
                    local_move_counts.white, local_move_counts.black
                ),
                recursive_nodes: profile.recursive_nodes,
            });
    }

    let budget_profile_count = profiles_by_digest
        .values()
        .map(std::vec::Vec::len)
        .sum::<usize>();
    let unique_value_digest_count = profiles_by_digest.len();
    let duplicate_profile_count = profiles_by_digest
        .values()
        .filter(|profiles| profiles.len() > 1)
        .map(std::vec::Vec::len)
        .sum::<usize>();
    let mut clusters = profiles_by_digest
        .into_iter()
        .filter_map(|(value_digest, mut examples)| {
            if examples.len() <= 1 {
                return None;
            }
            examples.sort_by(|left, right| {
                left.active_pieces
                    .cmp(&right.active_pieces)
                    .then(left.material_balance.cmp(&right.material_balance))
                    .then(left.local_move_counts.cmp(&right.local_move_counts))
                    .then(left.recursive_nodes.cmp(&right.recursive_nodes))
            });
            let signatures = examples
                .iter()
                .map(|example| {
                    format!(
                        "material:{},moves:{}",
                        example.material_balance, example.local_move_counts
                    )
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            Some(GeneratedDepthTwoDuplicateClusterSummary {
                value_digest,
                profile_count: examples.len(),
                distinct_signature_count: signatures.len(),
                signatures,
                examples: examples
                    .into_iter()
                    .take(GENERATED_DEPTH_TWO_DUPLICATE_CLUSTER_EXAMPLE_LIMIT)
                    .collect(),
            })
        })
        .collect::<Vec<_>>();
    clusters.sort_by(|left, right| {
        right
            .profile_count
            .cmp(&left.profile_count)
            .then(left.value_digest.cmp(&right.value_digest))
    });
    let duplicate_cluster_count = clusters.len();
    clusters.truncate(GENERATED_DEPTH_TWO_DUPLICATE_CLUSTER_REPORT_LIMIT);

    GeneratedDepthTwoDuplicateClusterSideReport {
        pattern_count,
        wall_safe_pattern_count,
        budget_profile_count,
        unique_value_digest_count,
        duplicate_cluster_count,
        duplicate_profile_count,
        rejection_counts,
        clusters,
    }
}

fn generated_local_move_profile_inventory_side(
    patterns: Vec<Vec<(Square, char)>>,
    component_depth: u8,
) -> GeneratedDepthTwoProfileInventorySideReport {
    let pattern_count = patterns.len();
    let mut wall_safe_pattern_count = 0usize;
    let mut profiles = Vec::new();
    let mut seen_value_digests = BTreeSet::new();
    let mut rejection_counts = BTreeMap::new();

    for active_pieces in patterns {
        let board = non_fixture_component_profile_board(&active_pieces);
        if !generated_component_pattern_respects_wall(&board, &active_pieces) {
            increment_count(&mut rejection_counts, "wall_safety");
            continue;
        }
        wall_safe_pattern_count += 1;

        let Some(profile) = generated_local_move_component_profile(active_pieces, component_depth)
        else {
            increment_count(&mut rejection_counts, "materialization_failure");
            continue;
        };
        if profile.recursive_nodes > GENERATED_DEPTH_TWO_MAX_COMPONENT_RECURSIVE_NODES {
            increment_count(&mut rejection_counts, "component_recursive_node_budget");
            continue;
        }
        if !seen_value_digests.insert(profile.value_digest.clone()) {
            increment_count(&mut rejection_counts, "duplicate_value_digest");
            continue;
        }

        let payload = profile.value.exact_value_payload();
        profiles.push(GeneratedDepthTwoComponentProfileReport {
            profile_index: profiles.len(),
            active_pieces: generated_depth_two_active_piece_summary(&profile.active_pieces),
            value_class: payload.value_class.as_str().to_owned(),
            value_digest: payload.digest,
            recursive_nodes: profile.recursive_nodes,
        });
    }

    GeneratedDepthTwoProfileInventorySideReport {
        pattern_count,
        wall_safe_pattern_count,
        accepted_profile_count: profiles.len(),
        rejection_counts,
        profiles,
    }
}

fn generated_depth_two_wall_safe_component_profiles(
    patterns: Vec<Vec<(Square, char)>>,
) -> Vec<GeneratedDepthTwoComponentProfile> {
    generated_depth_two_component_profiles(
        patterns
            .into_iter()
            .filter(|active_pieces| {
                let board = non_fixture_component_profile_board(active_pieces);
                generated_component_pattern_respects_wall(&board, active_pieces)
            })
            .collect(),
    )
}

fn generated_depth_two_signature_profiles(
    patterns: Vec<Vec<(Square, char)>>,
) -> Vec<GeneratedDepthTwoSignatureProfile> {
    let mut profiles = Vec::new();
    let mut seen_signatures = BTreeSet::new();
    for active_pieces in patterns {
        let board = non_fixture_component_profile_board(&active_pieces);
        if !generated_component_pattern_respects_wall(&board, &active_pieces) {
            continue;
        }
        let Some(profile) = generated_depth_two_component_profile(active_pieces) else {
            continue;
        };
        if profile.recursive_nodes > GENERATED_DEPTH_TWO_MAX_COMPONENT_RECURSIVE_NODES {
            continue;
        }
        let Some((material_balance, local_move_counts)) =
            generated_depth_two_component_signature(&profile.active_pieces)
        else {
            continue;
        };
        let component_signature = generated_depth_two_component_signature_key(
            &profile.value_digest,
            material_balance,
            local_move_counts,
        );
        if seen_signatures.insert(component_signature.clone()) {
            profiles.push(GeneratedDepthTwoSignatureProfile {
                active_pieces: profile.active_pieces,
                value: profile.value,
                value_digest: profile.value_digest,
                component_signature,
                recursive_nodes: profile.recursive_nodes,
            });
        }
    }
    profiles
}

fn generated_depth_two_signature_profile_candidate_pairs(
    family_index: usize,
    white_profiles: &[GeneratedDepthTwoSignatureProfile],
    black_profiles: &[GeneratedDepthTwoSignatureProfile],
) -> Vec<(usize, usize, usize)> {
    generated_depth_two_signature_profile_candidate_pairs_with_order(
        family_index,
        white_profiles,
        black_profiles,
        GeneratedDepthTwoCandidatePairOrder::ProfileIndexSpread,
    )
}

fn generated_depth_two_signature_profile_candidate_pairs_with_order(
    family_index: usize,
    white_profiles: &[GeneratedDepthTwoSignatureProfile],
    black_profiles: &[GeneratedDepthTwoSignatureProfile],
    candidate_pair_order: GeneratedDepthTwoCandidatePairOrder,
) -> Vec<(usize, usize, usize)> {
    let mut pairs = Vec::new();
    for (left_index, left) in white_profiles.iter().enumerate() {
        for (right_index, right) in black_profiles.iter().enumerate() {
            let total_recursive_nodes = left.recursive_nodes + right.recursive_nodes;
            if total_recursive_nodes > GENERATED_DEPTH_TWO_MAX_RECURSIVE_NODES
                || left.component_signature == right.component_signature
            {
                continue;
            }
            pairs.push((left_index, right_index, total_recursive_nodes));
        }
    }

    let product = white_profiles
        .len()
        .saturating_mul(black_profiles.len())
        .max(1);
    match candidate_pair_order {
        GeneratedDepthTwoCandidatePairOrder::ProfileIndexSpread => {
            pairs.sort_by_key(|(left_index, right_index, total_recursive_nodes)| {
                (
                    (left_index * 37 + right_index * 53 + family_index * 97) % product,
                    *total_recursive_nodes,
                    *left_index,
                    *right_index,
                )
            });
        }
        GeneratedDepthTwoCandidatePairOrder::ComponentValueDigestSpread => {
            let left_value_ranks = generated_signature_profile_value_digest_ranks(white_profiles);
            let right_value_ranks = generated_signature_profile_value_digest_ranks(black_profiles);
            let value_product = left_value_ranks
                .len()
                .saturating_mul(right_value_ranks.len())
                .max(1);
            pairs.sort_by_key(|(left_index, right_index, total_recursive_nodes)| {
                let left = &white_profiles[*left_index];
                let right = &black_profiles[*right_index];
                let left_value_rank = left_value_ranks
                    .get(&left.value_digest)
                    .copied()
                    .expect("left profile value digest must have a rank");
                let right_value_rank = right_value_ranks
                    .get(&right.value_digest)
                    .copied()
                    .expect("right profile value digest must have a rank");
                (
                    (left_value_rank * 37 + right_value_rank * 53 + family_index * 97)
                        % value_product,
                    *total_recursive_nodes,
                    *left_index,
                    *right_index,
                )
            });
        }
    }
    pairs
}

fn generated_signature_profile_value_digest_ranks(
    profiles: &[GeneratedDepthTwoSignatureProfile],
) -> BTreeMap<String, usize> {
    let mut ranks = BTreeMap::new();
    for profile in profiles {
        let next_rank = ranks.len();
        ranks
            .entry(profile.value_digest.clone())
            .or_insert(next_rank);
    }
    ranks
}

fn generated_depth_two_component_signature_key(
    value_digest: &str,
    material_balance: i32,
    local_move_counts: ComponentLocalMoveCounts,
) -> String {
    format!(
        "value:{value_digest};material:{material_balance};moves:white:{},black:{}",
        local_move_counts.white, local_move_counts.black
    )
}

fn generated_depth_two_result_signature_key(
    topology_family: &str,
    left_component_signature: &str,
    right_component_signature: &str,
) -> String {
    format!("{topology_family};left:{left_component_signature};right:{right_component_signature}")
}

fn generated_depth_two_component_profile(
    active_pieces: Vec<(Square, char)>,
) -> Option<GeneratedDepthTwoComponentProfile> {
    let board = non_fixture_component_profile_board(&active_pieces);
    let decomposition = bitmesh::certify_decomposition(&board);
    let mut components = decomposition
        .components
        .iter()
        .filter(|component| component.active_mask.into_iter().next().is_some());
    let component = components.next()?;
    if components.next().is_some() {
        return None;
    }
    let material_value = component_material_balance(&board, component.active_mask);
    let local_move_counts =
        component_local_move_counts(&board, component.mask, component.active_mask);
    let component_evaluation = component_cgt_evaluation(
        NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame,
        &board,
        component.mask,
        component.active_mask,
        material_value,
        local_move_counts,
    );
    let payload = component_evaluation.value.exact_value_payload();
    Some(GeneratedDepthTwoComponentProfile {
        active_pieces,
        value: component_evaluation.value,
        value_digest: payload.digest,
        recursive_nodes: component_evaluation.recursive_nodes?,
    })
}

fn generated_depth_two_component_signature(
    active_pieces: &[(Square, char)],
) -> Option<(i32, ComponentLocalMoveCounts)> {
    let board = non_fixture_component_profile_board(active_pieces);
    let decomposition = bitmesh::certify_decomposition(&board);
    let mut components = decomposition
        .components
        .iter()
        .filter(|component| component.active_mask.into_iter().next().is_some());
    let component = components.next()?;
    if components.next().is_some() {
        return None;
    }
    Some((
        component_material_balance(&board, component.active_mask),
        component_local_move_counts(&board, component.mask, component.active_mask),
    ))
}

fn generated_local_move_component_profile(
    active_pieces: Vec<(Square, char)>,
    component_depth: u8,
) -> Option<GeneratedDepthTwoComponentProfile> {
    let board = non_fixture_component_profile_board(&active_pieces);
    let decomposition = bitmesh::certify_decomposition(&board);
    let mut components = decomposition
        .components
        .iter()
        .filter(|component| component.active_mask.into_iter().next().is_some());
    let component = components.next()?;
    if components.next().is_some() {
        return None;
    }
    let (value, recursive_nodes) = component_recursive_local_move_game_value(
        &board,
        component.mask,
        component.active_mask,
        component_depth,
    );
    let payload = value.exact_value_payload();
    Some(GeneratedDepthTwoComponentProfile {
        active_pieces,
        value,
        value_digest: payload.digest,
        recursive_nodes,
    })
}

fn generated_component_pattern_respects_wall(
    board: &Board,
    active_pieces: &[(Square, char)],
) -> bool {
    // The composed-board proof rejects generated components that can exchange captures
    // with the alternating-color D-file wall before component decomposition.
    let wall_pieces = non_fixture_composed_board_wall_pieces();
    for (square, _) in active_pieces {
        let piece = board
            .piece_at(*square)
            .expect("generated active square must contain a piece");
        let attacks = if piece.role == shakmaty::Role::Pawn {
            shakmaty::attacks::pawn_attacks(piece.color, *square)
        } else {
            shakmaty::attacks::attacks(*square, piece, board.occupied())
        };

        for (wall_square, wall_piece) in &wall_pieces {
            let wall_piece = composition_fixture_piece(*wall_piece);
            if wall_piece.color != piece.color && attacks.contains(*wall_square) {
                return false;
            }
        }
    }

    for (wall_square, wall_piece) in &wall_pieces {
        let wall_piece = composition_fixture_piece(*wall_piece);
        let attacks = shakmaty::attacks::pawn_attacks(wall_piece.color, *wall_square);
        for (active_square, _) in active_pieces {
            let active_piece = board
                .piece_at(*active_square)
                .expect("generated active square must contain a piece");
            if active_piece.color != wall_piece.color && attacks.contains(*active_square) {
                return false;
            }
        }
    }

    true
}

fn non_fixture_component_profile_board(active_pieces: &[(Square, char)]) -> Board {
    let mut board = Board::empty();
    for (square, piece) in non_fixture_composed_board_wall_pieces() {
        board.set_piece_at(square, composition_fixture_piece(piece));
    }
    for (square, piece) in active_pieces {
        board.set_piece_at(*square, composition_fixture_piece(*piece));
    }
    board
}

fn generated_white_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::A1, 'N'), (Square::A2, 'P'), (Square::B2, 'P')],
        vec![(Square::A1, 'N'), (Square::A2, 'P')],
        vec![
            (Square::A1, 'N'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
        ],
        vec![(Square::A1, 'N'), (Square::B1, 'B'), (Square::C2, 'P')],
        vec![
            (Square::A1, 'N'),
            (Square::B1, 'B'),
            (Square::A2, 'P'),
            (Square::C2, 'P'),
        ],
        vec![
            (Square::A1, 'N'),
            (Square::B1, 'B'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
        ],
        vec![(Square::A1, 'B'), (Square::B2, 'P'), (Square::C2, 'P')],
        vec![
            (Square::A1, 'B'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
        ],
        vec![(Square::A1, 'R'), (Square::B1, 'B'), (Square::C2, 'P')],
        vec![
            (Square::A1, 'R'),
            (Square::B1, 'B'),
            (Square::A2, 'P'),
            (Square::C2, 'P'),
        ],
        vec![
            (Square::A1, 'Q'),
            (Square::B1, 'B'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
        ],
        vec![
            (Square::A1, 'Q'),
            (Square::B1, 'B'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
        ],
        vec![
            (Square::A1, 'N'),
            (Square::C1, 'R'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
        ],
        vec![
            (Square::A1, 'N'),
            (Square::C1, 'N'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
        ],
        vec![
            (Square::B1, 'R'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
        ],
        vec![
            (Square::A1, 'R'),
            (Square::B1, 'B'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
        ],
        vec![(Square::A1, 'Q'), (Square::B1, 'B'), (Square::C2, 'P')],
        vec![
            (Square::A1, 'R'),
            (Square::B1, 'R'),
            (Square::A2, 'P'),
            (Square::C2, 'P'),
        ],
        vec![
            (Square::A1, 'Q'),
            (Square::B1, 'R'),
            (Square::A2, 'P'),
            (Square::C2, 'P'),
        ],
        vec![
            (Square::A1, 'Q'),
            (Square::B1, 'R'),
            (Square::A2, 'P'),
            (Square::B2, 'P'),
            (Square::C2, 'P'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::A1, &['N', 'B', 'R', 'Q'][..]),
        (Square::B1, &['N', 'B', 'R', 'Q'][..]),
        (Square::C1, &['N', 'B', 'R', 'Q'][..]),
        (Square::A2, &['P', 'N'][..]),
        (Square::B2, &['P', 'N'][..]),
        (Square::C2, &['P', 'N'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_black_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::H8, 'n'), (Square::H7, 'p')],
        vec![(Square::H8, 'n'), (Square::H7, 'p'), (Square::G7, 'p')],
        vec![
            (Square::H8, 'n'),
            (Square::H7, 'p'),
            (Square::G7, 'p'),
            (Square::F7, 'p'),
        ],
        vec![(Square::H8, 'n'), (Square::G8, 'b'), (Square::F7, 'p')],
        vec![
            (Square::H8, 'n'),
            (Square::G8, 'b'),
            (Square::H7, 'p'),
            (Square::F7, 'p'),
        ],
        vec![
            (Square::H8, 'n'),
            (Square::G8, 'b'),
            (Square::H7, 'p'),
            (Square::G7, 'p'),
            (Square::F7, 'p'),
        ],
        vec![(Square::H8, 'b'), (Square::G7, 'p'), (Square::F7, 'p')],
        vec![
            (Square::H8, 'b'),
            (Square::H7, 'p'),
            (Square::G7, 'p'),
            (Square::F7, 'p'),
        ],
        vec![(Square::H8, 'r'), (Square::G8, 'b'), (Square::F7, 'p')],
        vec![
            (Square::H8, 'r'),
            (Square::G8, 'b'),
            (Square::H7, 'p'),
            (Square::F7, 'p'),
        ],
        vec![
            (Square::H8, 'q'),
            (Square::G8, 'b'),
            (Square::G7, 'p'),
            (Square::F7, 'p'),
        ],
        vec![
            (Square::H8, 'q'),
            (Square::G8, 'b'),
            (Square::H7, 'p'),
            (Square::G7, 'p'),
            (Square::F7, 'p'),
        ],
        vec![
            (Square::H8, 'n'),
            (Square::F8, 'r'),
            (Square::H7, 'p'),
            (Square::G7, 'p'),
        ],
        vec![
            (Square::H8, 'n'),
            (Square::F8, 'n'),
            (Square::H7, 'p'),
            (Square::G7, 'p'),
        ],
        vec![
            (Square::G8, 'r'),
            (Square::H7, 'p'),
            (Square::G7, 'p'),
            (Square::F7, 'p'),
        ],
        vec![(Square::H8, 'q'), (Square::H7, 'p'), (Square::G7, 'p')],
        vec![(Square::H8, 'q'), (Square::G8, 'b'), (Square::H7, 'p')],
        vec![
            (Square::H8, 'q'),
            (Square::G8, 'r'),
            (Square::H7, 'p'),
            (Square::G7, 'p'),
        ],
        vec![
            (Square::H8, 'q'),
            (Square::G8, 'r'),
            (Square::F8, 'n'),
            (Square::H7, 'p'),
        ],
        vec![
            (Square::H8, 'q'),
            (Square::G8, 'r'),
            (Square::F8, 'n'),
            (Square::H7, 'p'),
            (Square::G7, 'p'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::H8, &['n', 'b', 'r', 'q'][..]),
        (Square::G8, &['n', 'b', 'r', 'q'][..]),
        (Square::F8, &['n', 'b', 'r', 'q'][..]),
        (Square::H7, &['p', 'n'][..]),
        (Square::G7, &['p', 'n'][..]),
        (Square::F7, &['p', 'n'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_white_edge_minor_ladder_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::A1, 'N'), (Square::A3, 'P')],
        vec![(Square::A1, 'B'), (Square::B2, 'P'), (Square::B3, 'P')],
        vec![(Square::B1, 'N'), (Square::A2, 'P'), (Square::A3, 'P')],
        vec![
            (Square::A1, 'R'),
            (Square::B1, 'B'),
            (Square::A3, 'P'),
            (Square::B3, 'P'),
        ],
        vec![
            (Square::A1, 'Q'),
            (Square::B1, 'N'),
            (Square::A2, 'P'),
            (Square::B3, 'P'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::A1, &['N', 'B', 'R', 'Q'][..]),
        (Square::B1, &['N', 'B', 'R', 'Q'][..]),
        (Square::A2, &['P', 'N', 'B'][..]),
        (Square::B2, &['P', 'N', 'B'][..]),
        (Square::A3, &['P', 'N'][..]),
        (Square::B3, &['P', 'N'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_black_edge_minor_ladder_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::H8, 'n'), (Square::H6, 'p')],
        vec![(Square::H8, 'b'), (Square::G7, 'p'), (Square::G6, 'p')],
        vec![(Square::G8, 'n'), (Square::H7, 'p'), (Square::H6, 'p')],
        vec![
            (Square::H8, 'r'),
            (Square::G8, 'b'),
            (Square::H6, 'p'),
            (Square::G6, 'p'),
        ],
        vec![
            (Square::H8, 'q'),
            (Square::G8, 'n'),
            (Square::H7, 'p'),
            (Square::G6, 'p'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::H8, &['n', 'b', 'r', 'q'][..]),
        (Square::G8, &['n', 'b', 'r', 'q'][..]),
        (Square::H7, &['p', 'n', 'b'][..]),
        (Square::G7, &['p', 'n', 'b'][..]),
        (Square::H6, &['p', 'n'][..]),
        (Square::G6, &['p', 'n'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_white_rank4_minor_ladder_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::A1, 'N'), (Square::A4, 'P'), (Square::B4, 'P')],
        vec![(Square::A1, 'B'), (Square::A3, 'P'), (Square::B4, 'N')],
        vec![(Square::B1, 'N'), (Square::A3, 'P'), (Square::A4, 'P')],
        vec![
            (Square::A1, 'R'),
            (Square::B1, 'N'),
            (Square::A4, 'P'),
            (Square::B4, 'P'),
        ],
        vec![
            (Square::A1, 'Q'),
            (Square::B1, 'B'),
            (Square::A3, 'P'),
            (Square::B4, 'P'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::A1, &['N', 'B', 'R', 'Q'][..]),
        (Square::B1, &['N', 'B', 'R'][..]),
        (Square::A3, &['P', 'N', 'B'][..]),
        (Square::B3, &['P', 'N', 'B'][..]),
        (Square::A4, &['P', 'N'][..]),
        (Square::B4, &['P', 'N'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_black_rank5_minor_ladder_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::H8, 'n'), (Square::H5, 'p'), (Square::G5, 'p')],
        vec![(Square::H8, 'b'), (Square::H6, 'p'), (Square::G5, 'n')],
        vec![(Square::G8, 'n'), (Square::H6, 'p'), (Square::H5, 'p')],
        vec![
            (Square::H8, 'r'),
            (Square::G8, 'n'),
            (Square::H5, 'p'),
            (Square::G5, 'p'),
        ],
        vec![
            (Square::H8, 'q'),
            (Square::G8, 'b'),
            (Square::H6, 'p'),
            (Square::G5, 'p'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::H8, &['n', 'b', 'r', 'q'][..]),
        (Square::G8, &['n', 'b', 'r'][..]),
        (Square::H6, &['p', 'n', 'b'][..]),
        (Square::G6, &['p', 'n', 'b'][..]),
        (Square::H5, &['p', 'n'][..]),
        (Square::G5, &['p', 'n'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_white_cfile_minor_bridge_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::A1, 'N'), (Square::B3, 'P'), (Square::C4, 'N')],
        vec![(Square::B1, 'B'), (Square::A3, 'P'), (Square::C4, 'P')],
        vec![(Square::C1, 'N'), (Square::B3, 'P'), (Square::C4, 'P')],
        vec![(Square::A1, 'R'), (Square::B3, 'N'), (Square::C4, 'P')],
        vec![
            (Square::A1, 'Q'),
            (Square::B1, 'N'),
            (Square::B3, 'P'),
            (Square::C4, 'P'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::A1, &['N', 'B', 'R', 'Q'][..]),
        (Square::B1, &['N', 'B', 'R'][..]),
        (Square::C1, &['N', 'B'][..]),
        (Square::A3, &['P', 'N'][..]),
        (Square::B3, &['P', 'N', 'B'][..]),
        (Square::C4, &['P', 'N', 'B'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_black_ffile_minor_bridge_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::H8, 'n'), (Square::G6, 'p'), (Square::F5, 'n')],
        vec![(Square::G8, 'b'), (Square::H6, 'p'), (Square::F5, 'p')],
        vec![(Square::F8, 'n'), (Square::G6, 'p'), (Square::F5, 'p')],
        vec![(Square::H8, 'r'), (Square::G6, 'n'), (Square::F5, 'p')],
        vec![
            (Square::H8, 'q'),
            (Square::G8, 'n'),
            (Square::G6, 'p'),
            (Square::F5, 'p'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::H8, &['n', 'b', 'r', 'q'][..]),
        (Square::G8, &['n', 'b', 'r'][..]),
        (Square::F8, &['n', 'b'][..]),
        (Square::H6, &['p', 'n'][..]),
        (Square::G6, &['p', 'n', 'b'][..]),
        (Square::F5, &['p', 'n', 'b'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_white_wide_pawn_shelf_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![
            (Square::A1, 'N'),
            (Square::A4, 'P'),
            (Square::B4, 'P'),
            (Square::C4, 'P'),
        ],
        vec![(Square::B1, 'N'), (Square::B3, 'P'), (Square::C4, 'P')],
        vec![
            (Square::A1, 'B'),
            (Square::B3, 'P'),
            (Square::B4, 'N'),
            (Square::C4, 'P'),
        ],
        vec![
            (Square::A1, 'R'),
            (Square::B1, 'N'),
            (Square::B3, 'P'),
            (Square::C4, 'P'),
        ],
        vec![
            (Square::A1, 'Q'),
            (Square::B1, 'B'),
            (Square::A4, 'P'),
            (Square::C4, 'P'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::A1, &['N', 'B', 'R'][..]),
        (Square::B1, &['N', 'B', 'R', 'Q'][..]),
        (Square::B3, &['P', 'N', 'B'][..]),
        (Square::A4, &['P', 'N'][..]),
        (Square::B4, &['P', 'N'][..]),
        (Square::C4, &['P', 'N'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_black_wide_pawn_shelf_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![
            (Square::H8, 'n'),
            (Square::H5, 'p'),
            (Square::G5, 'p'),
            (Square::F5, 'p'),
        ],
        vec![(Square::G8, 'n'), (Square::G6, 'p'), (Square::F5, 'p')],
        vec![
            (Square::H8, 'b'),
            (Square::G6, 'p'),
            (Square::G5, 'n'),
            (Square::F5, 'p'),
        ],
        vec![
            (Square::H8, 'r'),
            (Square::G8, 'n'),
            (Square::G6, 'p'),
            (Square::F5, 'p'),
        ],
        vec![
            (Square::H8, 'q'),
            (Square::G8, 'b'),
            (Square::H5, 'p'),
            (Square::F5, 'p'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::H8, &['n', 'b', 'r'][..]),
        (Square::G8, &['n', 'b', 'r', 'q'][..]),
        (Square::G6, &['p', 'n', 'b'][..]),
        (Square::H5, &['p', 'n'][..]),
        (Square::G5, &['p', 'n'][..]),
        (Square::F5, &['p', 'n'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_white_mixed_color_hook_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::A1, 'N'), (Square::A2, 'P'), (Square::B3, 'p')],
        vec![
            (Square::A1, 'N'),
            (Square::A2, 'P'),
            (Square::B3, 'p'),
            (Square::C2, 'P'),
        ],
        vec![(Square::A1, 'R'), (Square::B1, 'n'), (Square::A2, 'P')],
        vec![(Square::A1, 'R'), (Square::B1, 'n'), (Square::B2, 'P')],
        vec![(Square::A1, 'B'), (Square::B2, 'P'), (Square::C3, 'p')],
        vec![(Square::A2, 'P'), (Square::B2, 'N'), (Square::C3, 'p')],
        vec![(Square::A1, 'Q'), (Square::B2, 'p'), (Square::C2, 'P')],
        vec![
            (Square::A1, 'N'),
            (Square::A2, 'P'),
            (Square::B3, 'p'),
            (Square::C4, 'P'),
        ],
        vec![
            (Square::A2, 'P'),
            (Square::B2, 'N'),
            (Square::B3, 'p'),
            (Square::C4, 'P'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::A1, &['N', 'B', 'R', 'Q'][..]),
        (Square::B1, &['N', 'R', 'n'][..]),
        (Square::A2, &['P', 'N'][..]),
        (Square::B2, &['P', 'N', 'p', 'n'][..]),
        (Square::B3, &['P', 'N', 'p'][..]),
        (Square::C3, &['p'][..]),
        (Square::C4, &['P', 'N'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_black_mixed_color_hook_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::H8, 'n'), (Square::H7, 'p'), (Square::G6, 'P')],
        vec![
            (Square::H8, 'n'),
            (Square::H7, 'p'),
            (Square::G6, 'P'),
            (Square::F7, 'p'),
        ],
        vec![(Square::H8, 'r'), (Square::G8, 'N'), (Square::H7, 'p')],
        vec![(Square::H8, 'r'), (Square::G8, 'N'), (Square::G7, 'p')],
        vec![(Square::H8, 'b'), (Square::G7, 'p'), (Square::F6, 'P')],
        vec![(Square::H7, 'p'), (Square::G7, 'n'), (Square::F6, 'P')],
        vec![(Square::H8, 'q'), (Square::G7, 'P'), (Square::F7, 'p')],
        vec![
            (Square::H8, 'n'),
            (Square::H7, 'p'),
            (Square::G6, 'P'),
            (Square::F5, 'p'),
        ],
        vec![
            (Square::H7, 'p'),
            (Square::G7, 'n'),
            (Square::G6, 'P'),
            (Square::F5, 'p'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::H8, &['n', 'b', 'r', 'q'][..]),
        (Square::G8, &['n', 'r', 'N'][..]),
        (Square::H7, &['p', 'n'][..]),
        (Square::G7, &['p', 'n', 'P', 'N'][..]),
        (Square::G6, &['p', 'n', 'P'][..]),
        (Square::F6, &['P'][..]),
        (Square::F5, &['p', 'n'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_white_expanded_mixed_color_hook_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::B1, 'N'), (Square::C2, 'P'), (Square::D3, 'p')],
        vec![
            (Square::A1, 'B'),
            (Square::B2, 'P'),
            (Square::C3, 'n'),
            (Square::D4, 'P'),
        ],
        vec![(Square::C1, 'R'), (Square::B2, 'p'), (Square::C2, 'P')],
        vec![(Square::A1, 'Q'), (Square::C2, 'P'), (Square::D3, 'p')],
        vec![
            (Square::B1, 'N'),
            (Square::B2, 'P'),
            (Square::C3, 'p'),
            (Square::D4, 'N'),
        ],
        vec![
            (Square::A2, 'P'),
            (Square::B3, 'N'),
            (Square::C3, 'p'),
            (Square::D4, 'P'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::A1, &['N', 'B', 'R', 'Q'][..]),
        (Square::B1, &['N', 'B', 'R'][..]),
        (Square::C1, &['N', 'B', 'R'][..]),
        (Square::A2, &['P', 'N'][..]),
        (Square::B2, &['P', 'N', 'p', 'n'][..]),
        (Square::C2, &['P', 'N', 'p', 'n'][..]),
        (Square::B3, &['P', 'N', 'p'][..]),
        (Square::C3, &['P', 'N', 'p', 'n'][..]),
        (Square::D3, &['P', 'N', 'p', 'n'][..]),
        (Square::D4, &['P', 'N'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_black_expanded_mixed_color_hook_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::G8, 'n'), (Square::F7, 'p'), (Square::E6, 'P')],
        vec![
            (Square::H8, 'b'),
            (Square::G7, 'p'),
            (Square::F6, 'N'),
            (Square::E5, 'p'),
        ],
        vec![(Square::F8, 'r'), (Square::G7, 'P'), (Square::F7, 'p')],
        vec![(Square::H8, 'q'), (Square::F7, 'p'), (Square::E6, 'P')],
        vec![
            (Square::G8, 'n'),
            (Square::G7, 'p'),
            (Square::F6, 'P'),
            (Square::E5, 'n'),
        ],
        vec![
            (Square::H7, 'p'),
            (Square::G6, 'n'),
            (Square::F6, 'P'),
            (Square::E5, 'p'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::H8, &['n', 'b', 'r', 'q'][..]),
        (Square::G8, &['n', 'b', 'r'][..]),
        (Square::F8, &['n', 'b', 'r'][..]),
        (Square::H7, &['p', 'n'][..]),
        (Square::G7, &['p', 'n', 'P', 'N'][..]),
        (Square::F7, &['p', 'n', 'P', 'N'][..]),
        (Square::G6, &['p', 'n', 'P'][..]),
        (Square::F6, &['p', 'n', 'P', 'N'][..]),
        (Square::E6, &['p', 'n', 'P', 'N'][..]),
        (Square::E5, &['p', 'n'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_white_interior_mixed_color_hook_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::C1, 'N'), (Square::C2, 'P'), (Square::E3, 'p')],
        vec![
            (Square::C1, 'B'),
            (Square::C2, 'P'),
            (Square::E3, 'p'),
            (Square::C4, 'N'),
        ],
        vec![(Square::B1, 'R'), (Square::C2, 'p'), (Square::E2, 'P')],
        vec![(Square::C1, 'Q'), (Square::B2, 'P'), (Square::E3, 'p')],
        vec![
            (Square::B1, 'N'),
            (Square::C2, 'P'),
            (Square::E3, 'p'),
            (Square::C4, 'P'),
        ],
        vec![
            (Square::C2, 'P'),
            (Square::B3, 'N'),
            (Square::E3, 'p'),
            (Square::C4, 'P'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::B1, &['N', 'B', 'R', 'Q'][..]),
        (Square::C1, &['N', 'B', 'R', 'Q'][..]),
        (Square::E1, &['N', 'B', 'R'][..]),
        (Square::B2, &['P', 'N', 'p'][..]),
        (Square::C2, &['P', 'N', 'p', 'n'][..]),
        (Square::E2, &['P', 'N', 'p', 'n'][..]),
        (Square::B3, &['P', 'N', 'p'][..]),
        (Square::C3, &['P', 'N', 'p', 'n'][..]),
        (Square::E3, &['P', 'N', 'p', 'n'][..]),
        (Square::C4, &['P', 'N'][..]),
        (Square::E4, &['P', 'N'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_white_near_wall_mixed_color_hook_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::C1, 'N'), (Square::C2, 'P'), (Square::C3, 'p')],
        vec![(Square::C1, 'B'), (Square::B2, 'P'), (Square::C3, 'n')],
        vec![(Square::B1, 'R'), (Square::C2, 'p'), (Square::B3, 'P')],
        vec![(Square::C1, 'Q'), (Square::B2, 'n'), (Square::C3, 'P')],
        vec![
            (Square::B1, 'N'),
            (Square::C2, 'P'),
            (Square::B3, 'p'),
            (Square::C4, 'P'),
        ],
        vec![
            (Square::C1, 'R'),
            (Square::B2, 'P'),
            (Square::C3, 'n'),
            (Square::B4, 'P'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::B1, &['N', 'B', 'R'][..]),
        (Square::C1, &['N', 'B', 'R', 'Q'][..]),
        (Square::B2, &['P', 'N', 'p', 'n'][..]),
        (Square::C2, &['P', 'N', 'p', 'n'][..]),
        (Square::B3, &['P', 'N', 'p', 'n'][..]),
        (Square::C3, &['P', 'N', 'p', 'n'][..]),
        (Square::B4, &['P', 'N'][..]),
        (Square::C4, &['P', 'N'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_white_outer_mixed_color_hook_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::A1, 'B'), (Square::A2, 'P'), (Square::A3, 'p')],
        vec![(Square::B1, 'N'), (Square::A2, 'p'), (Square::B3, 'P')],
        vec![(Square::A1, 'R'), (Square::B2, 'n'), (Square::A3, 'P')],
        vec![(Square::B1, 'Q'), (Square::A2, 'P'), (Square::C2, 'p')],
        vec![
            (Square::A1, 'N'),
            (Square::B2, 'P'),
            (Square::A3, 'p'),
            (Square::C3, 'P'),
        ],
        vec![
            (Square::B1, 'R'),
            (Square::A2, 'P'),
            (Square::B3, 'n'),
            (Square::A4, 'P'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::A1, &['N', 'B', 'R', 'Q'][..]),
        (Square::B1, &['N', 'B', 'R', 'Q'][..]),
        (Square::A2, &['P', 'N', 'p', 'n'][..]),
        (Square::B2, &['P', 'N', 'p', 'n'][..]),
        (Square::C2, &['P', 'N', 'p'][..]),
        (Square::A3, &['P', 'N', 'p'][..]),
        (Square::B3, &['P', 'N', 'p', 'n'][..]),
        (Square::C3, &['P', 'N'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_white_diagonal_mixed_color_hook_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::A1, 'N'), (Square::B2, 'P'), (Square::C3, 'p')],
        vec![(Square::A1, 'B'), (Square::B2, 'n'), (Square::C3, 'P')],
        vec![(Square::B1, 'R'), (Square::C2, 'P'), (Square::A3, 'p')],
        vec![(Square::C1, 'Q'), (Square::B2, 'p'), (Square::A3, 'P')],
        vec![
            (Square::A1, 'R'),
            (Square::B2, 'P'),
            (Square::C3, 'n'),
            (Square::C4, 'P'),
        ],
        vec![
            (Square::C1, 'N'),
            (Square::B2, 'P'),
            (Square::A3, 'p'),
            (Square::B4, 'N'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::A1, &['N', 'B', 'R', 'Q'][..]),
        (Square::B1, &['N', 'B', 'R'][..]),
        (Square::C1, &['N', 'B', 'R', 'Q'][..]),
        (Square::A2, &['P', 'N', 'p'][..]),
        (Square::B2, &['P', 'N', 'p', 'n'][..]),
        (Square::C2, &['P', 'N', 'p', 'n'][..]),
        (Square::A3, &['P', 'N', 'p'][..]),
        (Square::C3, &['P', 'N', 'p', 'n'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_black_interior_mixed_color_hook_component_patterns() -> Vec<Vec<(Square, char)>> {
    let mut patterns = vec![
        vec![(Square::F8, 'n'), (Square::F7, 'p'), (Square::E6, 'P')],
        vec![
            (Square::F8, 'b'),
            (Square::F7, 'p'),
            (Square::E6, 'P'),
            (Square::F5, 'n'),
        ],
        vec![(Square::G8, 'r'), (Square::F7, 'P'), (Square::E7, 'p')],
        vec![(Square::F8, 'q'), (Square::G7, 'p'), (Square::E6, 'P')],
        vec![
            (Square::G8, 'n'),
            (Square::F7, 'p'),
            (Square::E6, 'P'),
            (Square::F5, 'p'),
        ],
        vec![
            (Square::F7, 'p'),
            (Square::G6, 'n'),
            (Square::E6, 'P'),
            (Square::F5, 'p'),
        ],
    ];
    patterns.extend(generated_component_patterns(&[
        (Square::G8, &['n', 'b', 'r', 'q'][..]),
        (Square::F8, &['n', 'b', 'r', 'q'][..]),
        (Square::E8, &['n', 'b', 'r'][..]),
        (Square::G7, &['p', 'n', 'P'][..]),
        (Square::F7, &['p', 'n', 'P', 'N'][..]),
        (Square::E7, &['p', 'n', 'P', 'N'][..]),
        (Square::G6, &['p', 'n', 'P'][..]),
        (Square::F6, &['p', 'n', 'P', 'N'][..]),
        (Square::E6, &['p', 'n', 'P', 'N'][..]),
        (Square::F5, &['p', 'n'][..]),
        (Square::E5, &['p', 'n'][..]),
    ]));
    unique_generated_component_patterns(patterns)
}

fn generated_left_supply_outer_vs_expanded_right_component_patterns()
-> (Vec<Vec<(Square, char)>>, Vec<Vec<(Square, char)>>) {
    let current_white_patterns = generated_combined_component_patterns(
        generated_combined_component_patterns(
            generated_white_component_patterns(),
            generated_white_edge_minor_ladder_component_patterns(),
        ),
        generated_white_mixed_color_hook_component_patterns(),
    );
    let current_black_patterns = generated_combined_component_patterns(
        generated_combined_component_patterns(
            generated_black_component_patterns(),
            generated_black_edge_minor_ladder_component_patterns(),
        ),
        generated_black_mixed_color_hook_component_patterns(),
    );
    let white_patterns = generated_unbounded_combined_component_patterns(
        generated_unbounded_combined_component_patterns(
            current_white_patterns,
            generated_white_expanded_mixed_color_hook_component_patterns(),
        ),
        generated_white_outer_mixed_color_hook_component_patterns(),
    );
    let black_patterns = generated_combined_component_patterns(
        current_black_patterns,
        generated_black_expanded_mixed_color_hook_component_patterns(),
    );

    (white_patterns, black_patterns)
}

fn generated_combined_component_patterns(
    mut first: Vec<Vec<(Square, char)>>,
    second: Vec<Vec<(Square, char)>>,
) -> Vec<Vec<(Square, char)>> {
    first.extend(second);
    unique_generated_component_patterns(first)
}

fn generated_unbounded_combined_component_patterns(
    mut first: Vec<Vec<(Square, char)>>,
    second: Vec<Vec<(Square, char)>>,
) -> Vec<Vec<(Square, char)>> {
    first.extend(second);
    unique_generated_component_patterns_with_limit(first, usize::MAX)
}

fn unique_generated_component_patterns(
    patterns: Vec<Vec<(Square, char)>>,
) -> Vec<Vec<(Square, char)>> {
    unique_generated_component_patterns_with_limit(
        patterns,
        GENERATED_DEPTH_TWO_COMPONENT_PATTERN_LIMIT,
    )
}

fn unique_generated_component_patterns_with_limit(
    patterns: Vec<Vec<(Square, char)>>,
    limit: usize,
) -> Vec<Vec<(Square, char)>> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for mut pattern in patterns {
        pattern.sort_by_key(|(square, piece)| (usize::from(*square), *piece));
        let key = component_pattern_key(&pattern);
        if seen.insert(key) {
            unique.push(pattern);
        }
        if unique.len() == limit {
            break;
        }
    }
    unique
}

fn component_pattern_key(pattern: &[(Square, char)]) -> String {
    pattern
        .iter()
        .map(|(square, piece)| format!("{}{}", usize::from(*square), piece))
        .collect::<Vec<_>>()
        .join("|")
}

fn generated_component_patterns(options: &[(Square, &[char])]) -> Vec<Vec<(Square, char)>> {
    fn visit(
        options: &[(Square, &[char])],
        index: usize,
        current: &mut Vec<(Square, char)>,
        patterns: &mut Vec<Vec<(Square, char)>>,
    ) {
        if index == options.len() {
            if generated_component_pattern_is_in_scope(current) {
                patterns.push(current.clone());
            }
            return;
        }

        visit(options, index + 1, current, patterns);
        let (square, pieces) = options[index];
        for piece in pieces {
            current.push((square, *piece));
            visit(options, index + 1, current, patterns);
            current.pop();
        }
    }

    let mut patterns = Vec::new();
    visit(options, 0, &mut Vec::new(), &mut patterns);
    patterns.sort_by_key(|pieces| {
        (
            pieces.len(),
            pieces
                .iter()
                .map(|(square, piece)| format!("{}{}", usize::from(*square), piece))
                .collect::<Vec<_>>()
                .join("|"),
        )
    });
    patterns.truncate(GENERATED_DEPTH_TWO_COMPONENT_PATTERN_GROUP_LIMIT);
    patterns
}

fn generated_component_pattern_is_in_scope(pieces: &[(Square, char)]) -> bool {
    let piece_count = pieces.len();
    (2..=5).contains(&piece_count)
        && pieces
            .iter()
            .any(|(_, piece)| !piece.eq_ignore_ascii_case(&'p'))
}

fn generated_depth_two_row_within_node_budget(row: &DatasetLabelRow) -> bool {
    match &row.label {
        LabelPayload::Exact { exact, .. } => exact
            .value
            .get("component_recursive_total_nodes")
            .and_then(|nodes| nodes.parse::<usize>().ok())
            .is_some_and(|nodes| nodes <= GENERATED_DEPTH_TWO_MAX_RECURSIVE_NODES),
        _ => false,
    }
}

fn composition_digest_summary_for_row(
    row: &DatasetLabelRow,
) -> Option<(String, String, Vec<String>)> {
    let LabelPayload::Exact { provenance, .. } = &row.label else {
        return None;
    };
    let composition = provenance.certificate.composition.as_ref()?;
    let mut component_digests = composition
        .component_values
        .values()
        .cloned()
        .collect::<Vec<_>>();
    component_digests.sort();
    Some((
        composition.decomposition_digest.clone(),
        composition.result_value_digest.clone(),
        component_digests,
    ))
}

fn composition_identity_summary_for_row(
    row: &DatasetLabelRow,
) -> Option<CompositionIdentitySummary> {
    let LabelPayload::Exact { provenance, .. } = &row.label else {
        return None;
    };
    let composition = provenance.certificate.composition.as_ref()?;
    let mut component_identities = Vec::new();
    let mut component_value_digests = Vec::new();
    let mut component_value_identities = Vec::new();
    for (component_root, value_digest) in &composition.component_values {
        let component_identity = format!("{}:{component_root}", composition.decomposition_digest);
        component_identities.push(component_identity.clone());
        component_value_digests.push(value_digest.clone());
        component_value_identities.push(format!("{component_identity}={value_digest}"));
    }
    component_identities.sort();
    component_value_digests.sort();
    component_value_identities.sort();
    Some(CompositionIdentitySummary {
        decomposition_digest: composition.decomposition_digest.clone(),
        composition_digest: composition.composition_digest.clone(),
        component_identities,
        component_value_digests,
        component_value_identities,
        result_value_digest: composition.result_value_digest.clone(),
    })
}

fn has_duplicates(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() != values.len()
}

fn increment_count(counts: &mut BTreeMap<String, usize>, key: &str) {
    *counts.entry(key.to_owned()).or_default() += 1;
}

impl SampleLabelCandidate {
    fn to_row(self) -> DatasetLabelRow {
        match domain::validate_first_constrained_fen(self.fen) {
            Ok(validated) => exact_row(self.row_id, &validated, SAMPLE_EXACT_CONTEXT)
                .unwrap_or_else(|| {
                    DatasetLabelRow::rejected(
                        self.row_id,
                        FIRST_CONSTRAINED_DOMAIN_ID,
                        DatasetPosition::fen(self.fen),
                        RejectedLabel::unsupported(vec![
                            "exact_solver_terminal_only: exact generation is currently limited to legal terminal positions".to_owned(),
                        ]),
                    )
                }),
            Err(report) => DatasetLabelRow::rejected(
                self.row_id,
                FIRST_CONSTRAINED_DOMAIN_ID,
                DatasetPosition::fen(report.fen()),
                RejectedLabel::unsupported(report.reason_messages()),
            ),
        }
    }
}

fn generated_material_row(
    row_id: &str,
    fen: &str,
    context: ExactGenerationContext,
    family: &str,
) -> Option<DatasetLabelRow> {
    match domain::validate_first_constrained_fen(fen) {
        Ok(validated) => exact_row(row_id, &validated, context).or_else(|| {
            Some(DatasetLabelRow::rejected(
                row_id,
                FIRST_CONSTRAINED_DOMAIN_ID,
                DatasetPosition::fen(fen),
                RejectedLabel::unsupported(vec![format!(
                    "frontier_generator: legal {family} candidate has no terminal or one-ply terminal-frontier certificate"
                )]),
            ))
        }),
        Err(report) => {
            if report
                .reasons()
                .iter()
                .any(|reason| matches!(
                    reason.code,
                    domain::DomainRejectionCode::InvalidFen
                        | domain::DomainRejectionCode::InvalidPosition
                ))
            {
                return None;
            }

            Some(DatasetLabelRow::rejected(
                row_id,
                FIRST_CONSTRAINED_DOMAIN_ID,
                DatasetPosition::fen(report.fen()),
                RejectedLabel::unsupported(report.reason_messages()),
            ))
        }
    }
}

fn exact_row(
    row_id: &str,
    validated: &ValidatedDomainPosition,
    context: ExactGenerationContext,
) -> Option<DatasetLabelRow> {
    if let Some(terminal_status) = validated.terminal_status() {
        return Some(terminal_exact_row(
            row_id,
            validated,
            terminal_status,
            context,
        ));
    }

    validated
        .immediate_terminal_tactic()
        .map(|tactic| immediate_tactic_exact_row(row_id, validated, tactic, context))
}

fn composition_fixture_exact_row(spec: &CompositionFixtureSpec) -> DatasetLabelRow {
    let board = composition_fixture_board(spec);
    let decomposition = bitmesh::certify_decomposition(&board);
    assert_eq!(
        decomposition.status,
        DecompositionStatus::Strict,
        "composition fixture must have a strict decomposition certificate",
    );
    assert_eq!(
        decomposition.components.len(),
        spec.expected_component_count,
        "composition fixture emitted an unexpected component count",
    );
    let decomposition_digest = decomposition
        .digest()
        .expect("composition fixture decomposition must digest");

    let mut component_roots = decomposition
        .components
        .iter()
        .map(|component| component.root)
        .collect::<Vec<_>>();
    component_roots.sort();

    let mut component_values = BTreeMap::new();
    let mut bmcompose_component_values = Vec::new();
    let mut component_value_summaries = Vec::new();
    let mut result_integer = 0;
    for (index, component_root) in component_roots.iter().enumerate() {
        let component_integer = spec.value_offset
            + i32::try_from(index + 1).expect("composition fixture component index fits i32");
        result_integer += component_integer;
        let payload = CGTValue::Integer(component_integer).exact_value_payload();
        component_values.insert(component_root.to_string(), payload.digest.clone());
        bmcompose_component_values.push(CompositionComponentValue {
            component_root: *component_root,
            value_digest: payload.digest.clone(),
        });
        component_value_summaries.push(format!(
            "{component_root}={}",
            payload.canonical_serialization
        ));
    }

    let result_payload = CGTValue::Integer(result_integer).exact_value_payload();
    let bmcompose = BitmeshCompositionCertificate {
        decomposition_digest,
        component_values: bmcompose_component_values,
        result_value_digest: result_payload.digest.clone(),
    };
    bmcompose
        .validate_against_decomposition(&decomposition)
        .expect("composition fixture must satisfy root coverage");
    let composition_digest = bmcompose
        .digest()
        .expect("composition fixture must digest")
        .to_string();

    let mut exact = ExactLabel::from_thermograph_payload(&result_payload);
    exact.value.insert(
        "solver_scope".to_owned(),
        "composition_certificate_fixture".to_owned(),
    );
    exact.value.insert(
        "composition_value_rule".to_owned(),
        "component_index_integer_sum_fixture_v0".to_owned(),
    );
    exact.value.insert(
        "component_count".to_owned(),
        component_roots.len().to_string(),
    );
    exact.value.insert(
        "component_roots".to_owned(),
        component_roots
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(","),
    );
    exact.value.insert(
        "component_values".to_owned(),
        component_value_summaries.join(","),
    );
    exact.value.insert(
        "component_value_offset".to_owned(),
        spec.value_offset.to_string(),
    );

    DatasetLabelRow::exact(
        spec.row_id,
        COMPOSITION_FIXTURE_SHARD_CONFIG.domain_id,
        DatasetPosition::fen(composition_fixture_fen(spec)),
        exact,
        ExactProvenance {
            code_commit: std::env::var("ASTRALBASE_CODE_COMMIT")
                .unwrap_or_else(|_| "workspace".to_owned()),
            generator: COMPOSITION_FIXTURE_SHARD_CONFIG.generator.to_owned(),
            generator_config_hash: COMPOSITION_FIXTURE_SHARD_CONFIG
                .generator_config_hash
                .to_owned(),
            random_seed: 0,
            domain_definition: COMPOSITION_FIXTURE_SHARD_CONFIG
                .domain_definition
                .to_owned(),
            verifier: "bitmesh_bmcompose_fixture_sum_verifier".to_owned(),
            verifier_version: env!("CARGO_PKG_VERSION").to_owned(),
            certificate: LabelCertificate::composition(
                "bitmesh-bmcompose-v1+thermograph-exact-value+fixture-sum",
                format!(
                    "bitmesh:{};bmcompose:{};thermograph:{}",
                    decomposition_digest, composition_digest, result_payload.digest
                ),
                decomposition_digest.to_string(),
                composition_digest,
                component_values,
                result_payload.digest,
            ),
        },
    )
}

fn composition_fixture_board(spec: &CompositionFixtureSpec) -> Board {
    let mut board = Board::empty();

    for (square, piece) in composition_fixture_wall_pieces(spec.topology) {
        board.set_piece_at(square, composition_fixture_piece(piece));
    }

    board.set_piece_at(
        spec.bottom_square,
        composition_fixture_piece(spec.bottom_piece),
    );
    board.set_piece_at(spec.top_square, composition_fixture_piece(spec.top_piece));
    board
}

fn composition_fixture_fen(spec: &CompositionFixtureSpec) -> String {
    let board = board_fen(&composition_fixture_fen_pieces(spec));
    format!("{board} w - - 0 {}", spec.fullmove_number)
}

fn composition_fixture_fen_pieces(spec: &CompositionFixtureSpec) -> Vec<(usize, char)> {
    let mut pieces = Vec::with_capacity(18);
    for (square, piece) in composition_fixture_wall_pieces(spec.topology) {
        pieces.push((usize::from(square), piece));
    }
    pieces.push((usize::from(spec.bottom_square), spec.bottom_piece));
    pieces.push((usize::from(spec.top_square), spec.top_piece));
    pieces
}

fn composition_fixture_wall_pieces(topology: CompositionFixtureTopology) -> Vec<(Square, char)> {
    let mut pieces = Vec::new();
    match topology {
        CompositionFixtureTopology::SingleHorizontalWall => {
            for square in [
                Square::A4,
                Square::B4,
                Square::C4,
                Square::D4,
                Square::E4,
                Square::F4,
                Square::G4,
                Square::H4,
                Square::A5,
                Square::B5,
                Square::C5,
                Square::D5,
                Square::E5,
                Square::F5,
                Square::G5,
                Square::H5,
            ] {
                pieces.push((square, 'P'));
            }
        }
        CompositionFixtureTopology::DoubleHorizontalWall => {
            for square in [
                Square::A2,
                Square::B2,
                Square::C2,
                Square::D2,
                Square::E2,
                Square::F2,
                Square::G2,
                Square::H2,
                Square::A3,
                Square::B3,
                Square::C3,
                Square::D3,
                Square::E3,
                Square::F3,
                Square::G3,
                Square::H3,
            ] {
                pieces.push((square, 'P'));
            }
            for square in [
                Square::A4,
                Square::B4,
                Square::C4,
                Square::D4,
                Square::E4,
                Square::F4,
                Square::G4,
                Square::H4,
                Square::A5,
                Square::B5,
                Square::C5,
                Square::D5,
                Square::E5,
                Square::F5,
                Square::G5,
                Square::H5,
            ] {
                pieces.push((square, 'p'));
            }
        }
    }
    pieces
}

fn composition_fixture_piece(piece: char) -> shakmaty::Piece {
    match piece {
        'B' => Color::White.bishop(),
        'N' => Color::White.knight(),
        'P' => Color::White.pawn(),
        'Q' => Color::White.queen(),
        'R' => Color::White.rook(),
        'b' => Color::Black.bishop(),
        'n' => Color::Black.knight(),
        'p' => Color::Black.pawn(),
        'q' => Color::Black.queen(),
        'r' => Color::Black.rook(),
        _ => panic!("unsupported composition fixture piece: {piece}"),
    }
}

fn non_fixture_composed_board_exact_row(spec: &NonFixtureComposedBoardSpec<'_>) -> DatasetLabelRow {
    try_non_fixture_composed_board_exact_row(spec)
        .expect("non-fixture board composition spec must pass exact-row generation")
}

fn try_non_fixture_composed_board_exact_row(
    spec: &NonFixtureComposedBoardSpec<'_>,
) -> Result<DatasetLabelRow, String> {
    let board = non_fixture_composed_board(spec);
    let replay = replay_non_fixture_composed_board(&board, spec.value_rule)?;
    let mut exact = ExactLabel::verified(replay.exact_values.clone(), replay.exact_value_class);
    exact.value.insert(
        "component_topology_family".to_owned(),
        spec.topology_family.to_owned(),
    );
    exact.value.insert(
        "composition_spec_source".to_owned(),
        spec.spec_source.to_owned(),
    );

    Ok(DatasetLabelRow::exact(
        spec.row_id,
        NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG.domain_id,
        DatasetPosition::fen(non_fixture_composed_board_fen(spec)),
        exact,
        ExactProvenance {
            code_commit: std::env::var("ASTRALBASE_CODE_COMMIT")
                .unwrap_or_else(|_| "workspace".to_owned()),
            generator: NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG.generator.to_owned(),
            generator_config_hash: NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG
                .generator_config_hash
                .to_owned(),
            random_seed: 0,
            domain_definition: NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG
                .domain_definition
                .to_owned(),
            verifier: replay.verifier.to_owned(),
            verifier_version: env!("CARGO_PKG_VERSION").to_owned(),
            certificate: LabelCertificate::composition(
                replay.certificate_kind,
                replay.certificate_digest,
                replay.decomposition_digest,
                replay.composition_digest,
                replay.component_values,
                replay.result_value_digest,
            ),
        },
    ))
}

fn replay_verify_non_fixture_composed_board_exact_row(
    row: &DatasetLabelRow,
    exact: &ExactLabel,
    provenance: &ExactProvenance,
    issues: &mut Vec<LabelValidationIssue>,
) {
    if row.position.encoding != PositionEncoding::Fen {
        issues.push(LabelValidationIssue::row(format!(
            "{} replay requires fen position encoding",
            row.row_id
        )));
        return;
    }
    let Some(rule_text) = exact.value.get("composition_value_rule") else {
        issues.push(LabelValidationIssue::row(format!(
            "{} exact.value.composition_value_rule is required for replay",
            row.row_id
        )));
        return;
    };
    let Some(value_rule) =
        NonFixtureComposedBoardValueRule::from_composition_value_rule(rule_text.as_str())
    else {
        issues.push(LabelValidationIssue::row(format!(
            "{} unsupported composition_value_rule for replay: {rule_text}",
            row.row_id
        )));
        return;
    };
    let board = match board_from_fen_board_part(row.position.text.as_str()) {
        Ok(board) => board,
        Err(error) => {
            issues.push(LabelValidationIssue::row(format!(
                "{} replay could not parse FEN: {error}",
                row.row_id
            )));
            return;
        }
    };
    let replay = match replay_non_fixture_composed_board(&board, value_rule) {
        Ok(replay) => replay,
        Err(error) => {
            issues.push(LabelValidationIssue::row(format!(
                "{} replay recomputation failed: {error}",
                row.row_id
            )));
            return;
        }
    };

    compare_replay_field(
        row.row_id.as_str(),
        "exact.value_class",
        exact.value_class,
        replay.exact_value_class,
        issues,
    );
    for (key, expected) in &replay.exact_values {
        compare_replay_value(
            row.row_id.as_str(),
            format!("exact.value.{key}").as_str(),
            exact.value.get(key),
            expected,
            issues,
        );
    }
    for optional_key in [
        "dyadic_numerator",
        "dyadic_denominator_power",
        "solver_depth",
        "component_recursive_node_counts",
        "component_recursive_total_nodes",
        "recursive_leaf_rule",
    ] {
        if !replay.exact_values.contains_key(optional_key)
            && let Some(actual) = exact.value.get(optional_key)
        {
            issues.push(LabelValidationIssue::row(format!(
                "{} exact.value.{optional_key} should be absent under replay, got {actual:?}",
                row.row_id
            )));
        }
    }

    compare_replay_value(
        row.row_id.as_str(),
        "provenance.verifier",
        Some(&provenance.verifier),
        replay.verifier,
        issues,
    );
    compare_replay_value(
        row.row_id.as_str(),
        "provenance.certificate.kind",
        Some(&provenance.certificate.kind),
        replay.certificate_kind,
        issues,
    );
    compare_replay_value(
        row.row_id.as_str(),
        "provenance.certificate.digest",
        Some(&provenance.certificate.digest),
        &replay.certificate_digest,
        issues,
    );
    let Some(composition) = provenance.certificate.composition.as_deref() else {
        issues.push(LabelValidationIssue::row(format!(
            "{} replay requires structured composition certificate",
            row.row_id
        )));
        return;
    };
    compare_replay_value(
        row.row_id.as_str(),
        "provenance.certificate.decomposition_digest",
        Some(&composition.decomposition_digest),
        &replay.decomposition_digest,
        issues,
    );
    compare_replay_value(
        row.row_id.as_str(),
        "provenance.certificate.composition_digest",
        Some(&composition.composition_digest),
        &replay.composition_digest,
        issues,
    );
    compare_replay_value(
        row.row_id.as_str(),
        "provenance.certificate.result_value_digest",
        Some(&composition.result_value_digest),
        &replay.result_value_digest,
        issues,
    );
    if composition.component_values != replay.component_values {
        issues.push(LabelValidationIssue::row(format!(
            "{} provenance.certificate.component_values replay mismatch: expected {:?}, got {:?}",
            row.row_id, replay.component_values, composition.component_values
        )));
    }
}

fn replay_non_fixture_composed_board(
    board: &Board,
    value_rule: NonFixtureComposedBoardValueRule,
) -> Result<NonFixtureCompositionReplay, String> {
    let decomposition = bitmesh::certify_decomposition(board);
    let proof = bitmesh::verify_conservative_legal_independence(board, &decomposition)
        .map_err(|error| format!("conservative independence proof failed: {error:?}"))?;
    if proof.component_count != 2 {
        return Err(format!(
            "expected two independent components, got {}",
            proof.component_count
        ));
    }
    let decomposition_digest = proof.decomposition_digest;
    let decomposition_digest_text = decomposition_digest.to_string();

    let mut components = decomposition.components.iter().collect::<Vec<_>>();
    components.sort_by_key(|component| component.root);

    let mut component_values = BTreeMap::new();
    let mut bmcompose_component_values = Vec::new();
    let mut component_root_summaries = Vec::new();
    let mut component_value_summaries = Vec::new();
    let mut component_value_class_summaries = Vec::new();
    let mut component_material_summaries = Vec::new();
    let mut component_local_move_summaries = Vec::new();
    let mut component_signatures = Vec::new();
    let mut component_recursive_node_summaries = Vec::new();
    let mut total_white_local_moves = 0usize;
    let mut total_black_local_moves = 0usize;
    let mut total_recursive_nodes = 0usize;
    let mut component_cgt_values = Vec::new();
    let mut component_value_digests = Vec::new();

    for component in components {
        let material_value = component_material_balance(board, component.active_mask);
        let local_move_counts =
            component_local_move_counts(board, component.mask, component.active_mask);
        total_white_local_moves += local_move_counts.white;
        total_black_local_moves += local_move_counts.black;
        let component_evaluation = component_cgt_evaluation(
            value_rule,
            board,
            component.mask,
            component.active_mask,
            material_value,
            local_move_counts,
        );
        if let Some(recursive_nodes) = component_evaluation.recursive_nodes {
            total_recursive_nodes += recursive_nodes;
            component_recursive_node_summaries.push(format!(
                "{}=depth:{},nodes:{}",
                component.root,
                component_evaluation.recursive_depth.unwrap_or(0),
                recursive_nodes
            ));
        }
        let payload = component_evaluation.value.exact_value_payload();
        component_values.insert(component.root.to_string(), payload.digest.clone());
        component_value_digests.push(payload.digest.clone());
        component_signatures.push(generated_depth_two_component_signature_key(
            &payload.digest,
            material_value,
            local_move_counts,
        ));
        bmcompose_component_values.push(CompositionComponentValue {
            component_root: component.root,
            value_digest: payload.digest.clone(),
        });
        component_root_summaries.push(component.root.to_string());
        component_value_summaries.push(format!(
            "{}={}",
            component.root, payload.canonical_serialization
        ));
        component_value_class_summaries.push(format!(
            "{}={}",
            component.root,
            payload.value_class.as_str()
        ));
        component_material_summaries.push(format!("{}={material_value}", component.root));
        component_local_move_summaries.push(format!(
            "{}=white:{},black:{}",
            component.root, local_move_counts.white, local_move_counts.black
        ));
        component_cgt_values.push(component_evaluation.value);
    }

    let result_value = CGTValue::sum_all(&component_cgt_values);
    let result_payload = result_value.exact_value_payload();
    let bmcompose = BitmeshCompositionCertificate {
        decomposition_digest,
        component_values: bmcompose_component_values,
        result_value_digest: result_payload.digest.clone(),
    };
    bmcompose
        .validate_against_decomposition(&decomposition)
        .map_err(|error| format!("BMCOMPOSE root coverage failed: {error:?}"))?;
    let composition_digest = bmcompose
        .digest()
        .map_err(|error| format!("BMCOMPOSE digest failed: {error:?}"))?
        .to_string();

    let mut exact_values = thermograph_exact_value_map(&result_payload);
    exact_values.insert(
        "solver_scope".to_owned(),
        value_rule.solver_scope().to_owned(),
    );
    exact_values.insert(
        "composition_value_rule".to_owned(),
        value_rule.composition_value_rule().to_owned(),
    );
    exact_values.insert("proof_kind".to_owned(), proof.proof_kind.to_owned());
    exact_values.insert(
        "component_count".to_owned(),
        proof.component_count.to_string(),
    );
    exact_values.insert(
        "component_roots".to_owned(),
        component_root_summaries.join(","),
    );
    exact_values.insert(
        "component_values".to_owned(),
        component_value_summaries.join(","),
    );
    exact_values.insert(
        "component_value_classes".to_owned(),
        component_value_class_summaries.join(","),
    );
    exact_values.insert(
        "component_material_balances".to_owned(),
        component_material_summaries.join(","),
    );
    exact_values.insert(
        "component_local_move_counts".to_owned(),
        component_local_move_summaries.join(","),
    );
    exact_values.insert(
        "component_local_move_totals".to_owned(),
        format!("white:{total_white_local_moves},black:{total_black_local_moves}"),
    );
    exact_values.insert(
        "component_local_move_imbalance".to_owned(),
        (i64::try_from(total_white_local_moves).expect("local move count fits i64")
            - i64::try_from(total_black_local_moves).expect("local move count fits i64"))
        .to_string(),
    );
    if !component_recursive_node_summaries.is_empty() {
        exact_values.insert(
            "solver_depth".to_owned(),
            COMPONENT_DEPTH_TWO_LOCAL_MOVE_DEPTH.to_string(),
        );
        exact_values.insert(
            "component_recursive_node_counts".to_owned(),
            component_recursive_node_summaries.join(","),
        );
        exact_values.insert(
            "component_recursive_total_nodes".to_owned(),
            total_recursive_nodes.to_string(),
        );
        exact_values.insert(
            "recursive_leaf_rule".to_owned(),
            "component_material_balance_at_depth_cutoff_or_no_moves_v0".to_owned(),
        );
    }
    exact_values.insert(
        "result_digest_v1_sha256".to_owned(),
        result_value.digest_v1_sha256(),
    );

    Ok(NonFixtureCompositionReplay {
        exact_values,
        exact_value_class: result_payload.value_class.into(),
        verifier: value_rule.verifier(),
        certificate_kind: value_rule.certificate_kind(),
        certificate_digest: format!(
            "bitmesh:{};proof:{};rule:{};bmcompose:{};thermograph:{}",
            decomposition_digest_text,
            proof.proof_kind,
            value_rule.composition_value_rule(),
            composition_digest,
            result_payload.digest
        ),
        decomposition_digest: decomposition_digest_text,
        composition_digest,
        component_values,
        component_value_digests,
        component_signatures,
        result_value_digest: result_payload.digest,
    })
}

fn compare_replay_value(
    row_id: &str,
    field: &str,
    actual: Option<&String>,
    expected: &str,
    issues: &mut Vec<LabelValidationIssue>,
) {
    match actual {
        Some(actual) if actual == expected => {}
        Some(actual) => issues.push(LabelValidationIssue::row(format!(
            "{row_id} {field} replay mismatch: expected {expected:?}, got {actual:?}"
        ))),
        None => issues.push(LabelValidationIssue::row(format!(
            "{row_id} {field} missing under replay; expected {expected:?}"
        ))),
    }
}

fn compare_replay_field<T>(
    row_id: &str,
    field: &str,
    actual: T,
    expected: T,
    issues: &mut Vec<LabelValidationIssue>,
) where
    T: std::fmt::Debug + PartialEq,
{
    if actual != expected {
        issues.push(LabelValidationIssue::row(format!(
            "{row_id} {field} replay mismatch: expected {expected:?}, got {actual:?}"
        )));
    }
}

fn non_fixture_composed_board(spec: &NonFixtureComposedBoardSpec<'_>) -> Board {
    let mut board = Board::empty();
    for (square, piece) in non_fixture_composed_board_wall_pieces() {
        board.set_piece_at(square, composition_fixture_piece(piece));
    }
    for (square, piece) in spec.active_pieces {
        board.set_piece_at(*square, composition_fixture_piece(*piece));
    }
    board
}

fn non_fixture_composed_board_fen(spec: &NonFixtureComposedBoardSpec<'_>) -> String {
    let mut pieces = non_fixture_composed_board_wall_pieces()
        .into_iter()
        .map(|(square, piece)| (usize::from(square), piece))
        .collect::<Vec<_>>();
    pieces.extend(
        spec.active_pieces
            .iter()
            .map(|(square, piece)| (usize::from(*square), *piece)),
    );
    let board = board_fen(&pieces);
    format!("{board} w - - 0 {}", spec.fullmove_number)
}

fn non_fixture_composed_board_wall_pieces() -> Vec<(Square, char)> {
    vec![
        (Square::D1, 'P'),
        (Square::D2, 'p'),
        (Square::D3, 'P'),
        (Square::D4, 'p'),
        (Square::D5, 'P'),
        (Square::D6, 'p'),
        (Square::D7, 'P'),
        (Square::D8, 'p'),
    ]
}

fn non_fixture_composed_board_active_piece_summary_from_board(board: &Board) -> String {
    let wall_squares = non_fixture_composed_board_wall_pieces()
        .into_iter()
        .map(|(square, _piece)| square)
        .collect::<BTreeSet<_>>();
    let mut active_pieces = Vec::new();
    for square in board.occupied() {
        if wall_squares.contains(&square) {
            continue;
        }
        let Some(piece) = board.piece_at(square) else {
            continue;
        };
        let Some(piece_char) = composition_piece_char(piece) else {
            continue;
        };
        active_pieces.push((square, piece_char));
    }
    active_pieces.sort_by_key(|(square, piece)| (usize::from(*square), *piece));
    generated_depth_two_active_piece_summary(&active_pieces)
}

fn composition_piece_char(piece: shakmaty::Piece) -> Option<char> {
    let role_char = match piece.role {
        shakmaty::Role::Bishop => 'B',
        shakmaty::Role::Knight => 'N',
        shakmaty::Role::Pawn => 'P',
        shakmaty::Role::Queen => 'Q',
        shakmaty::Role::Rook => 'R',
        shakmaty::Role::King => return None,
    };
    Some(if piece.color == Color::White {
        role_char
    } else {
        role_char.to_ascii_lowercase()
    })
}

fn component_material_balance(board: &Board, active_mask: shakmaty::Bitboard) -> i32 {
    active_mask
        .into_iter()
        .map(|square| {
            let piece = board
                .piece_at(square)
                .expect("component active mask square must contain a piece");
            let sign = if piece.color == Color::White { 1 } else { -1 };
            sign * material_value(piece.role)
        })
        .sum()
}

fn component_cgt_evaluation(
    value_rule: NonFixtureComposedBoardValueRule,
    board: &Board,
    component_mask: shakmaty::Bitboard,
    active_mask: shakmaty::Bitboard,
    material_balance: i32,
    local_move_counts: ComponentLocalMoveCounts,
) -> ComponentValueEvaluation {
    let value = match value_rule {
        NonFixtureComposedBoardValueRule::MaterialBalanceSum => CGTValue::Integer(material_balance),
        NonFixtureComposedBoardValueRule::AgencyAtomSum => {
            component_agency_atom_value(material_balance)
        }
        NonFixtureComposedBoardValueRule::LocalMoveGame => {
            component_local_move_game_value(local_move_counts)
        }
        NonFixtureComposedBoardValueRule::DepthTwoLocalMoveGame => {
            let (value, nodes) = component_recursive_local_move_game_value(
                board,
                component_mask,
                active_mask,
                COMPONENT_DEPTH_TWO_LOCAL_MOVE_DEPTH,
            );
            return ComponentValueEvaluation {
                value,
                recursive_depth: Some(COMPONENT_DEPTH_TWO_LOCAL_MOVE_DEPTH),
                recursive_nodes: Some(nodes),
            };
        }
    };

    ComponentValueEvaluation {
        value,
        recursive_depth: None,
        recursive_nodes: None,
    }
}

fn component_agency_atom_value(material_balance: i32) -> CGTValue {
    match material_balance.cmp(&0) {
        std::cmp::Ordering::Greater => CGTValue::Up,
        std::cmp::Ordering::Equal => CGTValue::Star,
        std::cmp::Ordering::Less => CGTValue::Down,
    }
}

fn component_local_move_game_value(local_move_counts: ComponentLocalMoveCounts) -> CGTValue {
    let left = if local_move_counts.white == 0 {
        Vec::new()
    } else {
        vec![CGTValue::Integer(
            i32::try_from(local_move_counts.white).expect("component move count fits i32"),
        )]
    };
    let right = if local_move_counts.black == 0 {
        Vec::new()
    } else {
        vec![CGTValue::Integer(
            -i32::try_from(local_move_counts.black).expect("component move count fits i32"),
        )]
    };
    CGTValue::GameTree { left, right }
}

fn component_recursive_local_move_game_value(
    board: &Board,
    component_mask: shakmaty::Bitboard,
    active_mask: shakmaty::Bitboard,
    depth: u8,
) -> (CGTValue, usize) {
    let material_value = component_material_balance(board, active_mask);
    if depth == 0 {
        return (CGTValue::Integer(material_value), 1);
    }

    let left_moves = component_local_moves(board, component_mask, active_mask, Color::White);
    let right_moves = component_local_moves(board, component_mask, active_mask, Color::Black);
    if left_moves.is_empty() && right_moves.is_empty() {
        return (CGTValue::Integer(material_value), 1);
    }

    let mut node_count = 1;
    let mut left = Vec::with_capacity(left_moves.len());
    for local_move in left_moves {
        let (child_board, child_active_mask) =
            apply_component_local_move(board, active_mask, local_move);
        let (child_value, child_nodes) = component_recursive_local_move_game_value(
            &child_board,
            component_mask,
            child_active_mask,
            depth - 1,
        );
        node_count += child_nodes;
        left.push(child_value);
    }

    let mut right = Vec::with_capacity(right_moves.len());
    for local_move in right_moves {
        let (child_board, child_active_mask) =
            apply_component_local_move(board, active_mask, local_move);
        let (child_value, child_nodes) = component_recursive_local_move_game_value(
            &child_board,
            component_mask,
            child_active_mask,
            depth - 1,
        );
        node_count += child_nodes;
        right.push(child_value);
    }

    (CGTValue::GameTree { left, right }, node_count)
}

fn component_local_move_counts(
    board: &Board,
    component_mask: shakmaty::Bitboard,
    active_mask: shakmaty::Bitboard,
) -> ComponentLocalMoveCounts {
    ComponentLocalMoveCounts {
        white: component_local_move_count(board, component_mask, active_mask, Color::White),
        black: component_local_move_count(board, component_mask, active_mask, Color::Black),
    }
}

fn component_local_move_count(
    board: &Board,
    component_mask: shakmaty::Bitboard,
    active_mask: shakmaty::Bitboard,
    color: Color,
) -> usize {
    let occupied = board.occupied();
    let mut count = 0;

    for from in active_mask {
        let piece = board
            .piece_at(from)
            .expect("component active mask square must contain a piece");
        if piece.color != color {
            continue;
        }

        if piece.role == shakmaty::Role::Pawn {
            count += (shakmaty::attacks::pawn_attacks(color, from)
                & board.by_color(!color)
                & component_mask)
                .count();
            count += component_local_pawn_quiet_move_count(board, component_mask, from, color);
        } else {
            count += (shakmaty::attacks::attacks(from, piece, occupied)
                & !board.by_color(color)
                & component_mask)
                .count();
        }
    }

    count
}

fn component_local_moves(
    board: &Board,
    component_mask: shakmaty::Bitboard,
    active_mask: shakmaty::Bitboard,
    color: Color,
) -> Vec<ComponentLocalMove> {
    let mut moves = Vec::new();
    for from in active_mask {
        let piece = board
            .piece_at(from)
            .expect("component active mask square must contain a piece");
        if piece.color != color {
            continue;
        }

        if piece.role == shakmaty::Role::Pawn {
            moves.extend(
                (shakmaty::attacks::pawn_attacks(color, from)
                    & board.by_color(!color)
                    & component_mask)
                    .into_iter()
                    .map(|to| ComponentLocalMove { from, to }),
            );
            moves.extend(component_local_pawn_quiet_moves(
                board,
                component_mask,
                from,
                color,
            ));
        } else {
            moves.extend(
                (shakmaty::attacks::attacks(from, piece, board.occupied())
                    & !board.by_color(color)
                    & component_mask)
                    .into_iter()
                    .map(|to| ComponentLocalMove { from, to }),
            );
        }
    }

    moves.sort_by_key(|local_move| (usize::from(local_move.from), usize::from(local_move.to)));
    moves
}

fn component_local_pawn_quiet_move_count(
    board: &Board,
    component_mask: shakmaty::Bitboard,
    from: Square,
    color: Color,
) -> usize {
    let occupied = board.occupied();
    let forward_offset = if color == Color::White { 8 } else { -8 };
    let start_rank = if color == Color::White {
        shakmaty::Rank::Second
    } else {
        shakmaty::Rank::Seventh
    };

    let Some(one_step) = from.offset(forward_offset) else {
        return 0;
    };
    if occupied.contains(one_step) || !component_mask.contains(one_step) {
        return 0;
    }

    let mut count = 1;
    if from.rank() == start_rank
        && let Some(two_step) = one_step.offset(forward_offset)
        && !occupied.contains(two_step)
        && component_mask.contains(two_step)
    {
        count += 1;
    }
    count
}

fn component_local_pawn_quiet_moves(
    board: &Board,
    component_mask: shakmaty::Bitboard,
    from: Square,
    color: Color,
) -> Vec<ComponentLocalMove> {
    let occupied = board.occupied();
    let forward_offset = if color == Color::White { 8 } else { -8 };
    let start_rank = if color == Color::White {
        shakmaty::Rank::Second
    } else {
        shakmaty::Rank::Seventh
    };

    let mut moves = Vec::with_capacity(2);
    let Some(one_step) = from.offset(forward_offset) else {
        return moves;
    };
    if occupied.contains(one_step) || !component_mask.contains(one_step) {
        return moves;
    }

    moves.push(ComponentLocalMove { from, to: one_step });
    if from.rank() == start_rank
        && let Some(two_step) = one_step.offset(forward_offset)
        && !occupied.contains(two_step)
        && component_mask.contains(two_step)
    {
        moves.push(ComponentLocalMove { from, to: two_step });
    }
    moves
}

fn apply_component_local_move(
    board: &Board,
    active_mask: shakmaty::Bitboard,
    local_move: ComponentLocalMove,
) -> (Board, shakmaty::Bitboard) {
    let mut child = board.clone();
    let piece = child
        .remove_piece_at(local_move.from)
        .expect("component local move origin must contain a piece");
    child.discard_piece_at(local_move.to);
    child.set_piece_at(local_move.to, piece);

    let mut child_active_mask = active_mask;
    child_active_mask.discard(local_move.from);
    child_active_mask.discard(local_move.to);
    child_active_mask.add(local_move.to);
    (child, child_active_mask)
}

fn material_value(role: shakmaty::Role) -> i32 {
    match role {
        shakmaty::Role::Pawn => 1,
        shakmaty::Role::Knight | shakmaty::Role::Bishop => 3,
        shakmaty::Role::Rook => 5,
        shakmaty::Role::Queen => 9,
        shakmaty::Role::King => 0,
    }
}

fn composition_fixture_rejected_row(row_id: &str, fen: &str, reason: &str) -> DatasetLabelRow {
    DatasetLabelRow::rejected(
        row_id,
        COMPOSITION_FIXTURE_SHARD_CONFIG.domain_id,
        DatasetPosition::fen(fen),
        RejectedLabel::excluded(vec![reason.to_owned()]),
    )
}

fn non_fixture_composed_domain_rejected_row(
    index: usize,
    candidate: &NonFixtureComposedDomainCandidate,
) -> DatasetLabelRow {
    let reason = non_fixture_composed_domain_rejection_reason(candidate.fen);
    DatasetLabelRow::rejected(
        format!(
            "{}-{index:03}",
            NON_FIXTURE_COMPOSED_DOMAIN_SHARD_CONFIG.row_id_prefix
        ),
        NON_FIXTURE_COMPOSED_DOMAIN_SHARD_CONFIG.domain_id,
        DatasetPosition::fen(candidate.fen),
        RejectedLabel::unsupported(vec![reason]),
    )
}

fn non_fixture_composed_domain_rejection_reason(fen: &str) -> String {
    let board = match board_from_fen(fen) {
        Ok(board) => board,
        Err(error) => {
            return format!(
                "unsupported_non_fixture_composition: invalid FEN for conservative legal-independence proof: {error}"
            );
        }
    };

    let decomposition = bitmesh::certify_decomposition(&board);
    match bitmesh::verify_conservative_legal_independence(&board, &decomposition) {
        Ok(proof) => format!(
            "unsupported_non_fixture_composition: conservative legal-independence proof {} with {} components is available, but exact component solving and BMCOMPOSE promotion are not wired yet",
            proof.proof_kind, proof.component_count
        ),
        Err(error) => format!(
            "unsupported_non_fixture_composition: conservative legal-independence proof rejected: {error:?}"
        ),
    }
}

fn board_from_fen(fen: &str) -> Result<Board, String> {
    let parsed = Fen::from_str(fen).map_err(|error| error.to_string())?;
    let position: Chess = parsed
        .into_position(CastlingMode::Standard)
        .map_err(|error| error.to_string())?;
    Ok(position.board().clone())
}

fn board_from_fen_board_part(fen: &str) -> Result<Board, String> {
    let board_part = fen
        .split_whitespace()
        .next()
        .ok_or_else(|| "missing board FEN field".to_owned())?;
    Board::from_str(board_part).map_err(|error| error.to_string())
}

fn terminal_exact_row(
    row_id: &str,
    validated: &ValidatedDomainPosition,
    terminal_status: TerminalStatus,
    context: ExactGenerationContext,
) -> DatasetLabelRow {
    let value = terminal_value(terminal_status);
    let payload = value.exact_value_payload();
    let mut exact = ExactLabel::from_thermograph_payload(&payload);
    exact
        .value
        .insert("solver_scope".to_owned(), "terminal_position".to_owned());
    exact.value.insert(
        "terminal_status".to_owned(),
        terminal_status.as_str().to_owned(),
    );
    exact
        .value
        .insert("legal_move_count".to_owned(), "0".to_owned());

    DatasetLabelRow::exact(
        row_id,
        FIRST_CONSTRAINED_DOMAIN_ID,
        DatasetPosition::fen(validated.fen()),
        exact,
        ExactProvenance {
            code_commit: std::env::var("ASTRALBASE_CODE_COMMIT")
                .unwrap_or_else(|_| "workspace".to_owned()),
            generator: context.generator.to_owned(),
            generator_config_hash: context.terminal_config_hash.to_owned(),
            random_seed: 0,
            domain_definition: FIRST_CONSTRAINED_DOMAIN_DEFINITION.to_owned(),
            verifier: "astralbase_terminal_exact_solver".to_owned(),
            verifier_version: env!("CARGO_PKG_VERSION").to_owned(),
            certificate: LabelCertificate::legacy(
                "terminal-legal-move-enumeration+thermograph-exact-value+bitmesh-domain-gate",
                format!(
                    "bitmesh:{};thermograph:{}",
                    validated.decomposition().digest.as_str(),
                    payload.digest
                ),
            ),
        },
    )
}

fn immediate_tactic_exact_row(
    row_id: &str,
    validated: &ValidatedDomainPosition,
    tactic: &ImmediateTerminalTactic,
    context: ExactGenerationContext,
) -> DatasetLabelRow {
    let value = CGTValue::Integer(1);
    let payload = value.exact_value_payload();
    let frontier_value = terminal_frontier_value(tactic);
    let frontier_payload = frontier_value.exact_value_payload();
    let (frontier_temperature, frontier_mean) = frontier_value.thermograph();
    let mut exact = ExactLabel::from_thermograph_payload(&payload);
    let moves = tactic.checkmating_moves().join(",");
    let stalemating_moves = join_or_none(tactic.stalemating_moves());
    let terminal_child_statuses = terminal_child_statuses(tactic).join(",");
    let terminal_child_values = terminal_child_values(tactic).join(",");
    exact.value.insert(
        "solver_scope".to_owned(),
        "immediate_terminal_frontier".to_owned(),
    );
    exact
        .value
        .insert("terminal_distance_plies".to_owned(), "1".to_owned());
    exact
        .value
        .insert("terminal_frontier_depth_plies".to_owned(), "1".to_owned());
    exact.value.insert(
        "legal_move_count".to_owned(),
        tactic.legal_move_count().to_string(),
    );
    exact.value.insert(
        "terminal_child_count".to_owned(),
        tactic.terminal_child_count().to_string(),
    );
    exact.value.insert(
        "checkmating_move_count".to_owned(),
        tactic.checkmating_move_count().to_string(),
    );
    exact
        .value
        .insert("checkmating_moves".to_owned(), moves.clone());
    exact.value.insert(
        "stalemating_move_count".to_owned(),
        tactic.stalemating_move_count().to_string(),
    );
    exact
        .value
        .insert("stalemating_moves".to_owned(), stalemating_moves);
    exact.value.insert(
        "terminal_child_statuses".to_owned(),
        terminal_child_statuses.clone(),
    );
    exact
        .value
        .insert("terminal_child_values".to_owned(), terminal_child_values);
    exact.value.insert(
        "frontier_value_class".to_owned(),
        frontier_payload.value_class.as_str().to_owned(),
    );
    exact.value.insert(
        "frontier_canonical_serialization".to_owned(),
        frontier_payload.canonical_serialization.clone(),
    );
    exact.value.insert(
        "frontier_digest".to_owned(),
        frontier_payload.digest.clone(),
    );
    exact
        .value
        .insert("frontier_mean".to_owned(), frontier_mean.to_string());
    exact.value.insert(
        "frontier_temperature".to_owned(),
        frontier_temperature.to_string(),
    );
    exact.value.insert(
        "frontier_perspective".to_owned(),
        "parent_side_to_move".to_owned(),
    );

    let tactic_digest = stable_digest_hex(
        format!(
            "{}|{}|{}|{}|{}",
            validated.fen(),
            tactic.legal_move_count(),
            moves,
            terminal_child_statuses,
            frontier_payload.digest
        )
        .as_bytes(),
    );

    DatasetLabelRow::exact(
        row_id,
        FIRST_CONSTRAINED_DOMAIN_ID,
        DatasetPosition::fen(validated.fen()),
        exact,
        ExactProvenance {
            code_commit: std::env::var("ASTRALBASE_CODE_COMMIT")
                .unwrap_or_else(|_| "workspace".to_owned()),
            generator: context.generator.to_owned(),
            generator_config_hash: context.frontier_config_hash.to_owned(),
            random_seed: 0,
            domain_definition: FIRST_CONSTRAINED_DOMAIN_DEFINITION.to_owned(),
            verifier: "astralbase_immediate_terminal_tactic_solver".to_owned(),
            verifier_version: env!("CARGO_PKG_VERSION").to_owned(),
            certificate: LabelCertificate::legacy(
                "immediate-checkmate-enumeration+thermograph-exact-value+bitmesh-domain-gate",
                format!(
                    "bitmesh:{};thermograph:{};frontier:{};frontier_thermograph:{}",
                    validated.decomposition().digest.as_str(),
                    payload.digest,
                    tactic_digest,
                    frontier_payload.digest
                ),
            ),
        },
    )
}

fn formal_switch_exact_row(row_id: &str) -> DatasetLabelRow {
    let value = CGTValue::GameTree {
        left: vec![CGTValue::Integer(1)],
        right: vec![CGTValue::Integer(-1)],
    };
    let payload = value.exact_value_payload();
    let (temperature, mean) = value.thermograph();
    let mut exact = ExactLabel::from_thermograph_payload(&payload);
    exact.value.insert(
        "solver_scope".to_owned(),
        "formal_cgt_switch_fixture".to_owned(),
    );
    exact
        .value
        .insert("left_options".to_owned(), "Number(1/2^0)".to_owned());
    exact
        .value
        .insert("right_options".to_owned(), "Number(-1/2^0)".to_owned());
    exact.value.insert("mean".to_owned(), mean.to_string());
    exact
        .value
        .insert("temperature".to_owned(), temperature.to_string());
    exact
        .value
        .insert("dyadic_payload".to_owned(), "none".to_owned());

    DatasetLabelRow::exact(
        row_id,
        FORMAL_CGT_DOMAIN_ID,
        DatasetPosition::cgt_canonical(payload.canonical_serialization.clone()),
        exact,
        ExactProvenance {
            code_commit: std::env::var("ASTRALBASE_CODE_COMMIT")
                .unwrap_or_else(|_| "workspace".to_owned()),
            generator: "astralbase_formal_cgt_fixture_generator".to_owned(),
            generator_config_hash: "astralbase:formal_cgt_switch_fixture:v1".to_owned(),
            random_seed: 0,
            domain_definition: FORMAL_CGT_DOMAIN_DEFINITION.to_owned(),
            verifier: "thermograph_switch_fixture_verifier".to_owned(),
            verifier_version: env!("CARGO_PKG_VERSION").to_owned(),
            certificate: LabelCertificate::legacy(
                "thermograph-canonical-switch-fixture",
                format!("thermograph:{}", payload.digest),
            ),
        },
    )
}

fn terminal_frontier_value(tactic: &ImmediateTerminalTactic) -> CGTValue {
    let mut left = Vec::new();
    for _ in tactic.checkmating_moves() {
        left.push(CGTValue::Integer(1));
    }
    for _ in tactic.stalemating_moves() {
        left.push(CGTValue::Integer(0));
    }

    CGTValue::GameTree {
        left,
        right: Vec::new(),
    }
}

fn terminal_child_statuses(tactic: &ImmediateTerminalTactic) -> Vec<String> {
    let mut statuses = Vec::new();
    statuses.extend(
        tactic
            .checkmating_moves()
            .iter()
            .map(|mv| format!("{mv}=checkmate")),
    );
    statuses.extend(
        tactic
            .stalemating_moves()
            .iter()
            .map(|mv| format!("{mv}=stalemate")),
    );
    statuses.sort();
    statuses
}

fn terminal_child_values(tactic: &ImmediateTerminalTactic) -> Vec<String> {
    let mut values = Vec::new();
    values.extend(
        tactic
            .checkmating_moves()
            .iter()
            .map(|mv| format!("{mv}=Number(1/2^0)")),
    );
    values.extend(
        tactic
            .stalemating_moves()
            .iter()
            .map(|mv| format!("{mv}=Number(0/2^0)")),
    );
    values.sort();
    values
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
}

fn terminal_value(terminal_status: TerminalStatus) -> CGTValue {
    match terminal_status {
        TerminalStatus::Checkmate => CGTValue::Integer(-1),
        TerminalStatus::Stalemate => CGTValue::Integer(0),
    }
}

fn thermograph_exact_value_map(payload: &ThermographExactValuePayload) -> BTreeMap<String, String> {
    let mut value = BTreeMap::new();
    value.insert(
        "value_class".to_owned(),
        payload.value_class.as_str().to_owned(),
    );
    value.insert(
        "canonical_serialization".to_owned(),
        payload.canonical_serialization.clone(),
    );
    value.insert("digest".to_owned(), payload.digest.clone());

    if let Some(dyadic) = payload.dyadic {
        value.insert(
            "dyadic_numerator".to_owned(),
            dyadic.numerator().to_string(),
        );
        value.insert(
            "dyadic_denominator_power".to_owned(),
            dyadic.denominator_power().to_string(),
        );
    }

    value
}

fn stable_digest_hex(bytes: &[u8]) -> String {
    const FNV64_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV64_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV64_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV64_PRIME);
    }
    format!("{hash:016x}")
}

fn validate_raw_payload_shape(value: &Value) -> Vec<String> {
    let mut issues = Vec::new();
    let Some(object) = value.as_object() else {
        return vec!["row must be a JSON object".to_owned()];
    };

    for field in COMMON_REQUIRED_FIELDS {
        if !object.contains_key(field) {
            issues.push(format!("row missing required field {field}"));
        }
    }

    for field in AMBIGUOUS_TOP_LEVEL_FIELDS {
        if object.contains_key(field) {
            issues.push(format!(
                "top-level {field} is ambiguous; use the label_kind payload object"
            ));
        }
    }

    let label_kind = object.get("label_kind").and_then(Value::as_str);
    if let Some(label_kind) = label_kind
        && !LABEL_PAYLOAD_KEYS.contains(&label_kind)
    {
        issues.push(format!(
            "label_kind must be one of {}",
            LABEL_PAYLOAD_KEYS.join(", ")
        ));
    }

    let payload_keys = LABEL_PAYLOAD_KEYS
        .iter()
        .filter(|key| object.contains_key(**key))
        .copied()
        .collect::<Vec<_>>();

    if payload_keys.len() != 1 {
        issues.push(format!(
            "row must contain exactly one label payload object: {}",
            LABEL_PAYLOAD_KEYS.join(", ")
        ));
    } else if let Some(label_kind) = label_kind
        && payload_keys[0] != label_kind
    {
        issues.push(format!(
            "label_kind {label_kind:?} does not match payload {:?}",
            payload_keys[0]
        ));
    }

    if let Some(position) = object.get("position") {
        if let Some(position) = position.as_object() {
            for field in ["encoding", "text"] {
                if !position.contains_key(field) {
                    issues.push(format!("position missing required field {field}"));
                }
            }
        } else {
            issues.push("position must be an object".to_owned());
        }
    }

    issues
}

fn validate_exact_provenance(
    exact: &ExactLabel,
    provenance: &ExactProvenance,
    issues: &mut Vec<LabelValidationIssue>,
) {
    require_non_empty(
        provenance.code_commit.as_str(),
        "provenance.code_commit",
        issues,
    );
    require_non_empty(
        provenance.generator.as_str(),
        "provenance.generator",
        issues,
    );
    require_non_empty(
        provenance.generator_config_hash.as_str(),
        "provenance.generator_config_hash",
        issues,
    );
    require_non_empty(
        provenance.domain_definition.as_str(),
        "provenance.domain_definition",
        issues,
    );
    require_non_empty(provenance.verifier.as_str(), "provenance.verifier", issues);
    require_non_empty(
        provenance.verifier_version.as_str(),
        "provenance.verifier_version",
        issues,
    );
    validate_label_certificate(&provenance.certificate, exact, issues);
}

fn validate_label_certificate(
    certificate: &LabelCertificate,
    exact: &ExactLabel,
    issues: &mut Vec<LabelValidationIssue>,
) {
    require_non_empty(
        certificate.kind.as_str(),
        "provenance.certificate.kind",
        issues,
    );
    require_non_empty(
        certificate.digest.as_str(),
        "provenance.certificate.digest",
        issues,
    );

    let Some(composition) = certificate.composition.as_deref() else {
        return;
    };

    require_non_empty(
        composition.decomposition_digest.as_str(),
        "provenance.certificate.decomposition_digest",
        issues,
    );
    require_non_empty(
        composition.composition_digest.as_str(),
        "provenance.certificate.composition_digest",
        issues,
    );
    require_non_empty_map(
        &composition.component_values,
        "provenance.certificate.component_values",
        issues,
    );
    require_non_empty(
        composition.result_value_digest.as_str(),
        "provenance.certificate.result_value_digest",
        issues,
    );
    match exact.value.get("digest") {
        Some(exact_digest)
            if !exact_digest.trim().is_empty()
                && !composition.result_value_digest.trim().is_empty()
                && composition.result_value_digest != *exact_digest =>
        {
            issues.push(LabelValidationIssue::row(
                "provenance.certificate.result_value_digest must equal exact.value.digest for composition rows",
            ));
        }
        Some(_) => {}
        None => issues.push(LabelValidationIssue::row(
            "exact.value.digest must be present for composition rows",
        )),
    }
}

fn is_composition_domain(domain: &str) -> bool {
    domain == COMPOSITION_FIXTURE_DOMAIN_ID
        || domain == NON_FIXTURE_COMPOSED_DOMAIN_ID
        || domain == NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID
}

fn require_non_empty(value: &str, field: &'static str, issues: &mut Vec<LabelValidationIssue>) {
    if value.trim().is_empty() {
        issues.push(LabelValidationIssue::row(format!(
            "{field} must be non-empty"
        )));
    }
}

fn require_non_empty_map(
    values: &BTreeMap<String, String>,
    field: &'static str,
    issues: &mut Vec<LabelValidationIssue>,
) {
    if values.is_empty() {
        issues.push(LabelValidationIssue::row(format!(
            "{field} must be non-empty"
        )));
        return;
    }

    for (key, value) in values {
        require_non_empty(key.as_str(), field, issues);
        require_non_empty(value.as_str(), field, issues);
    }
}

#[cfg(test)]
mod tests {
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
            composition_hard_target_shard_jsonl(DEFAULT_COMPOSITION_HARD_TARGET_SHARD_LIMIT)
                .unwrap(),
            composition_hard_target_shard_jsonl(DEFAULT_COMPOSITION_HARD_TARGET_SHARD_LIMIT)
                .unwrap()
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
                    assert!(provenance.certificate.composition.as_ref().is_some_and(
                        |composition| {
                            composition.result_value_digest == *exact.value.get("digest").unwrap()
                        }
                    ));
                }
                LabelPayload::Rejected { rejected } => {
                    assert_eq!(non_fixture_row.domain, NON_FIXTURE_COMPOSED_DOMAIN_ID);
                    assert_eq!(rejected.status, RejectedStatus::Unsupported);
                    assert!(rejected.reasons.iter().any(|reason| {
                        reason.starts_with("unsupported_non_fixture_composition:")
                    }));
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
}
