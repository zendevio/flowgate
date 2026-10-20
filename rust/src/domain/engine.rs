use indexmap::IndexMap;
use serde_json::{Map, Value};

use super::types::{FlowConfig, FlowEvaluation, NextStep, Requirements, UserObject};

/// Evaluates all flows against a user, returning matching flows in definition order.
pub(crate) fn evaluate_flows(
    user: &UserObject,
    requirements: &Requirements,
) -> Vec<FlowEvaluation> {
    let mut results = Vec::new();

    for (name, flow) in &requirements.flows {
        if !matches_conditions(user, &flow.conditions) {
            continue;
        }
        let next_step = find_next_step(user, flow, requirements);
        results.push(FlowEvaluation {
            name: name.clone(),
            next_step,
        });
    }

    results
}

/// Checks whether a user satisfies all flow conditions (AND semantics).
pub(crate) fn matches_conditions(user: &UserObject, conditions: &IndexMap<String, String>) -> bool {
    conditions.iter().all(|(key, value)| {
        user.get_field(key)
            .and_then(Value::as_str)
            .is_some_and(|s| s == value)
    })
}

/// Checks whether a user has completed all fields for a given step.
/// Both missing fields and null values are treated as incomplete.
pub(crate) fn is_step_complete(
    user: &UserObject,
    step_name: &str,
    requirements: &Requirements,
) -> bool {
    let Some(step) = requirements.steps.get(step_name) else {
        return false;
    };
    step.fields
        .iter()
        .all(|field| matches!(user.get_field(field), Some(v) if !v.is_null()))
}

/// Finds the first incomplete step in a flow, returning its name and fields.
/// Returns None when all steps are complete.
pub(crate) fn find_next_step(
    user: &UserObject,
    flow: &FlowConfig,
    requirements: &Requirements,
) -> Option<NextStep> {
    for step_name in &flow.steps {
        if !is_step_complete(user, step_name, requirements) {
            let step = requirements.steps.get(step_name)?;
            return Some(NextStep {
                name: step_name.clone(),
                fields: step
                    .fields
                    .iter()
                    .filter(|field| user.get_field(field).is_none_or(Value::is_null))
                    .cloned()
                    .collect(),
            });
        }
    }
    None
}

