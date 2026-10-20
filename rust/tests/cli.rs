use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use rstest::rstest;
use serde_json::{Value, json};

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

/// Checks values and serialized key order at the expected JSON paths.
fn assert_at(results: &Value, expected: Vec<(&str, Value)>) {
    for (path, value) in expected {
        let actual = results.pointer(path).unwrap();
        assert_eq!(actual, &value);
        assert_eq!(
            serde_json::to_string(actual).unwrap(),
            serde_json::to_string(&value).unwrap()
        );
    }
}

// --- Extension examples ---

#[test]
fn golden_test_produces_output_matching_output_json() {
    let input_path = crate_root().join("input.json");
    let output_path = crate_root().join("output.json");

    let output = Command::cargo_bin("onboard")
        .unwrap()
        .arg(crate_root().join("requirements.json"))
        .write_stdin(std::fs::read(input_path).unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, std::fs::read(output_path).unwrap());
}

// --- create_user ---

#[rstest]
#[case::succeeds_with_valid_id_and_country(
    json!([{"command": "create_user", "user": {"id": "cu1", "country": "DE"}}]),
    vec![
        ("", json!([{"user": "cu1", "submission_result": "success"}])),
    ]
)]
#[case::fails_when_country_is_missing(
    json!([{"command": "create_user", "user": {"id": "cu2"}}]),
    vec![
        ("", json!([
            {
                "submission_result": "failure",
                "submission_errors": [{"field": "country", "rule": "required"}]
            }
        ])),
    ]
)]
#[case::fails_when_user_id_is_duplicate(
    json!([
        {"command": "create_user", "user": {"id": "cu3", "country": "US"}},
        {"command": "create_user", "user": {"id": "cu3", "country": "US"}}
    ]),
    vec![
        ("/1", json!({
            "submission_result": "failure",
            "submission_errors": [{"field": "user_id", "rule": "duplicate"}]
        })),
    ]
)]
fn create_user(#[case] commands: Value, #[case] expected: Vec<(&str, Value)>) {
    let results = run(&commands);
    assert_at(&results, expected);
}

// --- get_flows with user_id ---

#[rstest]
#[case::returns_flows_for_created_user(
    json!([
        {"command": "create_user", "user": {"id": "gf1", "country": "DE"}},
        {"command": "get_flows", "user_id": "gf1"}
    ]),
    vec![
        ("/1", json!({
            "user": "gf1",
            "flows": [
                {
                    "name": "de_onboarding",
                    "status": "pending",
                    "next_step": {
                        "name": "personal_information",
                        "fields": ["full_name", "date_of_birth", "city"]
                    }
                }
            ]
        })),
    ]
)]
#[case::returns_empty_flows_for_user_with_no_matching_country(
    json!([
        {"command": "create_user", "user": {"id": "gf2", "country": "FR"}},
        {"command": "get_flows", "user_id": "gf2"}
    ]),
    vec![
        ("/1", json!({"user": "gf2", "flows": []})),
    ]
)]
#[case::excludes_completed_flows_from_output(
    json!([
        {"command": "create_user", "user": {"id": "gf3", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "gf3",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"full_name": "Alice", "date_of_birth": "1990-01-01", "city": "Berlin"}
        },
        {"command": "get_flows", "user_id": "gf3"}
    ]),
    vec![
        ("/2", json!({"user": "gf3", "flows": []})),
    ]
)]
fn get_flows_with_user_id(#[case] commands: Value, #[case] expected: Vec<(&str, Value)>) {
    let results = run(&commands);
    assert_at(&results, expected);
}

// --- submit with persistence ---

#[rstest]
#[case::persists_data_across_commands(
    json!([
        {"command": "create_user", "user": {"id": "sp1", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "sp1",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"full_name": "Alice"}
        },
        {"command": "get_flows", "user_id": "sp1"}
    ]),
    vec![
        ("/2/flows/0/next_step/fields", json!(["date_of_birth", "city"])),
    ]
)]
#[case::allows_partial_step_submission_and_accumulates_fields(
    json!([
        {"command": "create_user", "user": {"id": "sp2", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "sp2",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"full_name": "Alice", "date_of_birth": "1990-01-01"}
        },
        {
            "command": "submit",
            "user_id": "sp2",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"city": "Berlin"}
        },
        {"command": "get_flows", "user_id": "sp2"}
    ]),
    vec![
        ("/1", json!({"user": "sp2", "submission_result": "success"})),
        ("/2", json!({"user": "sp2", "submission_result": "success"})),
        ("/3", json!({"user": "sp2", "flows": []})),
    ]
)]
#[case::returns_success_without_flows_on_submit(
    json!([
        {"command": "create_user", "user": {"id": "sp3", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "sp3",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"full_name": "Alice", "date_of_birth": "1990-01-01", "city": "Berlin"}
        }
    ]),
    vec![
        ("/1", json!({"user": "sp3", "submission_result": "success"})),
    ]
)]
fn submit_with_persistence(#[case] commands: Value, #[case] expected: Vec<(&str, Value)>) {
    let results = run(&commands);
    assert_at(&results, expected);
}

