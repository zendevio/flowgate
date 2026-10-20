use indexmap::IndexMap;
use serde_json::{Map, Value};

use super::types::{DataType, Requirements, SubmissionError};

/// Validates a single field value against its expected data type.
pub(crate) fn validate_field_type(data_type: DataType, value: &Value) -> bool {
    match data_type {
        DataType::String => value.is_string(),
        DataType::Number => value.is_number(),
        DataType::Date => value.as_str().is_some_and(is_valid_date),
    }
}

/// Validates a date string in YYYY-MM-DD format with full calendar checking.
/// No regex crate needed; the format is fixed.
pub(crate) fn is_valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 {
        return false;
    }
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    if !bytes[..4].iter().all(u8::is_ascii_digit) {
        return false;
    }
    if !bytes[5..7].iter().all(u8::is_ascii_digit) {
        return false;
    }
    if !bytes[8..10].iter().all(u8::is_ascii_digit) {
        return false;
    }

    let Ok(year) = value[..4].parse::<u32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u32>() else {
        return false;
    };

    if !(1..=12).contains(&month) {
        return false;
    }
    if day < 1 {
        return false;
    }

    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => return false,
    };

    day <= max_day
}

/// Checks whether a year is a leap year using the Gregorian calendar rules.
fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Validates submitted data fields against expected types, iterating in step
/// field definition order. Missing fields fail validation with the expected
/// must_be_<type> as the error rule. Extra data fields are silently ignored.
pub(crate) fn validate_step_fields(
    step_fields: &[String],
    data: &Map<String, Value>,
    data_types: &IndexMap<String, DataType>,
) -> Vec<SubmissionError> {
    let mut errors = Vec::new();

    for field in step_fields {
        let Some(&expected_type) = data_types.get(field) else {
            continue;
        };
        let value = data.get(field.as_str());
        let valid = value.is_some_and(|v| validate_field_type(expected_type, v));
        if !valid {
            errors.push(SubmissionError {
                field: field.clone(),
                rule: expected_type.as_rule_str().to_owned(),
            });
        }
    }

    errors
}

