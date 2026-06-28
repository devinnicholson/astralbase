use bitmesh::{self, DecompositionRejectionReason, DecompositionStatus};
use shakmaty::{CastlingMode, Chess, EnPassantMode, Position, fen::Fen};
use std::{fmt, str::FromStr};

pub const FIRST_CONSTRAINED_DOMAIN_ID: &str = "formal_domain:first_constrained_chess:v0";
pub const FIRST_CONSTRAINED_DOMAIN_DEFINITION: &str =
    "docs/formal_domain.md#first-candidate-domain";
pub const FIRST_CONSTRAINED_MAX_PIECES: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedDomainPosition {
    fen: String,
    position: Chess,
    terminal_status: Option<TerminalStatus>,
    decomposition: DecompositionGate,
}

impl ValidatedDomainPosition {
    #[must_use]
    pub fn fen(&self) -> &str {
        &self.fen
    }

    #[must_use]
    pub fn position(&self) -> &Chess {
        &self.position
    }

    #[must_use]
    pub fn terminal_status(&self) -> Option<TerminalStatus> {
        self.terminal_status
    }

    #[must_use]
    pub fn decomposition(&self) -> &DecompositionGate {
        &self.decomposition
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecompositionGate {
    pub status: DecompositionGateStatus,
    pub active_component_count: u8,
    pub digest: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecompositionGateStatus {
    Strict,
    Rejected(DecompositionGateRejection),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecompositionGateRejection {
    NoLockedBarrier,
    LessThanTwoActiveComponents,
    InvalidCertificate,
}

impl DecompositionGateRejection {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoLockedBarrier => "no_locked_barrier",
            Self::LessThanTwoActiveComponents => "less_than_two_active_components",
            Self::InvalidCertificate => "invalid_certificate",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalStatus {
    Checkmate,
    Stalemate,
}

impl TerminalStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Checkmate => "checkmate",
            Self::Stalemate => "stalemate",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainRejectionReport {
    fen: String,
    reasons: Vec<DomainRejection>,
}

impl DomainRejectionReport {
    #[must_use]
    pub fn fen(&self) -> &str {
        &self.fen
    }

    #[must_use]
    pub fn reasons(&self) -> &[DomainRejection] {
        &self.reasons
    }

    #[must_use]
    pub fn reason_messages(&self) -> Vec<String> {
        self.reasons.iter().map(ToString::to_string).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainRejection {
    pub code: DomainRejectionCode,
    pub detail: Option<String>,
}

impl DomainRejection {
    #[must_use]
    pub fn new(code: DomainRejectionCode) -> Self {
        Self { code, detail: None }
    }

    #[must_use]
    pub fn with_detail(code: DomainRejectionCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: Some(detail.into()),
        }
    }
}

impl fmt::Display for DomainRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.code.message())?;
        if let Some(detail) = &self.detail {
            write!(f, " ({detail})")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainRejectionCode {
    InvalidFen,
    InvalidPosition,
    CastlingRights,
    EnPassantTarget,
    TooManyPieces,
    NoStrictDecomposition,
    CertificateInvalid,
}

impl DomainRejectionCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidFen => "invalid_fen",
            Self::InvalidPosition => "invalid_position",
            Self::CastlingRights => "castling_rights",
            Self::EnPassantTarget => "en_passant_target",
            Self::TooManyPieces => "too_many_pieces",
            Self::NoStrictDecomposition => "no_strict_decomposition",
            Self::CertificateInvalid => "certificate_invalid",
        }
    }

    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidFen => "The first constrained chess domain accepts standard 8x8 FEN only.",
            Self::InvalidPosition => {
                "The first constrained chess domain accepts legal standard chess positions only."
            }
            Self::CastlingRights => {
                "The first constrained domain accepts only positions with no castling rights."
            }
            Self::EnPassantTarget => {
                "The first constrained domain accepts only positions with no en-passant target."
            }
            Self::TooManyPieces => {
                "The first constrained mini-endgame domain is capped at 8 pieces."
            }
            Self::NoStrictDecomposition => {
                "Non-terminal positions require a strict bitmesh decomposition certificate."
            }
            Self::CertificateInvalid => {
                "The bitmesh decomposition certificate failed structural validation."
            }
        }
    }
}

pub fn validate_first_constrained_fen(
    fen: impl AsRef<str>,
) -> Result<ValidatedDomainPosition, DomainRejectionReport> {
    let fen = fen.as_ref().to_owned();
    let parsed_fen = Fen::from_str(&fen).map_err(|error| {
        rejection_report(
            &fen,
            vec![DomainRejection::with_detail(
                DomainRejectionCode::InvalidFen,
                error.to_string(),
            )],
        )
    })?;

    let position: Chess = parsed_fen
        .into_position(CastlingMode::Standard)
        .map_err(|error| {
            rejection_report(
                &fen,
                vec![DomainRejection::with_detail(
                    DomainRejectionCode::InvalidPosition,
                    error.to_string(),
                )],
            )
        })?;

    let mut reasons = Vec::new();
    if !position.castles().castling_rights().is_empty() {
        reasons.push(DomainRejection::new(DomainRejectionCode::CastlingRights));
    }
    if position.ep_square(EnPassantMode::Always).is_some() {
        reasons.push(DomainRejection::new(DomainRejectionCode::EnPassantTarget));
    }

    let piece_count = position.board().occupied().count();
    if piece_count > FIRST_CONSTRAINED_MAX_PIECES {
        reasons.push(DomainRejection::with_detail(
            DomainRejectionCode::TooManyPieces,
            format!("{piece_count} pieces"),
        ));
    }

    if !reasons.is_empty() {
        return Err(rejection_report(&fen, reasons));
    }

    let terminal_status = terminal_status(&position);
    let decomposition =
        probe_decomposition(&position).map_err(|reason| rejection_report(&fen, vec![reason]))?;

    if terminal_status.is_none() && !matches!(decomposition.status, DecompositionGateStatus::Strict)
    {
        let detail = match decomposition.status {
            DecompositionGateStatus::Strict => None,
            DecompositionGateStatus::Rejected(rejection) => {
                Some(format!("bitmesh reported {}", rejection.as_str()))
            }
        };
        let reason = match detail {
            Some(detail) => {
                DomainRejection::with_detail(DomainRejectionCode::NoStrictDecomposition, detail)
            }
            None => DomainRejection::new(DomainRejectionCode::NoStrictDecomposition),
        };
        return Err(rejection_report(&fen, vec![reason]));
    }

    Ok(ValidatedDomainPosition {
        fen,
        position,
        terminal_status,
        decomposition,
    })
}

fn terminal_status(position: &Chess) -> Option<TerminalStatus> {
    if !position.legal_moves().is_empty() {
        return None;
    }

    Some(if position.is_check() {
        TerminalStatus::Checkmate
    } else {
        TerminalStatus::Stalemate
    })
}

fn probe_decomposition(position: &Chess) -> Result<DecompositionGate, DomainRejection> {
    let certificate = bitmesh::certify_decomposition(position.board());
    let digest = certificate.digest().map_err(|error| {
        DomainRejection::with_detail(
            DomainRejectionCode::CertificateInvalid,
            format!("{error:?}"),
        )
    })?;

    let status = match certificate.status {
        DecompositionStatus::Strict => DecompositionGateStatus::Strict,
        DecompositionStatus::Rejected => {
            let rejection = match certificate.rejection_reason {
                Some(DecompositionRejectionReason::NoLockedBarrier) => {
                    DecompositionGateRejection::NoLockedBarrier
                }
                Some(DecompositionRejectionReason::LessThanTwoActiveComponents) => {
                    DecompositionGateRejection::LessThanTwoActiveComponents
                }
                None => DecompositionGateRejection::InvalidCertificate,
            };
            DecompositionGateStatus::Rejected(rejection)
        }
    };

    Ok(DecompositionGate {
        status,
        active_component_count: certificate.active_component_count,
        digest: digest.to_string(),
    })
}

fn rejection_report(fen: &str, reasons: Vec<DomainRejection>) -> DomainRejectionReport {
    DomainRejectionReport {
        fen: fen.to_owned(),
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_checkmate_is_inside_first_constrained_domain() {
        let validated = validate_first_constrained_fen("7k/5KQ1/8/8/8/8/8/8 b - - 0 1").unwrap();

        assert_eq!(validated.terminal_status(), Some(TerminalStatus::Checkmate));
        assert_eq!(
            validated.decomposition().status,
            DecompositionGateStatus::Rejected(DecompositionGateRejection::NoLockedBarrier)
        );
        assert_eq!(validated.decomposition().digest.len(), 64);
    }

    #[test]
    fn castling_rights_are_structured_rejections() {
        let report =
            validate_first_constrained_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap_err();

        assert_eq!(
            report.reasons()[0].code,
            DomainRejectionCode::CastlingRights
        );
        assert!(
            report.reason_messages()[0].contains("accepts only positions with no castling rights")
        );
    }

    #[test]
    fn non_terminal_positions_need_strict_decomposition() {
        let report = validate_first_constrained_fen("8/8/8/8/8/8/8/4K2k w - - 0 1").unwrap_err();

        assert_eq!(
            report.reasons()[0].code,
            DomainRejectionCode::NoStrictDecomposition
        );
    }
}
