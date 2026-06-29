use crate::domain::{
    self, FIRST_CONSTRAINED_DOMAIN_DEFINITION, FIRST_CONSTRAINED_DOMAIN_ID,
    ImmediateTerminalTactic, TerminalStatus, ValidatedDomainPosition,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use thermograph::{CGTValue, ExactValuePayload as ThermographExactValuePayload};

pub const DATASET_LABEL_SCHEMA_VERSION: &str = "partizan.dataset_label.v0";
pub const FORMAL_CGT_DOMAIN_ID: &str = "formal_domain:thermograph_golden_cgt:v0";
pub const FORMAL_CGT_DOMAIN_DEFINITION: &str = "thermograph:golden_values#hot_one_minus_one";
pub const DEFAULT_FRONTIER_SHARD_LIMIT: usize = 1_000;
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
            validate_exact_provenance(provenance, &mut issues);
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
    if limit == 0 {
        return Vec::new();
    }

    let exact_target = frontier_exact_target(limit);
    let rejected_target = limit.saturating_sub(exact_target);
    let mut exact_rows = Vec::with_capacity(exact_target);
    let mut rejected_rows = Vec::with_capacity(rejected_target);

    for (candidate_index, fen) in kqk_candidate_fens().into_iter().enumerate() {
        if exact_rows.len() >= exact_target && rejected_rows.len() >= rejected_target {
            break;
        }

        let row_id = format!("astralbase-w6-kqk-frontier-{candidate_index:06}");
        let Some(row) = generated_kqk_row(&row_id, fen.as_str()) else {
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

fn kqk_candidate_fens() -> Vec<String> {
    let mut fens = Vec::new();

    for side_to_move in ["w", "b"] {
        for white_king in 0..64 {
            for black_king in 0..64 {
                if black_king == white_king {
                    continue;
                }
                for white_queen in 0..64 {
                    if white_queen == white_king || white_queen == black_king {
                        continue;
                    }

                    let board =
                        board_fen(&[(white_king, 'K'), (black_king, 'k'), (white_queen, 'Q')]);
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

fn generated_kqk_row(row_id: &str, fen: &str) -> Option<DatasetLabelRow> {
    match domain::validate_first_constrained_fen(fen) {
        Ok(validated) => exact_row(row_id, &validated, KQK_FRONTIER_EXACT_CONTEXT).or_else(|| {
            Some(DatasetLabelRow::rejected(
                row_id,
                FIRST_CONSTRAINED_DOMAIN_ID,
                DatasetPosition::fen(fen),
                RejectedLabel::unsupported(vec![
                    "frontier_generator: legal KQK candidate has no terminal or one-ply terminal-frontier certificate".to_owned(),
                ]),
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
            certificate: LabelCertificate {
                kind: "terminal-legal-move-enumeration+thermograph-exact-value+bitmesh-domain-gate"
                    .to_owned(),
                digest: format!(
                    "bitmesh:{};thermograph:{}",
                    validated.decomposition().digest.as_str(),
                    payload.digest
                ),
            },
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
            certificate: LabelCertificate {
                kind: "immediate-checkmate-enumeration+thermograph-exact-value+bitmesh-domain-gate"
                    .to_owned(),
                digest: format!(
                    "bitmesh:{};thermograph:{};frontier:{};frontier_thermograph:{}",
                    validated.decomposition().digest.as_str(),
                    payload.digest,
                    tactic_digest,
                    frontier_payload.digest
                ),
            },
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
            certificate: LabelCertificate {
                kind: "thermograph-canonical-switch-fixture".to_owned(),
                digest: format!("thermograph:{}", payload.digest),
            },
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

fn validate_exact_provenance(provenance: &ExactProvenance, issues: &mut Vec<LabelValidationIssue>) {
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
    require_non_empty(
        provenance.certificate.kind.as_str(),
        "provenance.certificate.kind",
        issues,
    );
    require_non_empty(
        provenance.certificate.digest.as_str(),
        "provenance.certificate.digest",
        issues,
    );
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