// --- submit validation ---

#[rstest]
#[case::returns_must_be_prefixed_validation_errors(
    json!([
        {"command": "create_user", "user": {"id": "sv1", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "sv1",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"full_name": 123}
        }
    ]),
    vec![
        ("/1/submission_errors", json!([{"field": "full_name", "rule": "must_be_string"}])),
    ]
)]
#[case::rejects_invalid_date_with_must_be_date_rule(
    json!([
        {"command": "create_user", "user": {"id": "sv2", "country": "US"}},
        {
            "command": "submit",
            "user_id": "sv2",
            "flow": "us_onboarding",
            "step": "personal_information",
            "data": {"date_of_birth": "January 1st 1990"}
        }
    ]),
    vec![
        ("/1/submission_errors", json!([{"field": "date_of_birth", "rule": "must_be_date"}])),
    ]
)]
#[case::rejects_numeric_string_for_number_field(
    json!([
        {"command": "create_user", "user": {"id": "sv3", "country": "US"}},
        {
            "command": "submit",
            "user_id": "sv3",
            "flow": "us_onboarding",
            "step": "personal_information",
            "data": {"full_name": "Alice", "date_of_birth": "1990-01-01", "city": "NYC"}
        },
        {
            "command": "submit",
            "user_id": "sv3",
            "flow": "us_onboarding",
            "step": "identity_verification",
            "data": {"passport_number": "12345"}
        }
    ]),
    vec![
        ("/2/submission_errors", json!([{"field": "passport_number", "rule": "must_be_number"}])),
    ]
)]
#[case::returns_flow_not_found_when_user_country_does_not_match(
    json!([
        {"command": "create_user", "user": {"id": "sv4", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "sv4",
            "flow": "us_onboarding",
            "step": "personal_information",
            "data": {}
        }
    ]),
    vec![
        ("/1", json!({"user": "sv4", "flow": "us_onboarding", "error": "flow_not_found"})),
    ]
)]
#[case::returns_not_next_step_when_submitting_wrong_step(
    json!([
        {"command": "create_user", "user": {"id": "sv5", "country": "US"}},
        {
            "command": "submit",
            "user_id": "sv5",
            "flow": "us_onboarding",
            "step": "identity_verification",
            "data": {"passport_number": 12345}
        }
    ]),
    vec![
        ("/1", json!({"user": "sv5", "flow": "us_onboarding", "error": "not_next_step"})),
    ]
)]
fn submit_validation(#[case] commands: Value, #[case] expected: Vec<(&str, Value)>) {
    let results = run(&commands);
    assert_at(&results, expected);
}

// --- multi-step flow completion ---

#[rstest]
#[case::completes_us_onboarding_across_multiple_submits(
    json!([
        {"command": "create_user", "user": {"id": "ms1", "country": "US"}},
        {
            "command": "submit",
            "user_id": "ms1",
            "flow": "us_onboarding",
            "step": "personal_information",
            "data": {"full_name": "Alice", "date_of_birth": "1990-01-01", "city": "NYC"}
        },
        {
            "command": "submit",
            "user_id": "ms1",
            "flow": "us_onboarding",
            "step": "identity_verification",
            "data": {"passport_number": 12345}
        },
        {"command": "get_flows", "user_id": "ms1"}
    ]),
    vec![
        ("/1", json!({"user": "ms1", "submission_result": "success"})),
        ("/2", json!({"user": "ms1", "submission_result": "success"})),
        ("/3", json!({"user": "ms1", "flows": []})),
    ]
)]
fn multi_step_flow_completion(#[case] commands: Value, #[case] expected: Vec<(&str, Value)>) {
    let results = run(&commands);
    assert_at(&results, expected);
}

// --- empty input ---

#[rstest]
#[case::returns_empty_array_for_empty_commands_array(
    json!([]),
    vec![
        ("", json!([])),
    ]
)]
fn empty_input(#[case] commands: Value, #[case] expected: Vec<(&str, Value)>) {
    let results = run(&commands);
    assert_at(&results, expected);
}

// --- unknown users ---

