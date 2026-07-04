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
use std::{collections::BTreeMap, str::FromStr};
use thermograph::{CGTValue, ExactValuePayload as ThermographExactValuePayload};

pub const DATASET_LABEL_SCHEMA_VERSION: &str = "partizan.dataset_label.v0";
pub const FORMAL_CGT_DOMAIN_ID: &str = "formal_domain:thermograph_golden_cgt:v0";
pub const FORMAL_CGT_DOMAIN_DEFINITION: &str = "thermograph:golden_values#hot_one_minus_one";
pub const DEFAULT_FRONTIER_SHARD_LIMIT: usize = 1_000;
pub const DEFAULT_FAMILY_FRONTIER_LIMIT_PER_FAMILY: usize = 1_000;
pub const DEFAULT_EXPANDED_FAMILY_FRONTIER_LIMIT_PER_FAMILY: usize = 1_000;
pub const DEFAULT_COMPOSITION_HARD_TARGET_SHARD_LIMIT: usize = 21;
pub const DEFAULT_NON_FIXTURE_COMPOSED_DOMAIN_SHARD_LIMIT: usize = 9;
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
        generator_config_hash: "astralbase:non_fixture_composed_domain:v1",
        row_id_prefix: "astralbase-w18-non-fixture-composed-domain",
    };