/// Creates a new user with filtered step data merged in.
/// Only fields defined in the step are copied from data; extra fields are ignored.
/// Existing user fields are preserved unless supplied as step fields.
pub(crate) fn merge_step_data(
    user: &UserObject,
    data: &Map<String, Value>,
    step_fields: &[String],
) -> UserObject {
    let mut merged = user.clone();
    for field in step_fields {
        if let Some(value) = data.get(field) {
            merged.fields.insert(field.clone(), value.clone());
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    use crate::domain::types::{DataType, StepConfig};

    fn make_reqs() -> Requirements {
        let mut data_types = IndexMap::new();
        data_types.insert("name".into(), DataType::String);
        data_types.insert("age".into(), DataType::Number);
        data_types.insert("dob".into(), DataType::Date);

        let mut steps = IndexMap::new();
        steps.insert(
            "personal".into(),
            StepConfig {
                fields: vec!["name".into(), "age".into()],
            },
        );
        steps.insert(
            "identity".into(),
            StepConfig {
                fields: vec!["dob".into()],
            },
        );

        let mut conditions = IndexMap::new();
        conditions.insert("country".into(), "GB".into());

        let mut flows = IndexMap::new();
        flows.insert(
            "gb_flow".into(),
            FlowConfig {
                conditions,
                steps: vec!["personal".into(), "identity".into()],
            },
        );

        Requirements {
            data_types,
            steps,
            flows,
        }
    }

    fn make_user(fields: &[(&str, Value)]) -> UserObject {
        let mut map = Map::new();
        map.insert("id".into(), json!("u1"));
        for (k, v) in fields {
            map.insert((*k).to_owned(), v.clone());
        }
        UserObject {
            id: "u1".into(),
            fields: map,
        }
    }

    // --- matches_conditions ---

    #[test]
    fn matches_conditions_returns_true_when_all_match() {
        let user = make_user(&[("country", json!("GB"))]);
        let mut conds = IndexMap::new();
        conds.insert("country".into(), "GB".into());
        assert!(matches_conditions(&user, &conds));
    }

    #[test]
    fn matches_conditions_returns_false_when_value_differs() {
        let user = make_user(&[("country", json!("US"))]);
        let mut conds = IndexMap::new();
        conds.insert("country".into(), "GB".into());
        assert!(!matches_conditions(&user, &conds));
    }

    #[test]
    fn matches_conditions_returns_false_when_key_missing() {
        let user = make_user(&[]);
        let mut conds = IndexMap::new();
        conds.insert("country".into(), "GB".into());
        assert!(!matches_conditions(&user, &conds));
    }

    #[test]
    fn matches_conditions_returns_true_for_empty_conditions() {
        let user = make_user(&[]);
        let conds = IndexMap::new();
        assert!(matches_conditions(&user, &conds));
    }

    // --- is_step_complete ---

    #[test]
    fn step_complete_when_user_has_all_fields() {
        let user = make_user(&[("name", json!("Alice")), ("age", json!(30))]);
        assert!(is_step_complete(&user, "personal", &make_reqs()));
    }

    #[test]
    fn step_incomplete_when_field_missing() {
        let user = make_user(&[("name", json!("Alice"))]);
        assert!(!is_step_complete(&user, "personal", &make_reqs()));
    }

    #[test]
    fn step_incomplete_when_constructor_is_missing() {
        let mut reqs = make_reqs();
        reqs.data_types
            .insert("constructor".into(), DataType::String);
        reqs.steps.insert(
            "proto".into(),
            StepConfig {
                fields: vec!["constructor".into()],
            },
        );
        assert!(!is_step_complete(&make_user(&[]), "proto", &reqs));
    }

    #[test]
    fn id_is_available_for_conditions_and_step_fields() {
        let user = make_user(&[]);
        let mut conditions = IndexMap::new();
        conditions.insert("id".into(), "u1".into());
        assert!(matches_conditions(&user, &conditions));
        conditions.insert("id".into(), "other".into());
        assert!(!matches_conditions(&user, &conditions));
        let mut reqs = make_reqs();
        reqs.steps.insert(
            "identity".into(),
            StepConfig {
                fields: vec!["id".into()],
            },
        );
        assert!(is_step_complete(&user, "identity", &reqs));
        let flow = FlowConfig {
            conditions: IndexMap::new(),
            steps: vec!["identity".into(), "personal".into()],
        };
        assert_eq!(
            find_next_step(&user, &flow, &reqs).unwrap().name,
            "personal"
        );
    }

    #[test]
    fn step_incomplete_for_unknown_step() {
        let user = make_user(&[]);
        assert!(!is_step_complete(&user, "nonexistent", &make_reqs()));
    }

    #[test]
    fn step_incomplete_when_field_is_null() {
        let user = make_user(&[("name", Value::Null), ("age", json!(30))]);
        assert!(!is_step_complete(&user, "personal", &make_reqs()));
    }

    // --- find_next_step ---

    #[test]
    fn find_next_step_returns_first_incomplete() {
        let user = make_user(&[]);
        let reqs = make_reqs();
        let flow = reqs.flows.get("gb_flow").unwrap();
        let result = find_next_step(&user, flow, &reqs).unwrap();
        assert_eq!(result.name, "personal");
        assert_eq!(result.fields, vec!["name", "age"]);
    }

    #[test]
    fn find_next_step_returns_none_when_all_complete() {
        let user = make_user(&[
            ("name", json!("Alice")),
            ("age", json!(30)),
            ("dob", json!("1990-01-01")),
        ]);
        let reqs = make_reqs();
        let flow = reqs.flows.get("gb_flow").unwrap();
        assert!(find_next_step(&user, flow, &reqs).is_none());
    }

    #[test]
    fn find_next_step_skips_completed_steps() {
        let user = make_user(&[("name", json!("Alice")), ("age", json!(30))]);
        let reqs = make_reqs();
        let flow = reqs.flows.get("gb_flow").unwrap();
        let result = find_next_step(&user, flow, &reqs).unwrap();
        assert_eq!(result.name, "identity");
        assert_eq!(result.fields, vec!["dob"]);
    }

    #[rstest]
    #[case(json!({"name": "Alice"}), vec!["age"])]
    #[case(json!({"name": null, "age": 30}), vec!["name"])]
    #[case(json!({"name": "", "age": null}), vec!["age"])]
    fn find_next_step_returns_only_remaining_fields(
        #[case] fields: Value,
        #[case] expected: Vec<&str>,
    ) {
        let fields: Vec<_> = fields
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.as_str(), value.clone()))
            .collect();
        let user = make_user(&fields);
        let reqs = make_reqs();
        let flow = reqs.flows.get("gb_flow").unwrap();
        let next = find_next_step(&user, flow, &reqs).unwrap();
        assert_eq!(next.name, "personal");
        assert_eq!(next.fields, expected);
    }

    // --- merge_step_data ---

    #[test]
    fn merge_step_data_merges_only_step_fields() {
        let user = make_user(&[("country", json!("GB"))]);
        let mut data = Map::new();
        data.insert("name".into(), json!("Alice"));
        data.insert("age".into(), json!(30));

        let merged = merge_step_data(&user, &data, &["name".into(), "age".into()]);
        assert_eq!(merged.id, "u1");
        assert_eq!(merged.fields.get("country").unwrap(), "GB");
        assert_eq!(merged.fields.get("name").unwrap(), "Alice");
        assert_eq!(merged.fields.get("age").unwrap(), 30);
    }

    #[test]
    fn merge_step_data_ignores_extra_data_fields() {
        let user = make_user(&[]);
        let mut data = Map::new();
        data.insert("name".into(), json!("Alice"));
        data.insert("extra".into(), json!("ignored"));

        let merged = merge_step_data(&user, &data, &["name".into()]);
        assert!(merged.fields.get("extra").is_none());
        assert_eq!(merged.fields.get("name").unwrap(), "Alice");
    }

    #[test]
    fn merge_step_data_preserves_existing_user_fields() {
        let user = make_user(&[("country", json!("GB")), ("name", json!("Old"))]);
        let mut data = Map::new();
        data.insert("name".into(), json!("New"));

        let merged = merge_step_data(&user, &data, &["name".into()]);
        assert_eq!(merged.fields.get("country").unwrap(), "GB");
        assert_eq!(merged.fields.get("name").unwrap(), "New");
    }

    #[test]
    fn merge_step_data_does_not_overwrite_id() {
        let user = make_user(&[]);
        let mut data = Map::new();
        data.insert("id".into(), json!("hacker"));
        data.insert("name".into(), json!("Alice"));

        let merged = merge_step_data(&user, &data, &["name".into()]);
        assert_eq!(merged.fields.get("id").unwrap(), "u1");
        assert_eq!(merged.fields.get("name").unwrap(), "Alice");
    }

    // --- evaluate_flows ---

    #[test]
    fn evaluate_flows_returns_matching_flows() {
        let user = make_user(&[("country", json!("GB"))]);
        let results = evaluate_flows(&user, &make_reqs());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "gb_flow");
        let ns = results[0].next_step.as_ref().unwrap();
        assert_eq!(ns.name, "personal");
        assert_eq!(ns.fields, vec!["name", "age"]);
    }

    #[test]
    fn evaluate_flows_skips_non_matching() {
        let user = make_user(&[("country", json!("US"))]);
        assert!(evaluate_flows(&user, &make_reqs()).is_empty());
    }

    #[test]
    fn evaluate_flows_empty_for_no_matching_country() {
        let user = make_user(&[("country", json!("FR"))]);
        assert!(evaluate_flows(&user, &make_reqs()).is_empty());
    }

    #[test]
    fn evaluate_flows_complete_when_all_steps_done() {
        let user = make_user(&[
            ("country", json!("GB")),
            ("name", json!("Alice")),
            ("age", json!(30)),
            ("dob", json!("1990-01-01")),
        ]);
        let results = evaluate_flows(&user, &make_reqs());
        assert_eq!(results.len(), 1);
        assert!(results[0].next_step.is_none());
    }

    #[test]
    fn evaluate_flows_treats_null_field_as_incomplete() {
        let user = make_user(&[
            ("country", json!("GB")),
            ("name", Value::Null),
            ("age", json!(30)),
        ]);
        let results = evaluate_flows(&user, &make_reqs());
        let ns = results[0].next_step.as_ref().unwrap();
        assert_eq!(ns.name, "personal");
        assert_eq!(ns.fields, vec!["name"]);
    }

    #[test]
    fn evaluate_flows_preserves_definition_order() {
        let mut flows = IndexMap::new();
        flows.insert(
            "gb_flow".into(),
            FlowConfig {
                conditions: {
                    let mut c = IndexMap::new();
                    c.insert("country".into(), "GB".into());
                    c
                },
                steps: vec!["personal".into()],
            },
        );
        flows.insert(
            "gb_flow_2".into(),
            FlowConfig {
                conditions: {
                    let mut c = IndexMap::new();
                    c.insert("country".into(), "GB".into());
                    c
                },
                steps: vec!["identity".into()],
            },
        );
        let mut reqs = make_reqs();
        reqs.flows = flows;

        let user = make_user(&[("country", json!("GB"))]);
        let evals = evaluate_flows(&user, &reqs);
        let names: Vec<&str> = evals.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["gb_flow", "gb_flow_2"]);
    }
}
