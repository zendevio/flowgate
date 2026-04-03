//! Field type validation, date checking, and requirements integrity verification.

use indexmap::IndexMap;
use serde_json::{Map, Value};

use super::types::{DataType, Requirements, SubmissionError};

/// Validates a single field value against its expected data type.
pub(crate) fn validate_field_type(data_type: DataType, value: &Value) -> bool {
    match data_type {
        DataType::String => value.is_string(),
        DataType::Number => value.is_number(),
        DataType::Date => value.as_str().is_some_and(|s| is_valid_date(s)),
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
/// type as the error rule. Extra data fields are silently ignored.
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
pub(crate) fn validate_requirements_integrity(
    requirements: &Requirements,
) -> Result<(), String> {
    for (step_name, step) in &requirements.steps {
        for field in &step.fields {
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