pub const NON_FIXTURE_COMPOSED_BOARD_SHARD_CONFIG: CompositionShardConfig =
    CompositionShardConfig {
        kind: CompositionShardKind::NonFixtureComposedDomain,
        shard_name: "non_fixture_composed_board_material",
        domain_id: NON_FIXTURE_COMPOSED_BOARD_DOMAIN_ID,
        domain_definition: NON_FIXTURE_COMPOSED_BOARD_DOMAIN_DEFINITION,
        generator: "astralbase_non_fixture_composed_board_material_generator",
        generator_config_hash: "astralbase:non_fixture_composed_board_material:v0",
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
    rows.truncate(limit);
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
struct NonFixtureComposedBoardSpec {
    row_id: &'static str,
    active_pieces: &'static [(Square, char)],
    fullmove_number: u32,
    value_rule: NonFixtureComposedBoardValueRule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NonFixtureComposedBoardValueRule {
    MaterialBalanceSum,
    AgencyAtomSum,
    LocalMoveGame,
    DepthTwoLocalMoveGame,
}

impl NonFixtureComposedBoardValueRule {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ComponentLocalMove {
    from: Square,
    to: Square,
}

const COMPONENT_DEPTH_TWO_LOCAL_MOVE_DEPTH: u8 = 2;

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

const NON_FIXTURE_COMPOSED_BOARD_EXACT_SPECS: [NonFixtureComposedBoardSpec; 6] = [
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-001",
        active_pieces: &[(Square::A1, 'N'), (Square::H8, 'n'), (Square::G7, 'p')],
        fullmove_number: 101,
        value_rule: NonFixtureComposedBoardValueRule::MaterialBalanceSum,
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
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-003",
        active_pieces: &[(Square::A2, 'n'), (Square::H7, 'N'), (Square::G6, 'P')],
        fullmove_number: 103,
        value_rule: NonFixtureComposedBoardValueRule::MaterialBalanceSum,
    },
    NonFixtureComposedBoardSpec {
        row_id: "astralbase-w18-non-fixture-composed-board-004",
        active_pieces: &[(Square::A1, 'N'), (Square::A2, 'P'), (Square::H8, 'n')],
        fullmove_number: 104,
        value_rule: NonFixtureComposedBoardValueRule::AgencyAtomSum,
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
    },
];

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

fn non_fixture_composed_board_exact_row(spec: &NonFixtureComposedBoardSpec) -> DatasetLabelRow {
    let board = non_fixture_composed_board(spec);
    let decomposition = bitmesh::certify_decomposition(&board);
    let proof = bitmesh::verify_conservative_legal_independence(&board, &decomposition)
        .expect("non-fixture board composition spec must pass conservative independence proof");
    let decomposition_digest = proof.decomposition_digest;

    let mut components = decomposition.components.iter().collect::<Vec<_>>();
    components.sort_by_key(|component| component.root);

    let mut component_values = BTreeMap::new();
    let mut bmcompose_component_values = Vec::new();
    let mut component_root_summaries = Vec::new();
    let mut component_value_summaries = Vec::new();
    let mut component_value_class_summaries = Vec::new();
    let mut component_material_summaries = Vec::new();
    let mut component_local_move_summaries = Vec::new();
    let mut component_recursive_node_summaries = Vec::new();
    let mut total_recursive_nodes = 0usize;
    let mut component_cgt_values = Vec::new();

    for component in components {
        let material_value = component_material_balance(&board, component.active_mask);
        let local_move_counts =
            component_local_move_counts(&board, component.mask, component.active_mask);
        let component_evaluation = component_cgt_evaluation(
            spec.value_rule,
            &board,
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
        .expect("non-fixture board composition must satisfy BMCOMPOSE root coverage");
    let composition_digest = bmcompose
        .digest()
        .expect("non-fixture board composition certificate must digest")
        .to_string();

    let mut exact = ExactLabel::from_thermograph_payload(&result_payload);
    exact.value.insert(
        "solver_scope".to_owned(),
        spec.value_rule.solver_scope().to_owned(),
    );
    exact.value.insert(
        "composition_value_rule".to_owned(),
        spec.value_rule.composition_value_rule().to_owned(),
    );
    exact
        .value
        .insert("proof_kind".to_owned(), proof.proof_kind.to_owned());
    exact.value.insert(
        "component_count".to_owned(),
        proof.component_count.to_string(),
    );
    exact.value.insert(
        "component_roots".to_owned(),
        component_root_summaries.join(","),
    );
    exact.value.insert(
        "component_values".to_owned(),
        component_value_summaries.join(","),
    );
    exact.value.insert(
        "component_value_classes".to_owned(),
        component_value_class_summaries.join(","),
    );
    exact.value.insert(
        "component_material_balances".to_owned(),
        component_material_summaries.join(","),
    );
    exact.value.insert(
        "component_local_move_counts".to_owned(),
        component_local_move_summaries.join(","),
    );
    if !component_recursive_node_summaries.is_empty() {
        exact.value.insert(
            "solver_depth".to_owned(),
            COMPONENT_DEPTH_TWO_LOCAL_MOVE_DEPTH.to_string(),
        );
        exact.value.insert(
            "component_recursive_node_counts".to_owned(),
            component_recursive_node_summaries.join(","),
        );
        exact.value.insert(
            "component_recursive_total_nodes".to_owned(),
            total_recursive_nodes.to_string(),
        );
        exact.value.insert(
            "recursive_leaf_rule".to_owned(),
            "component_material_balance_at_depth_cutoff_or_no_moves_v0".to_owned(),
        );
    }
    exact.value.insert(
        "result_digest_v1_sha256".to_owned(),
        result_value.digest_v1_sha256(),
    );

    DatasetLabelRow::exact(
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
            verifier: spec.value_rule.verifier().to_owned(),
            verifier_version: env!("CARGO_PKG_VERSION").to_owned(),
            certificate: LabelCertificate::composition(
                spec.value_rule.certificate_kind(),
                format!(
                    "bitmesh:{};proof:{};rule:{};bmcompose:{};thermograph:{}",
                    decomposition_digest,
                    proof.proof_kind,
                    spec.value_rule.composition_value_rule(),
                    composition_digest,
                    result_payload.digest
                ),
                decomposition_digest.to_string(),
                composition_digest,
                component_values,
                result_payload.digest,
            ),
        },
    )
}

fn non_fixture_composed_board(spec: &NonFixtureComposedBoardSpec) -> Board {
    let mut board = Board::empty();
    for (square, piece) in non_fixture_composed_board_wall_pieces() {
        board.set_piece_at(square, composition_fixture_piece(piece));
    }
    for (square, piece) in spec.active_pieces {
        board.set_piece_at(*square, composition_fixture_piece(*piece));
    }
    board
}

fn non_fixture_composed_board_fen(spec: &NonFixtureComposedBoardSpec) -> String {
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
                            assert_eq!(exact.value_class, ExactValueClass::GameTree);
                            assert_eq!(
                                exact.value.get("solver_depth").map(String::as_str),
                                Some("2")
                            );
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