#[rstest]
#[case::returns_user_not_found_for_get_flows_on_a_user_that_was_never_created(
    json!([{"command": "get_flows", "user_id": "ghost"}]),
    vec![
        ("", json!([{"user": "ghost", "error": "user_not_found"}])),
    ]
)]
#[case::returns_user_not_found_before_any_flow_check_on_submit(
    json!([
        {
            "command": "submit",
            "user_id": "ghost",
            "flow": "no_such_flow",
            "step": "personal_information",
            "data": {"full_name": "Alice"}
        }
    ]),
    vec![
        ("", json!([{"user": "ghost", "flow": "no_such_flow", "error": "user_not_found"}])),
    ]
)]
#[case::does_not_create_a_user_from_a_failed_create_user(
    json!([{"command": "create_user", "user": {"id": "nu1"}}, {"command": "get_flows", "user_id": "nu1"}]),
    vec![
        ("/1", json!({"user": "nu1", "error": "user_not_found"})),
    ]
)]
fn unknown_users(#[case] commands: Value, #[case] expected: Vec<(&str, Value)>) {
    let results = run(&commands);
    assert_at(&results, expected);
}

// --- create_user edge cases ---

#[rstest]
#[case::treats_an_empty_country_as_missing(
    json!([{"command": "create_user", "user": {"id": "ce1", "country": ""}}]),
    vec![
        ("", json!([
            {
                "submission_result": "failure",
                "submission_errors": [{"field": "country", "rule": "required"}]
            }
        ])),
    ]
)]
#[case::keeps_processing_the_batch_after_a_failed_create_user(
    json!([
        {"command": "create_user", "user": {"id": "ce2"}},
        {"command": "create_user", "user": {"id": "ce2", "country": "DE"}}
    ]),
    vec![
        ("/1", json!({"user": "ce2", "submission_result": "success"})),
    ]
)]
#[case::leaves_the_stored_user_untouched_on_a_duplicate_create_user(
    json!([
        {"command": "create_user", "user": {"id": "ce3", "country": "DE"}},
        {"command": "submit", "user_id": "ce3", "flow": "de_onboarding", "step": "personal_information", "data": {"full_name": "Alice"}},
        {"command": "create_user", "user": {"id": "ce3", "country": "US"}},
        {"command": "get_flows", "user_id": "ce3"}
    ]),
    vec![
        ("/3", json!({"user": "ce3", "flows": [{
            "name": "de_onboarding",
            "status": "pending",
            "next_step": {"name": "personal_information", "fields": ["date_of_birth", "city"]}
        }]})),
    ]
)]
#[case::treats_a_null_country_as_missing_without_aborting_the_batch(
    json!([
        {"command": "create_user", "user": {"id": "ce4", "country": null}},
        {"command": "get_flows", "user_id": "ce4"}
    ]),
    vec![
        ("", json!([
            {"submission_result": "failure", "submission_errors": [{"field": "country", "rule": "required"}]},
            {"user": "ce4", "error": "user_not_found"}
        ])),
    ]
)]
fn create_user_edge_cases(#[case] commands: Value, #[case] expected: Vec<(&str, Value)>) {
    let results = run(&commands);
    assert_at(&results, expected);
}

// --- submit edge cases ---

