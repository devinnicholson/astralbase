use astralbase::{
    dataset_label::{self, LabelKind, LabelPayload},
    domain::{self, DomainRejectionCode, TerminalStatus},
};

#[test]
fn public_domain_gate_accepts_terminal_checkmate() {
    let validated =
        domain::validate_first_constrained_fen("7k/5KQ1/8/8/8/8/8/8 b - - 0 1").unwrap();

    assert_eq!(validated.terminal_status(), Some(TerminalStatus::Checkmate));
}

#[test]
fn public_domain_gate_rejects_castling_rights() {
    let report =
        domain::validate_first_constrained_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap_err();

    assert_eq!(
        report.reasons()[0].code,
        DomainRejectionCode::CastlingRights
    );
}

#[test]
fn generated_jsonl_keeps_unsupported_positions_out_of_exact_rows() {
    let jsonl = dataset_label::sample_audited_shard_jsonl().unwrap();
    let rows = dataset_label::parse_and_validate_jsonl(&jsonl).unwrap();

    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].label_kind(), LabelKind::Exact);
    assert_eq!(rows[1].label_kind(), LabelKind::Exact);
    assert_eq!(rows[2].label_kind(), LabelKind::Exact);
    assert!(
        rows[3..]
            .iter()
            .all(|row| row.label_kind() == LabelKind::Rejected)
    );

    for row in &rows[3..] {
        let LabelPayload::Rejected { rejected } = &row.label else {
            panic!("unsupported generated rows must be rejected rows");
        };
        assert!(!rejected.reasons.is_empty());
    }
}
