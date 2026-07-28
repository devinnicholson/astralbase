//! Target-aware verification for bounded board-derived CGT candidates.
//!
//! This adapter verifies one deliberately narrow research contract. It parses
//! the board field of a FEN, asks Bitmesh for a board-local conservative
//! decomposition, evaluates both certified components with the declared
//! depth-two local-move rule, and compares the resulting Thermograph
//! domain-separated SHA-256 digest with a requested target.
//!
//! A match is structural tree identity under that bounded rule. Equality
//! between arbitrary combinatorial games and orthodox-chess game-tree
//! solutions lie outside this board-level contract; so do side to move, check,
//! castling, and en-passant metadata.

use bitmesh::{
    self, CompositionCertificate as BitmeshCompositionCertificate, CompositionComponentValue,
};
use serde::{Deserialize, Serialize};
use shakmaty::{Board, Color, Square};
use std::{collections::BTreeMap, str::FromStr};
use thermograph::CGTValue;

/// The only domain currently supported by the target-aware verifier.
pub const SUPPORTED_DOMAIN_ID: &str = "formal_domain:bitmesh_composed_board_material:v0";

/// The bounded component evaluation rule implemented by this adapter.
pub const SUPPORTED_VALUE_RULE: &str = "component_depth2_local_move_game_v0";

/// The collision-resistant structural identity kind emitted by Thermograph.
pub const SUPPORTED_IDENTITY_KIND: &str = "thermograph_structural_tree_v1";

/// The precise meaning of a successful identity comparison.
pub const STRUCTURAL_IDENTITY_SEMANTICS: &str = "structural_tree_identity_only";

const COMPONENT_DEPTH: u8 = 2;

/// A single JSONL request to verify a board-derived target candidate.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetCandidateRequest {
    /// Caller-defined identifier copied into the response.
    pub request_id: String,
    /// Formal domain identifier for the requested computation.
    pub domain_id: String,
    /// Position to evaluate.
    pub position: TargetCandidatePosition,
    /// Explicit bounded value rule.
    pub value_rule: String,
    /// Structural target against which the computed value is compared.
    pub target: StructuralTarget,
    /// Maximum number of recursive component nodes the verifier may visit.
    pub node_budget: usize,
}

/// Board position supplied to the verifier.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetCandidatePosition {
    /// Position encoding. Only `fen` is supported.
    pub encoding: String,
    /// FEN text. Only its board field participates in this contract.
    pub text: String,
}

/// Collision-resistant structural target supplied by the caller.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuralTarget {
    /// Identity algorithm. Only `thermograph_structural_tree_v1` is supported.
    pub identity_kind: String,
    /// Expected stable Thermograph value-class label.
    pub value_class: String,
    /// Expected domain-separated Thermograph SHA-256 digest.
    pub digest_v1_sha256: String,
}

/// Exhaustive machine-readable outcome of a target verification request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetVerificationStatus {
    /// The candidate is valid under the declared rule and matches the target.
    VerifiedMatch,
    /// The candidate is valid under the declared rule but differs from the target.
    VerifiedNonmatch,
    /// The request or candidate is outside the supported verification contract.
    Rejected,
    /// An unexpected internal certificate construction or serialization failed.
    InternalError,
}

/// A deterministic response for one target verification request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TargetCandidateResponse {
    /// Request identifier, or a stable `line-N` identifier for malformed JSON.
    pub request_id: String,
    /// One of the four public verifier outcomes.
    pub status: TargetVerificationStatus,
    /// Stable rejection or failure code, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
    /// Human-readable rejection or failure detail, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Computed identity and certificate metadata for verified candidates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<VerifiedStructuralIdentity>,
}

/// Computed structural identity and replay provenance for a verified candidate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VerifiedStructuralIdentity {
    /// Identity algorithm used for the comparison.
    pub identity_kind: &'static str,
    /// Stable description of the bounded structural-identity semantics.
    pub semantics: &'static str,
    /// Stable Thermograph value-class label.
    pub value_class: String,
    /// Domain-separated SHA-256 structural digest used for target comparison.
    pub digest_v1_sha256: String,
    /// Legacy FNV-1a structural digest, retained only for dataset compatibility.
    pub legacy_digest: String,
    /// Recursive component nodes visited under the declared bounded rule.
    pub recursive_nodes: usize,
    /// Bitmesh decomposition certificate SHA-256 digest.
    pub decomposition_digest: String,
    /// Bitmesh composition certificate SHA-256 digest.
    pub composition_digest: String,
    /// Legacy component value digests keyed by sorted component root.
    pub component_legacy_digests: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug)]
struct ComponentLocalMove {
    from: Square,
    to: Square,
}

enum CandidateError {
    Rejected {
        reason_code: &'static str,
        reason: String,
    },
    Internal {
        reason_code: &'static str,
        reason: String,
    },
}

