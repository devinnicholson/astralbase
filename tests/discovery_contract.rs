use astralbase::discovery::{
    SUPPORTED_DOMAIN_ID, SUPPORTED_IDENTITY_KIND, SUPPORTED_VALUE_RULE, StructuralTarget,
    TargetCandidatePosition, TargetCandidateRequest, TargetVerificationStatus,
    verify_target_candidate, verify_target_candidates_jsonl,
};
use serde_json::Value;
use std::{collections::BTreeMap, process::Command};

const WAVE55_ROW_090_FEN: &str = "3p4/3P2Nn/3p4/3P1n2/3p4/2pP4/3p4/Qn1P4 w - - 0 290";
const WAVE55_ROW_090_SHA256: &str =
    "d31d0b8acc19a8174b7c3a48d268cd7d63effc9213b7c91b7b50e70260398bbc";
const WAVE55_ROW_090_LEGACY_DIGEST: &str = "c4135b41b202d498";
const WAVE55_ROW_090_DECOMPOSITION_DIGEST: &str =
    "cb18d5497662eadfeebfafe22ea8b3f3eddc609ab71fff49950411b749fe6115";
const WAVE55_ROW_090_COMPOSITION_DIGEST: &str =
    "d311c1a0c1b3c8878b395b7ae5c531aa6579924ddeff495e11d04c3688ef4460";

fn wave55_request(request_id: &str) -> TargetCandidateRequest {
    TargetCandidateRequest {
        request_id: request_id.to_owned(),
        domain_id: SUPPORTED_DOMAIN_ID.to_owned(),
        position: TargetCandidatePosition {
            encoding: "fen".to_owned(),
            text: WAVE55_ROW_090_FEN.to_owned(),
        },
        value_rule: SUPPORTED_VALUE_RULE.to_owned(),
        target: StructuralTarget {
            identity_kind: SUPPORTED_IDENTITY_KIND.to_owned(),
            value_class: "game_tree".to_owned(),
            digest_v1_sha256: WAVE55_ROW_090_SHA256.to_owned(),
        },
        node_budget: 349,
    }
}

#[test]
fn wave55_row_090_matches_structural_target() {
    let response = verify_target_candidate(&wave55_request("wave55-row090"));
    assert_eq!(response.status, TargetVerificationStatus::VerifiedMatch);
    assert_eq!(response.reason_code, None);

    let actual = response
        .actual
        .expect("verified candidate has actual identity");
    assert_eq!(actual.value_class, "game_tree");
    assert_eq!(actual.digest_v1_sha256, WAVE55_ROW_090_SHA256);
    assert_eq!(actual.legacy_digest, WAVE55_ROW_090_LEGACY_DIGEST);
    assert_eq!(actual.recursive_nodes, 349);
    assert_eq!(
        actual.decomposition_digest,
        WAVE55_ROW_090_DECOMPOSITION_DIGEST
    );
    assert_eq!(actual.composition_digest, WAVE55_ROW_090_COMPOSITION_DIGEST);
    assert_eq!(
        actual.component_legacy_digests,
        BTreeMap::from([
            ("9".to_owned(), "352e7aac7426b85a".to_owned()),
            ("13".to_owned(), "9e60f90d64f62a64".to_owned()),
        ])
    );
}

#[test]
fn valid_candidate_with_wrong_target_is_a_verified_nonmatch() {
    let mut request = wave55_request("wrong-target");
    request.target.digest_v1_sha256 = "0".repeat(64);

    let response = verify_target_candidate(&request);
    assert_eq!(response.status, TargetVerificationStatus::VerifiedNonmatch);
    assert_eq!(
        response
            .actual
            .expect("verified nonmatch has actual identity")
            .digest_v1_sha256,
        WAVE55_ROW_090_SHA256
    );
}

