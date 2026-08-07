use super::*;

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
    pub(super) fn row(message: impl Into<String>) -> Self {
        Self {
            line_number: None,
            message: message.into(),
        }
    }

    pub(super) fn line(line_number: usize, message: impl Into<String>) -> Self {
        Self {
            line_number: Some(line_number),
            message: message.into(),
        }
    }
}

pub type LabelValidationResult<T> = Result<T, Vec<LabelValidationIssue>>;