impl CandidateError {
    fn rejected(reason_code: &'static str, reason: impl Into<String>) -> Self {
        Self::Rejected {
            reason_code,
            reason: reason.into(),
        }
    }

    fn internal(reason_code: &'static str, reason: impl Into<String>) -> Self {
        Self::Internal {
            reason_code,
            reason: reason.into(),
        }
    }
}

/// Verifies one parsed target request.
///
/// `VerifiedMatch` and `VerifiedNonmatch` both mean that the candidate was
/// replayed successfully under [`SUPPORTED_VALUE_RULE`]. Only their structural
/// target comparison differs.
#[must_use]
pub fn verify_target_candidate(request: &TargetCandidateRequest) -> TargetCandidateResponse {
    let request_id = request.request_id.clone();
    match verify_target_candidate_inner(request) {
        Ok(actual) => {
            let target_matches = request.target.value_class == actual.value_class
                && request.target.digest_v1_sha256 == actual.digest_v1_sha256;
            TargetCandidateResponse {
                request_id,
                status: if target_matches {
                    TargetVerificationStatus::VerifiedMatch
                } else {
                    TargetVerificationStatus::VerifiedNonmatch
                },
                reason_code: None,
                reason: None,
                actual: Some(actual),
            }
        }
        Err(CandidateError::Rejected {
            reason_code,
            reason,
        }) => TargetCandidateResponse {
            request_id,
            status: TargetVerificationStatus::Rejected,
            reason_code: Some(reason_code.to_owned()),
            reason: Some(reason),
            actual: None,
        },
        Err(CandidateError::Internal {
            reason_code,
            reason,
        }) => TargetCandidateResponse {
            request_id,
            status: TargetVerificationStatus::InternalError,
            reason_code: Some(reason_code.to_owned()),
            reason: Some(reason),
            actual: None,
        },
    }
}

/// Verifies newline-delimited JSON requests and returns one compact JSON
/// response per non-empty input line, preserving input order.
///
/// Malformed request lines produce a `rejected` response with a stable
/// one-based `line-N` identifier. The output ends in a newline when at least one
/// non-empty request line was supplied.
#[must_use]
pub fn verify_target_candidates_jsonl(input: &str) -> String {
    let mut output = String::new();
    for (line_index, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<TargetCandidateRequest>(line) {
            Ok(request) => verify_target_candidate(&request),
            Err(error) => TargetCandidateResponse {
                request_id: format!("line-{}", line_index + 1),
                status: TargetVerificationStatus::Rejected,
                reason_code: Some("invalid_request_json".to_owned()),
                reason: Some(error.to_string()),
                actual: None,
            },
        };
        output.push_str(
            serde_json::to_string(&response)
                .expect("target verifier response contains only JSON-serializable values")
                .as_str(),
        );
        output.push('\n');
    }
    output
}

fn verify_target_candidate_inner(
    request: &TargetCandidateRequest,
) -> Result<VerifiedStructuralIdentity, CandidateError> {
    validate_request(request)?;

    let board_part = request
        .position
        .text
        .split_whitespace()
        .next()
        .ok_or_else(|| CandidateError::rejected("invalid_fen", "missing board FEN field"))?;
    let board = Board::from_str(board_part)
        .map_err(|error| CandidateError::rejected("invalid_fen", error.to_string()))?;

    let decomposition = bitmesh::certify_decomposition(&board);
    let proof = bitmesh::verify_conservative_legal_independence(&board, &decomposition).map_err(
        |error| {
            CandidateError::rejected(
                "decomposition_rejected",
                format!("conservative board-local decomposition rejected: {error:?}"),
            )
        },
    )?;
    if proof.component_count != 2 {
        return Err(CandidateError::rejected(
            "unsupported_component_count",
            format!(
                "the supported domain requires exactly two components, got {}",
                proof.component_count
            ),
        ));
    }

    let mut components = decomposition.components.iter().collect::<Vec<_>>();
    components.sort_by_key(|component| component.root);

    let mut remaining_nodes = request.node_budget;
    let mut component_values = Vec::with_capacity(components.len());
    let mut component_legacy_digests = BTreeMap::new();
    let mut bmcompose_component_values = Vec::with_capacity(components.len());

    for component in components {
        let value = component_recursive_local_move_game_value_bounded(
            &board,
            component.mask,
            component.active_mask,
            COMPONENT_DEPTH,
            &mut remaining_nodes,
        )?;
        let payload = value.exact_value_payload();
        component_legacy_digests.insert(component.root.to_string(), payload.digest.clone());
        bmcompose_component_values.push(CompositionComponentValue {
            component_root: component.root,
            value_digest: payload.digest,
        });
        component_values.push(value);
    }

    let result_value = CGTValue::sum_all(&component_values);
    let result_payload = result_value.exact_value_payload();
    let bmcompose = BitmeshCompositionCertificate {
        decomposition_digest: proof.decomposition_digest,
        component_values: bmcompose_component_values,
        result_value_digest: result_payload.digest.clone(),
    };
    bmcompose
        .validate_against_decomposition(&decomposition)
        .map_err(|error| {
            CandidateError::internal(
                "composition_validation_failed",
                format!("Bitmesh composition validation failed: {error:?}"),
            )
        })?;
    let composition_digest = bmcompose
        .digest()
        .map_err(|error| {
            CandidateError::internal(
                "composition_digest_failed",
                format!("Bitmesh composition digest failed: {error:?}"),
            )
        })?
        .to_string();

    Ok(VerifiedStructuralIdentity {
        identity_kind: SUPPORTED_IDENTITY_KIND,
        semantics: STRUCTURAL_IDENTITY_SEMANTICS,
        value_class: result_payload.value_class.as_str().to_owned(),
        digest_v1_sha256: result_value.digest_v1_sha256(),
        legacy_digest: result_payload.digest,
        recursive_nodes: request.node_budget - remaining_nodes,
        decomposition_digest: proof.decomposition_digest.to_string(),
        composition_digest,
        component_legacy_digests,
    })
}