#[test]
fn unsupported_rule_and_insufficient_budget_are_rejected() {
    let mut unsupported_rule = wave55_request("unsupported-rule");
    unsupported_rule.value_rule = "unbounded_orthodox_chess_equality".to_owned();
    let response = verify_target_candidate(&unsupported_rule);
    assert_eq!(response.status, TargetVerificationStatus::Rejected);
    assert_eq!(
        response.reason_code.as_deref(),
        Some("unsupported_value_rule")
    );

    let mut insufficient_budget = wave55_request("small-budget");
    insufficient_budget.node_budget = 348;
    let response = verify_target_candidate(&insufficient_budget);
    assert_eq!(response.status, TargetVerificationStatus::Rejected);
    assert_eq!(
        response.reason_code.as_deref(),
        Some("node_budget_exceeded")
    );
}

#[test]
fn stiller_kernel_without_locked_barrier_is_rejected() {
    let mut request = wave55_request("stiller-kernel");
    request.position.text = "8/q7/5q2/8/8/8/6QQ/k6K w - - 0 1".to_owned();
    request.node_budget = 1_000;

    let response = verify_target_candidate(&request);
    assert_eq!(response.status, TargetVerificationStatus::Rejected);
    assert_eq!(
        response.reason_code.as_deref(),
        Some("decomposition_rejected")
    );
}

#[test]
fn malformed_json_and_invalid_target_digest_are_rejected() {
    let malformed = verify_target_candidates_jsonl("{not-json}\n");
    let response: Value = serde_json::from_str(malformed.trim()).expect("response is JSON");
    assert_eq!(response["request_id"], "line-1");
    assert_eq!(response["status"], "rejected");
    assert_eq!(response["reason_code"], "invalid_request_json");

    let mut invalid_digest = wave55_request("bad-digest");
    invalid_digest.target.digest_v1_sha256 = "ABC".to_owned();
    let response = verify_target_candidate(&invalid_digest);
    assert_eq!(response.status, TargetVerificationStatus::Rejected);
    assert_eq!(
        response.reason_code.as_deref(),
        Some("invalid_target_digest")
    );
}

#[test]
fn jsonl_batch_output_is_byte_deterministic_and_preserves_order() {
    let first = serde_json::to_string(&wave55_request("first")).expect("request serializes");
    let mut wrong = wave55_request("second");
    wrong.target.digest_v1_sha256 = "0".repeat(64);
    let second = serde_json::to_string(&wrong).expect("request serializes");
    let input = format!("{first}\n\n{second}\n");

    let output = verify_target_candidates_jsonl(&input);
    assert_eq!(output, verify_target_candidates_jsonl(&input));
    let responses = output
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("response is JSON"))
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["request_id"], "first");
    assert_eq!(responses[0]["status"], "verified_match");
    assert_eq!(responses[1]["request_id"], "second");
    assert_eq!(responses[1]["status"], "verified_nonmatch");

    let reversed = verify_target_candidates_jsonl(format!("{second}\n{first}\n").as_str());
    let reversed_ids = reversed
        .lines()
        .map(|line| {
            serde_json::from_str::<Value>(line).expect("response is JSON")["request_id"]
                .as_str()
                .expect("request id is a string")
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(reversed_ids, ["second", "first"]);
}

#[test]
fn verify_target_candidates_cli_emits_compact_jsonl() {
    let input_path = std::env::temp_dir().join(format!(
        "astralbase-wave68-cli-{}.jsonl",
        std::process::id()
    ));
    let request = serde_json::to_string(&wave55_request("cli-row090")).expect("request serializes");
    std::fs::write(&input_path, format!("{request}\n"))
        .expect("temporary request file is writable");

    let output = Command::new(env!("CARGO_BIN_EXE_astralbase"))
        .arg("--verify-target-candidates")
        .arg(&input_path)
        .output()
        .expect("astralbase CLI runs");
    std::fs::remove_file(&input_path).expect("temporary request file is removable");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("CLI output is UTF-8");
    assert_eq!(stdout.lines().count(), 1);
    let response: Value = serde_json::from_str(stdout.trim()).expect("CLI response is JSON");
    assert_eq!(response["request_id"], "cli-row090");
    assert_eq!(response["status"], "verified_match");
    assert_eq!(
        response["actual"]["digest_v1_sha256"],
        WAVE55_ROW_090_SHA256
    );
}
