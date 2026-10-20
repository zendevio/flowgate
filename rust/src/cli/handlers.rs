use serde_json::{Map, Value};

use crate::domain::engine::{evaluate_flows, find_next_step, matches_conditions, merge_step_data};
use crate::domain::types::{
    Command, CreateUserFailureResponse, CreateUserInput, FlowEvaluation, FlowOutput, FlowStatus,
    GetFlowsResponse, NextStepOutput, Requirements, Response, StructuralErrorKind,
    StructuralErrorResponse, SubmissionError, SubmitFailureResponse, SubmitSuccessResponse,
    UserNotFoundResponse, UserObject, UserStore,
};
use crate::domain::validation::validate_step_fields;

/// Dispatches a command to the appropriate handler.
pub(crate) fn handle_command(
    command: Command,
    requirements: &Requirements,
    store: &mut UserStore,
) -> Response {
    match command {
        Command::CreateUser { user } => handle_create_user(user, store),
        Command::GetFlows { user_id } => handle_get_flows(user_id, requirements, store),
        Command::Submit {
            user_id,
            flow,
            step,
            data,
        } => handle_submit(user_id, flow, step, data, requirements, store),
    }
}

/// Creates a user only after country and uniqueness checks pass.
fn handle_create_user(user: CreateUserInput, store: &mut UserStore) -> Response {
    let Some(country) = user.country.filter(|country| !country.is_empty()) else {
        return create_user_failure("country", "required");
    };
    if store.contains_key(&user.id) {
        return create_user_failure("user_id", "duplicate");
    }

    let mut fields = Map::new();
    fields.insert("id".into(), Value::String(user.id.clone()));
    fields.insert("country".into(), Value::String(country));
    store.insert(
        user.id.clone(),
        UserObject {
            id: user.id.clone(),
            fields,
        },
    );
    success_response(user.id)
}

/// Handles get_flows using the persisted user and omitting completed flows.
fn handle_get_flows(user_id: String, requirements: &Requirements, store: &UserStore) -> Response {
    let Some(user) = store.get(&user_id) else {
        return Response::UserNotFound(UserNotFoundResponse {
            user: user_id,
            error: StructuralErrorKind::UserNotFound,
        });
    };
    Response::GetFlows(GetFlowsResponse {
        user: user.id.clone(),
        flows: to_flow_outputs(evaluate_flows(user, requirements)),
    })
}

/// Handles the submit command with the full error priority chain.
fn handle_submit(
    user_id: String,
    flow_name: String,
    step_name: String,
    data: Map<String, Value>,
    requirements: &Requirements,
    store: &mut UserStore,
) -> Response {
    // Error priority 1: user_not_found
    let Some(user) = store.get(&user_id) else {
        return structural_error(user_id, flow_name, StructuralErrorKind::UserNotFound);
    };

    // Error priority 2: flow_not_found (undefined OR conditions don't match)
    let Some(flow) = requirements.flows.get(&flow_name) else {
        return structural_error(user_id, flow_name, StructuralErrorKind::FlowNotFound);
    };
    if !matches_conditions(user, &flow.conditions) {
        return structural_error(user_id, flow_name, StructuralErrorKind::FlowNotFound);
    }

    // Error priority 3: step_not_found (step not in THIS flow's step list)
    if !flow.steps.contains(&step_name) {
        return structural_error(user_id, flow_name, StructuralErrorKind::StepNotFound);
    }

    // Error priority 4: not_next_step (including partially filled steps)
    let next_step = find_next_step(user, flow, requirements);
    let is_next = next_step.as_ref().is_some_and(|ns| ns.name == step_name);
    if !is_next {
        return structural_error(user_id, flow_name, StructuralErrorKind::NotNextStep);
    }

    // Error priority 5: validate only step fields present in data
    let Some(step) = requirements.steps.get(&step_name) else {
        return structural_error(user_id, flow_name, StructuralErrorKind::StepNotFound);
    };
    let provided_fields: Vec<_> = step
        .fields
        .iter()
        .filter(|field| data.contains_key(*field))
        .cloned()
        .collect();
    let errors = if provided_fields.is_empty() {
        vec![SubmissionError {
            field: "data".into(),
            rule: "required".into(),
        }]
    } else {
        validate_step_fields(&provided_fields, &data, &requirements.data_types)
    };
    if !errors.is_empty() {
        return Response::SubmitFailure(SubmitFailureResponse {
            user: user_id,
            submission_result: "failure",
            submission_errors: errors,
        });
    }

    // Success: merge only step fields and persist after all validation passes
    let merged_user = merge_step_data(user, &data, &step.fields);
    store.insert(user_id.clone(), merged_user);
    success_response(user_id)
}

/// Builds a failed user creation response.
fn create_user_failure(field: &str, rule: &str) -> Response {
    Response::CreateUserFailure(CreateUserFailureResponse {
        submission_result: "failure",
        submission_errors: vec![SubmissionError {
            field: field.into(),
            rule: rule.into(),
        }],
    })
}

/// Builds a successful creation or submission response.
fn success_response(user_id: String) -> Response {
    Response::SubmitSuccess(SubmitSuccessResponse {
        user: user_id,
        submission_result: "success",
    })
}

/// Builds a structural error response.
fn structural_error(user_id: String, flow_name: String, kind: StructuralErrorKind) -> Response {
    Response::StructuralError(StructuralErrorResponse {
        user: user_id,
        flow: flow_name,
        error: kind,
    })
}

/// Converts pending domain evaluations into output representations.
fn to_flow_outputs(evals: Vec<FlowEvaluation>) -> Vec<FlowOutput> {
    evals
        .into_iter()
        .filter_map(|evaluation| {
            evaluation.next_step.map(|next| FlowOutput {
                name: evaluation.name,
                status: FlowStatus::Pending,
                next_step: NextStepOutput {
                    name: next.name,
                    fields: next.fields,
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use serde_json::json;

    use crate::domain::types::{DataType, FlowConfig, StepConfig};

    /// Requirements with one flow gated on the user id instead of country.
    fn id_conditioned_reqs() -> Requirements {
        let mut data_types = IndexMap::new();
        data_types.insert("name".into(), DataType::String);
        let mut steps = IndexMap::new();
        steps.insert(
            "personal".into(),
            StepConfig {
                fields: vec!["name".into()],
            },
        );
        let mut flows = IndexMap::new();
        flows.insert(
            "u1_flow".into(),
            FlowConfig {
                conditions: IndexMap::from([("id".into(), "u1".into())]),
                steps: vec!["personal".into()],
            },
        );
        Requirements {
            data_types,
            steps,
            flows,
        }
    }

    /// Creates a user and returns the serialized get_flows response for it.
    fn flows_for(id: &str, reqs: &Requirements) -> Value {
        let mut store = UserStore::new();
        let create = Command::CreateUser {
            user: CreateUserInput {
                id: id.into(),
                country: Some("DE".into()),
            },
        };
        handle_command(create, reqs, &mut store);
        let get = Command::GetFlows { user_id: id.into() };
        serde_json::to_value(handle_command(get, reqs, &mut store)).unwrap()
    }

    #[test]
    fn create_user_makes_id_available_to_flow_conditions() {
        let reqs = id_conditioned_reqs();
        assert_eq!(
            flows_for("u1", &reqs),
            json!({
                "user": "u1",
                "flows": [{
                    "name": "u1_flow",
                    "status": "pending",
                    "next_step": {"name": "personal", "fields": ["name"]}
                }]
            })
        );
        assert_eq!(flows_for("u2", &reqs), json!({"user": "u2", "flows": []}));
    }
}