/// Cross-checks referential integrity of requirements: step fields must exist
/// in data_types, and flow steps must exist in the steps map.
pub(crate) fn validate_requirements_integrity(requirements: &Requirements) -> Result<(), String> {
    for (step_name, step) in &requirements.steps {
        for field in &step.fields {
            // The user id is identity, not step data: a submit must never overwrite it
            if field == "id" {
                return Err(format!("Step '{}' uses reserved field 'id'", step_name));
            }
            if !requirements.data_types.contains_key(field) {
                return Err(format!(
                    "Step '{}' references field '{}' not defined in data_types",
                    step_name, field
                ));
            }
        }
    }

    for (flow_name, flow) in &requirements.flows {
        for step_name in &flow.steps {
            if !requirements.steps.contains_key(step_name) {
                return Err(format!(
                    "Flow '{}' references step '{}' not defined in steps",
                    flow_name, step_name
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    use crate::domain::types::StepConfig;

    // --- is_valid_date ---

    #[rstest]
    #[case("2026-01-15", true)]
    #[case("2024-02-29", true)]
    #[case("0000-02-29", true)]
    #[case("1900-02-29", false)]
    #[case("2000-01-01", true)]
    #[case("1985-12-12", true)]
    #[case("01-15-2026", false)]
    #[case("2026/01/15", false)]
    #[case("2026-1-5", false)]
    #[case("2026-01", false)]
    #[case("not-a-date", false)]
    #[case("", false)]
    #[case("2025-02-29", false)]
    #[case("2026-02-30", false)]
    #[case("2026-04-31", false)]
    #[case("2026-13-01", false)]
    #[case("2026-00-15", false)]
    #[case("2026-01-00", false)]
    #[case("2026-01-32", false)]
    fn date_validation(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(
            is_valid_date(input),
            expected,
            "is_valid_date({input:?}) should be {expected}"
        );
    }

    // --- validate_field_type ---

    #[rstest]
    #[case(DataType::String, json!("hello"), true)]
    #[case(DataType::String, json!(""), true)]
    #[case(DataType::String, json!(42), false)]
    #[case(DataType::String, Value::Null, false)]
    #[case(DataType::String, json!(true), false)]
    #[case(DataType::Number, json!(42), true)]
    #[case(DataType::Number, json!(0), true)]
    #[case(DataType::Number, json!(-5), true)]
    #[case(DataType::Number, json!(3.5), true)]
    #[case(DataType::Number, json!("42"), false)]
    #[case(DataType::Number, json!(true), false)]
    #[case(DataType::Number, Value::Null, false)]
    #[case(DataType::Date, json!("2026-01-15"), true)]
    #[case(DataType::Date, json!("2024-02-29"), true)]
    #[case(DataType::Date, json!("2025-02-29"), false)]
    #[case(DataType::Date, json!("2026-02-30"), false)]
    #[case(DataType::Date, json!("01-15-2026"), false)]
    #[case(DataType::Date, json!(12345), false)]
    #[case(DataType::Date, Value::Null, false)]
    #[case(DataType::String, json!([1, 2]), false)]
    #[case(DataType::String, json!({"a": 1}), false)]
    #[case(DataType::Number, json!([1]), false)]
    #[case(DataType::Date, json!({}), false)]
    fn field_type_validation(
        #[case] data_type: DataType,
        #[case] value: Value,
        #[case] expected: bool,
    ) {
        assert_eq!(
            validate_field_type(data_type, &value),
            expected,
            "validate_field_type({data_type:?}, {value}) should be {expected}"
        );
    }

    // --- validate_step_fields ---

    #[test]
    fn step_fields_valid_returns_empty() {
        let mut dt = IndexMap::new();
        dt.insert("name".into(), DataType::String);
        let mut data = Map::new();
        data.insert("name".into(), json!("Alice"));

        let errors = validate_step_fields(&["name".into()], &data, &dt);
        assert!(errors.is_empty());
    }

    #[test]
    fn step_fields_invalid_returns_errors_in_field_order() {
        let mut dt = IndexMap::new();
        dt.insert("age".into(), DataType::Number);
        dt.insert("name".into(), DataType::String);
        let mut data = Map::new();
        data.insert("age".into(), json!("not-a-number"));
        data.insert("name".into(), json!(42));

        let errors = validate_step_fields(&["age".into(), "name".into()], &data, &dt);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].field, "age");
        assert_eq!(errors[0].rule, "must_be_number");
        assert_eq!(errors[1].field, "name");
        assert_eq!(errors[1].rule, "must_be_string");
    }

    #[test]
    fn step_fields_missing_returns_error_with_type_rule() {
        let mut dt = IndexMap::new();
        dt.insert("dob".into(), DataType::Date);
        let data = Map::new();

        let errors = validate_step_fields(&["dob".into()], &data, &dt);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "dob");
        assert_eq!(errors[0].rule, "must_be_date");
    }

    #[test]
    fn step_fields_ignores_extra_data_fields() {
        let mut dt = IndexMap::new();
        dt.insert("name".into(), DataType::String);
        let mut data = Map::new();
        data.insert("name".into(), json!("Alice"));
        data.insert("extra".into(), json!("ignored"));

        let errors = validate_step_fields(&["name".into()], &data, &dt);
        assert!(errors.is_empty());
    }

    #[test]
    fn step_fields_multiple_failures() {
        let mut dt = IndexMap::new();
        dt.insert("a".into(), DataType::String);
        dt.insert("b".into(), DataType::String);
        dt.insert("c".into(), DataType::Date);
        let mut data = Map::new();
        data.insert("a".into(), json!(1));
        data.insert("b".into(), json!(2));
        data.insert("c".into(), json!(3));

        let errors = validate_step_fields(&["a".into(), "b".into(), "c".into()], &data, &dt);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn step_fields_numeric_string_rejected_for_number() {
        let mut dt = IndexMap::new();
        dt.insert("count".into(), DataType::Number);
        let mut data = Map::new();
        data.insert("count".into(), json!("5"));

        let errors = validate_step_fields(&["count".into()], &data, &dt);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "count");
        assert_eq!(errors[0].rule, "must_be_number");
    }

    // --- validate_requirements_integrity ---

    #[test]
    fn integrity_passes_for_valid_requirements() {
        let mut dt = IndexMap::new();
        dt.insert("name".into(), DataType::String);
        let mut steps = IndexMap::new();
        steps.insert(
            "identity".into(),
            StepConfig {
                fields: vec!["name".into()],
            },
        );
        let mut flows = IndexMap::new();
        flows.insert(
            "kyc".into(),
            super::super::types::FlowConfig {
                conditions: IndexMap::new(),
                steps: vec!["identity".into()],
            },
        );
        let reqs = Requirements {
            data_types: dt,
            steps,
            flows,
        };
        assert!(validate_requirements_integrity(&reqs).is_ok());
    }

    #[test]
    fn integrity_fails_when_step_field_not_in_data_types() {
        let dt = IndexMap::new();
        let mut steps = IndexMap::new();
        steps.insert(
            "identity".into(),
            StepConfig {
                fields: vec!["name".into()],
            },
        );
        let reqs = Requirements {
            data_types: dt,
            steps,
            flows: IndexMap::new(),
        };
        let err = validate_requirements_integrity(&reqs).unwrap_err();
        assert!(
            err.contains("Step 'identity' references field 'name' not defined in data_types"),
            "Unexpected error: {err}"
        );
    }

    #[test]
    fn integrity_fails_when_step_uses_reserved_id_field() {
        let mut dt = IndexMap::new();
        dt.insert("id".into(), DataType::String);
        let mut steps = IndexMap::new();
        steps.insert(
            "identity".into(),
            StepConfig {
                fields: vec!["id".into()],
            },
        );
        let reqs = Requirements {
            data_types: dt,
            steps,
            flows: IndexMap::new(),
        };
        let err = validate_requirements_integrity(&reqs).unwrap_err();
        assert_eq!(err, "Step 'identity' uses reserved field 'id'");
    }

    #[test]
    fn integrity_fails_when_flow_step_not_in_steps() {
        let mut flows = IndexMap::new();
        flows.insert(
            "kyc".into(),
            super::super::types::FlowConfig {
                conditions: IndexMap::new(),
                steps: vec!["missing_step".into()],
            },
        );
        let reqs = Requirements {
            data_types: IndexMap::new(),
            steps: IndexMap::new(),
            flows,
        };
        let err = validate_requirements_integrity(&reqs).unwrap_err();
        assert!(
            err.contains("Flow 'kyc' references step 'missing_step' not defined in steps"),
            "Unexpected error: {err}"
        );
    }
}
