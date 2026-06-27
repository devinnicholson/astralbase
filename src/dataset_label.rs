use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const DATASET_LABEL_SCHEMA_VERSION: &str = "partizan.dataset_label.v0";
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionEncoding {
    Fen,
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExactStatus {
    Verified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExactValueClass {
    Integer,
    Dyadic,
    Switch,
    Fuzzy,
    Infinitesimal,
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

#[must_use]
pub fn sample_audited_shard() -> Vec<DatasetLabelRow> {
    const DOMAIN: &str = "formal_domain:first_constrained_chess:v0";

    let mut exact_value = BTreeMap::new();
    exact_value.insert("canonical".to_owned(), "0".to_owned());
    exact_value.insert("mean".to_owned(), "0".to_owned());
    exact_value.insert("temperature".to_owned(), "0".to_owned());

    vec![
        DatasetLabelRow::exact(
            "astralbase-sample-exact-terminal-001",
            DOMAIN,
            DatasetPosition::fen("8/8/8/8/8/8/8/4K2k w - - 0 1"),
            ExactLabel::verified(exact_value, ExactValueClass::Integer),
            ExactProvenance {
                code_commit: "astralbase-scaffold".to_owned(),
                generator: "astralbase_sample_audited_shard".to_owned(),
                generator_config_hash: "sha256:astralbase-sample-config".to_owned(),
                random_seed: 0,
                domain_definition: "docs/formal_domain.md#first-constrained-domain".to_owned(),
                verifier: "astralbase_terminal_fixture".to_owned(),
                verifier_version: env!("CARGO_PKG_VERSION").to_owned(),
                certificate: LabelCertificate {
                    kind: "terminal-tablebase".to_owned(),
                    digest: "sha256:astralbase-sample-terminal".to_owned(),
                },
            },
        ),
        DatasetLabelRow::rejected(
            "astralbase-sample-rejected-unsupported-001",
            DOMAIN,
            DatasetPosition::fen("8/8/8/8/8/8/8/4K2k w KQ - 0 1"),
            RejectedLabel::unsupported(vec![
                "Castling rights are outside the first constrained domain.".to_owned(),
            ]),
        ),
    ]
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
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].label_kind(), LabelKind::Exact);
        assert_eq!(rows[1].label_kind(), LabelKind::Rejected);

        for row in &rows {
            validate_dataset_label_row(row).unwrap();
        }

        let jsonl = serialize_jsonl(&rows).unwrap();
        let parsed = parse_and_validate_jsonl(&jsonl).unwrap();
        assert_eq!(parsed, rows);
    }

    #[test]
    fn sample_audited_shard_jsonl_is_deterministic() {
        let jsonl = sample_audited_shard_jsonl().unwrap();

        assert_eq!(
            jsonl,
            concat!(
                "{\"schema_version\":\"partizan.dataset_label.v0\",\"row_id\":\"astralbase-sample-exact-terminal-001\",\"domain\":\"formal_domain:first_constrained_chess:v0\",\"position\":{\"encoding\":\"fen\",\"text\":\"8/8/8/8/8/8/8/4K2k w - - 0 1\"},\"label_kind\":\"exact\",\"exact\":{\"status\":\"verified\",\"value\":{\"canonical\":\"0\",\"mean\":\"0\",\"temperature\":\"0\"},\"value_class\":\"integer\"},\"provenance\":{\"code_commit\":\"astralbase-scaffold\",\"generator\":\"astralbase_sample_audited_shard\",\"generator_config_hash\":\"sha256:astralbase-sample-config\",\"random_seed\":0,\"domain_definition\":\"docs/formal_domain.md#first-constrained-domain\",\"verifier\":\"astralbase_terminal_fixture\",\"verifier_version\":\"0.1.0\",\"certificate\":{\"kind\":\"terminal-tablebase\",\"digest\":\"sha256:astralbase-sample-terminal\"}}}\n",
                "{\"schema_version\":\"partizan.dataset_label.v0\",\"row_id\":\"astralbase-sample-rejected-unsupported-001\",\"domain\":\"formal_domain:first_constrained_chess:v0\",\"position\":{\"encoding\":\"fen\",\"text\":\"8/8/8/8/8/8/8/4K2k w KQ - 0 1\"},\"label_kind\":\"rejected\",\"rejected\":{\"status\":\"unsupported\",\"reasons\":[\"Castling rights are outside the first constrained domain.\"]}}\n"
            )
        );
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
