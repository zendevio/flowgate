//! Flow evaluation, condition matching, step completion, and data merging.

use indexmap::IndexMap;
use serde_json::{Map, Value};

use super::types::{
    FlowConfig, FlowEvaluation, NextStep, Requirements, UserObject,
};

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
pub(crate) fn matches_conditions(
    user: &UserObject,
    conditions: &IndexMap<String, String>,
) -> bool {
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
    step.fields.iter().all(|field| {
        matches!(user.get_field(field), Some(v) if !v.is_null())
    })
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
                fields: step.fields.clone(),
            });
        }
    }
    None
}

/// Creates a new user with filtered step data merged in.
/// Only fields defined in the step are copied from data; extra fields are ignored.
/// The user's id is preserved (it lives outside the fields map).
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