fn validate_request(request: &TargetCandidateRequest) -> Result<(), CandidateError> {
    if request.request_id.is_empty() {
        return Err(CandidateError::rejected(
            "invalid_request_id",
            "request_id must not be empty",
        ));
    }
    if request.domain_id != SUPPORTED_DOMAIN_ID {
        return Err(CandidateError::rejected(
            "unsupported_domain",
            format!("unsupported domain_id: {}", request.domain_id),
        ));
    }
    if request.position.encoding != "fen" {
        return Err(CandidateError::rejected(
            "unsupported_position_encoding",
            format!(
                "unsupported position encoding: {}",
                request.position.encoding
            ),
        ));
    }
    if request.value_rule != SUPPORTED_VALUE_RULE {
        return Err(CandidateError::rejected(
            "unsupported_value_rule",
            format!("unsupported value_rule: {}", request.value_rule),
        ));
    }
    if request.target.identity_kind != SUPPORTED_IDENTITY_KIND {
        return Err(CandidateError::rejected(
            "unsupported_identity_kind",
            format!(
                "unsupported target identity_kind: {}",
                request.target.identity_kind
            ),
        ));
    }
    if !matches!(
        request.target.value_class.as_str(),
        "number" | "star" | "up" | "down" | "switch" | "game_tree"
    ) {
        return Err(CandidateError::rejected(
            "unsupported_value_class",
            format!(
                "unsupported target value_class: {}",
                request.target.value_class
            ),
        ));
    }
    if !is_lowercase_sha256(&request.target.digest_v1_sha256) {
        return Err(CandidateError::rejected(
            "invalid_target_digest",
            "target digest_v1_sha256 must be exactly 64 lowercase hexadecimal characters",
        ));
    }
    if request.node_budget == 0 {
        return Err(CandidateError::rejected(
            "invalid_node_budget",
            "node_budget must be greater than zero",
        ));
    }
    Ok(())
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn component_recursive_local_move_game_value_bounded(
    board: &Board,
    component_mask: shakmaty::Bitboard,
    active_mask: shakmaty::Bitboard,
    depth: u8,
    remaining_nodes: &mut usize,
) -> Result<CGTValue, CandidateError> {
    if *remaining_nodes == 0 {
        return Err(CandidateError::rejected(
            "node_budget_exceeded",
            "component replay exceeded the declared recursive node budget",
        ));
    }
    *remaining_nodes -= 1;

    let material_value = component_material_balance(board, active_mask);
    if depth == 0 {
        return Ok(CGTValue::Integer(material_value));
    }

    let left_moves = component_local_moves(board, component_mask, active_mask, Color::White);
    let right_moves = component_local_moves(board, component_mask, active_mask, Color::Black);
    if left_moves.is_empty() && right_moves.is_empty() {
        return Ok(CGTValue::Integer(material_value));
    }

    let mut left = Vec::with_capacity(left_moves.len());
    for local_move in left_moves {
        let (child_board, child_active_mask) =
            apply_component_local_move(board, active_mask, local_move);
        left.push(component_recursive_local_move_game_value_bounded(
            &child_board,
            component_mask,
            child_active_mask,
            depth - 1,
            remaining_nodes,
        )?);
    }

    let mut right = Vec::with_capacity(right_moves.len());
    for local_move in right_moves {
        let (child_board, child_active_mask) =
            apply_component_local_move(board, active_mask, local_move);
        right.push(component_recursive_local_move_game_value_bounded(
            &child_board,
            component_mask,
            child_active_mask,
            depth - 1,
            remaining_nodes,
        )?);
    }

    Ok(CGTValue::GameTree { left, right })
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
