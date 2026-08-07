use super::*;

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