#[rstest]
#[case::rejects_a_submission_with_no_fields_for_the_step(
    json!([
        {"command": "create_user", "user": {"id": "se1", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "se1",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {}
        }
    ]),
    vec![
        ("/1", json!({
            "user": "se1",
            "submission_result": "failure",
            "submission_errors": [{"field": "data", "rule": "required"}]
        })),
    ]
)]
#[case::rejects_a_submission_that_only_carries_fields_outside_the_step(
    json!([
        {"command": "create_user", "user": {"id": "se2", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "se2",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"passport_number": 123, "extra": "x"}
        }
    ]),
    vec![
        ("/1", json!({
            "user": "se2",
            "submission_result": "failure",
            "submission_errors": [{"field": "data", "rule": "required"}]
        })),
    ]
)]
#[case::ignores_extra_fields_alongside_valid_step_fields(
    json!([
        {"command": "create_user", "user": {"id": "se3", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "se3",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"full_name": "Alice", "extra_field": "ignored"}
        }
    ]),
    vec![
        ("/1", json!({"user": "se3", "submission_result": "success"})),
    ]
)]
#[case::stores_nothing_when_any_provided_field_is_invalid(
    json!([
        {"command": "create_user", "user": {"id": "se4", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "se4",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"full_name": "Alice", "date_of_birth": "bad"}
        },
        {"command": "get_flows", "user_id": "se4"}
    ]),
    vec![
        ("/2/flows/0/next_step/fields", json!(["full_name", "date_of_birth", "city"])),
    ]
)]
#[case::returns_multiple_validation_errors_in_step_field_definition_order(
    json!([
        {"command": "create_user", "user": {"id": "se5", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "se5",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"city": 456, "full_name": 123, "date_of_birth": "bad"}
        }
    ]),
    vec![
        ("/1/submission_errors", json!([
            {"field": "full_name", "rule": "must_be_string"},
            {"field": "date_of_birth", "rule": "must_be_date"},
            {"field": "city", "rule": "must_be_string"}
        ])),
    ]
)]
#[case::rejects_null_for_a_provided_field(
    json!([
        {"command": "create_user", "user": {"id": "se6", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "se6",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"full_name": null}
        }
    ]),
    vec![
        ("/1/submission_errors", json!([{"field": "full_name", "rule": "must_be_string"}])),
    ]
)]
#[case::rejects_an_impossible_calendar_date(
    json!([
        {"command": "create_user", "user": {"id": "se7", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "se7",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"date_of_birth": "2026-02-30"}
        }
    ]),
    vec![
        ("/1/submission_errors", json!([{"field": "date_of_birth", "rule": "must_be_date"}])),
    ]
)]
#[case::accepts_an_empty_string_as_a_valid_string_field(
    json!([
        {"command": "create_user", "user": {"id": "se8", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "se8",
            "flow": "de_onboarding",
            "step": "personal_information",
            "data": {"full_name": "", "city": ""}
        }
    ]),
    vec![
        ("/1", json!({"user": "se8", "submission_result": "success"})),
    ]
)]
#[case::returns_not_next_step_when_resubmitting_a_completed_step(
    json!([
        {"command": "create_user", "user": {"id": "se9", "country": "US"}},
        {
            "command": "submit",
            "user_id": "se9",
            "flow": "us_onboarding",
            "step": "personal_information",
            "data": {"full_name": "Alice", "date_of_birth": "1990-01-01", "city": "NYC"}
        },
        {
            "command": "submit",
            "user_id": "se9",
            "flow": "us_onboarding",
            "step": "personal_information",
            "data": {"full_name": "Bob"}
        }
    ]),
    vec![
        ("/2", json!({"user": "se9", "flow": "us_onboarding", "error": "not_next_step"})),
    ]
)]
#[case::returns_step_not_found_for_a_step_outside_the_flow(
    json!([
        {"command": "create_user", "user": {"id": "se10", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "se10",
            "flow": "de_onboarding",
            "step": "identity_verification",
            "data": {"passport_number": 123}
        }
    ]),
    vec![
        ("/1", json!({"user": "se10", "flow": "de_onboarding", "error": "step_not_found"})),
    ]
)]
#[case::returns_flow_not_found_for_prototype_pollution_flow_names(
    json!([
        {"command": "create_user", "user": {"id": "se11", "country": "DE"}},
        {
            "command": "submit",
            "user_id": "se11",
            "flow": "__proto__",
            "step": "personal_information",
            "data": {"full_name": "A"}
        },
        {
            "command": "submit",
            "user_id": "se11",
            "flow": "constructor",
            "step": "personal_information",
            "data": {"full_name": "A"}
        }
    ]),
    vec![
        ("/1", json!({"user": "se11", "flow": "__proto__", "error": "flow_not_found"})),
        ("/2", json!({"user": "se11", "flow": "constructor", "error": "flow_not_found"})),
    ]
)]
fn submit_edge_cases(#[case] commands: Value, #[case] expected: Vec<(&str, Value)>) {
    let results = run(&commands);
    assert_at(&results, expected);
}

// --- error handling ---

#[rstest]
#[case::duplicate_user_id(r#"[{"command":"get_flows","user_id":"first","user_id":"last"}]"#, json!([{"user": "last", "error": "user_not_found"}]))]
#[case::duplicate_command(r#"[{"command":"submit","command":"get_flows","user_id":"last"}]"#, json!([{"user": "last", "error": "user_not_found"}]))]
#[case::duplicate_country(r#"[{"command":"create_user","user":{"id":"u1","country":"","country":"DE"}}]"#, json!([{"user": "u1", "submission_result": "success"}]))]
fn duplicate_keys_keep_the_last_value(#[case] input: &str, #[case] expected: Value) {
    let output = Command::cargo_bin("onboard")
        .unwrap()
        .arg(crate_root().join("requirements.json"))
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(output.status.success());
    let results: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_at(&results, vec![("", expected)]);
}

#[test]
fn invalid_element_aborts_the_batch_before_dispatch() {
    Command::cargo_bin("onboard")
        .unwrap()
        .arg(crate_root().join("requirements.json"))
        .write_stdin(r#"[{"command":"create_user","user":{"id":"u1","country":"DE"}},{"command":"get_flows","user_id":42}]"#)
        .assert()
        .code(1)
        .stdout("")
        .stderr("Error: Failed to parse input JSON from stdin\n");
}

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
