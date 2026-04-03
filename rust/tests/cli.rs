use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{json, Value};

/// Rust crate root directory containing fixture files.
fn crate_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Runs the onboard binary with the given commands as stdin JSON.
fn run(commands: &Value) -> Value {
    let output = Command::cargo_bin("onboard")
        .unwrap()
        .arg(crate_root().join("requirements.json"))
        .write_stdin(serde_json::to_string(commands).unwrap())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

// --- Spec examples ---

#[test]
fn golden_test_produces_output_matching_output_json() {
    let input_path = crate_root().join("input.json");
    let output_path = crate_root().join("output.json");

    let input: Value =
        serde_json::from_str(&std::fs::read_to_string(input_path).unwrap()).unwrap();
    let expected: Value =
        serde_json::from_str(&std::fs::read_to_string(output_path).unwrap()).unwrap();

    let results = run(&input);
    assert_eq!(results, expected);
}

// --- get_flows edge cases ---

#[test]
fn get_flows_empty_flows_for_user_with_no_country() {
    let results = run(&json!([
        { "command": "get_flows", "user": { "id": "u1" } }
    ]));
    assert_eq!(results, json!([{ "user": "u1", "flows": [] }]));
}

#[test]
fn get_flows_complete_flow_when_all_steps_done() {
    let results = run(&json!([{
        "command": "get_flows",
        "user": {
            "id": "u1",
            "country": "US",
            "full_name": "Alice",
            "date_of_birth": "1990-01-01",
            "city": "NYC",
            "passport_number": 12345
        }
    }]));
    assert_eq!(
        results,
        json!([{
            "user": "u1",
            "flows": [{ "name": "us_onboarding", "status": "complete" }]
        }])
    );
}

// --- submit edge cases ---

#[test]
fn submit_not_next_step_when_submitting_wrong_step() {
    let results = run(&json!([{
        "command": "submit",
        "user": { "id": "u1", "country": "US" },
        "flow": "us_onboarding",
        "step": "identity_verification",
        "data": { "passport_number": 12345 }
    }]));
    assert_eq!(
        results,
        json!([{
            "user": "u1",
            "flow": "us_onboarding",
            "error": "not_next_step"
        }])
    );
}

#[test]
fn submit_not_next_step_when_resubmitting_completed_step() {
    let results = run(&json!([{
        "command": "submit",
        "user": {
            "id": "u1",
            "country": "US",
            "full_name": "Alice",
            "date_of_birth": "1990-01-01",
            "city": "NYC"
        },
        "flow": "us_onboarding",
        "step": "personal_information",
        "data": { "full_name": "Bob", "date_of_birth": "1991-01-01", "city": "LA" }
    }]));
    assert_eq!(
        results,
        json!([{
            "user": "u1",
            "flow": "us_onboarding",
            "error": "not_next_step"
        }])
    );
}

#[test]
fn submit_flow_not_found_when_country_mismatch() {
    let results = run(&json!([{
        "command": "submit",
        "user": { "id": "u1", "country": "DE" },
        "flow": "us_onboarding",
        "step": "personal_information",
        "data": {}
    }]));
    assert_eq!(
        results,
        json!([{
            "user": "u1",
            "flow": "us_onboarding",
            "error": "flow_not_found"
        }])
    );
}

#[test]
fn submit_validation_errors_for_missing_required_fields() {
    let results = run(&json!([{
        "command": "submit",
        "user": { "id": "u1", "country": "DE" },
        "flow": "de_onboarding",
        "step": "personal_information",
        "data": {}
    }]));
    assert_eq!(
        results,
        json!([{
            "user": "u1",
            "submission_result": "failure",
            "submission_errors": [
                { "field": "full_name", "rule": "string" },
                { "field": "date_of_birth", "rule": "date" },
                { "field": "city", "rule": "string" }
            ],
            "flows": [{
                "name": "de_onboarding",
                "status": "pending",
                "next_step": {
                    "name": "personal_information",
                    "fields": ["full_name", "date_of_birth", "city"]
                }
            }]
        }])
    );
}

#[test]
fn submit_multiple_validation_errors_in_field_definition_order() {
    let results = run(&json!([{
        "command": "submit",
        "user": { "id": "u1", "country": "DE" },
        "flow": "de_onboarding",
        "step": "personal_information",
        "data": { "full_name": 123, "date_of_birth": "bad", "city": 456 }
    }]));
    let errors = results[0]["submission_errors"].as_array().unwrap();
    assert_eq!(
        errors,
        &vec![
            json!({ "field": "full_name", "rule": "string" }),
            json!({ "field": "date_of_birth", "rule": "date" }),
            json!({ "field": "city", "rule": "string" }),
        ]
    );
}

#[test]
fn submit_rejects_numeric_string_for_number_field() {
    let results = run(&json!([{
        "command": "submit",
        "user": {
            "id": "u1",
            "country": "US",
            "full_name": "Alice",
            "date_of_birth": "1990-01-01",
            "city": "NYC"
        },
        "flow": "us_onboarding",
        "step": "identity_verification",
        "data": { "passport_number": "12345" }
    }]));
    let errors = results[0]["submission_errors"].as_array().unwrap();
    assert_eq!(
        errors,
        &vec![json!({ "field": "passport_number", "rule": "number" })]
    );
}

#[test]
fn submit_ignores_extra_fields_in_data() {
    let results = run(&json!([{
        "command": "submit",
        "user": { "id": "u1", "country": "DE" },
        "flow": "de_onboarding",
        "step": "personal_information",
        "data": {
            "full_name": "Alice",
            "date_of_birth": "1990-01-01",
            "city": "Berlin",
            "extra_field": "ignored"
        }
    }]));
    assert_eq!(results[0]["submission_result"], "success");
}

#[test]
fn submit_rejects_invalid_date_format() {
    let results = run(&json!([{
        "command": "submit",
        "user": { "id": "u1", "country": "DE" },
        "flow": "de_onboarding",
        "step": "personal_information",
        "data": { "full_name": "Alice", "date_of_birth": "04-01-2000", "city": "Berlin" }
    }]));
    let errors = results[0]["submission_errors"].as_array().unwrap();
    assert_eq!(
        errors,
        &vec![json!({ "field": "date_of_birth", "rule": "date" })]
    );
}

#[test]
fn submit_rejects_impossible_calendar_date() {
    let results = run(&json!([{
        "command": "submit",
        "user": { "id": "u1", "country": "DE" },
        "flow": "de_onboarding",
        "step": "personal_information",
        "data": { "full_name": "Alice", "date_of_birth": "2026-02-30", "city": "Berlin" }
    }]));
    let errors = results[0]["submission_errors"].as_array().unwrap();
    assert_eq!(
        errors,
        &vec![json!({ "field": "date_of_birth", "rule": "date" })]
    );
}

#[test]
fn submit_accepts_empty_string_as_valid_string() {
    let results = run(&json!([{
        "command": "submit",
        "user": { "id": "u1", "country": "DE" },
        "flow": "de_onboarding",
        "step": "personal_information",
        "data": { "full_name": "", "date_of_birth": "1990-01-01", "city": "" }
    }]));
    assert_eq!(results[0]["submission_result"], "success");
}

#[test]
fn submit_flow_not_found_for_prototype_pollution_names() {
    let results = run(&json!([
        {
            "command": "submit",
            "user": { "id": "u1", "country": "US" },
            "flow": "__proto__",
            "step": "personal_information",
            "data": {}
        },
        {
            "command": "submit",
            "user": { "id": "u1", "country": "US" },
            "flow": "constructor",
            "step": "personal_information",
            "data": {}
        }
    ]));
    assert_eq!(
        results,
        json!([
            { "user": "u1", "flow": "__proto__", "error": "flow_not_found" },
            { "user": "u1", "flow": "constructor", "error": "flow_not_found" }
        ])
    );
}

#[test]
fn get_flows_null_fields_treated_as_incomplete() {
    let results = run(&json!([{
        "command": "get_flows",
        "user": {
            "id": "u1",
            "country": "DE",
            "full_name": null,
            "date_of_birth": null,
            "city": null
        }
    }]));
    assert_eq!(
        results,
        json!([{
            "user": "u1",
            "flows": [{
                "name": "de_onboarding",
                "status": "pending",
                "next_step": {
                    "name": "personal_information",
                    "fields": ["full_name", "date_of_birth", "city"]
                }
            }]
        }])
    );
}

#[test]
fn submit_completes_multi_step_flow() {
    let results = run(&json!([{
        "command": "submit",
        "user": {
            "id": "u1",
            "country": "US",
            "full_name": "Alice",
            "date_of_birth": "1990-01-01",
            "city": "NYC"
        },
        "flow": "us_onboarding",
        "step": "identity_verification",
        "data": { "passport_number": 12345 }
    }]));
    assert_eq!(
        results,
        json!([{
            "user": "u1",
            "submission_result": "success",
            "flows": [{ "name": "us_onboarding", "status": "complete" }]
        }])
    );
}

// --- empty input ---

#[test]
fn empty_commands_returns_empty_array() {
    let results = run(&json!([]));
    assert_eq!(results, json!([]));
}

// --- error handling ---

#[test]
fn malformed_json_exits_with_code_1() {
    Command::cargo_bin("onboard")
        .unwrap()
        .arg(crate_root().join("requirements.json"))
        .write_stdin("not json")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Failed to parse input JSON"));
}

#[test]
fn missing_requirements_arg_exits_with_code_2() {
    Command::cargo_bin("onboard")
        .unwrap()
        .write_stdin("[]")
        .assert()
        .failure()
        .code(2);
}
