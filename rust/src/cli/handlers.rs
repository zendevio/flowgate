use serde_json::{Map, Value};

use crate::domain::engine::{evaluate_flows, find_next_step, matches_conditions, merge_step_data};
use crate::domain::types::{
    Command, FlowEvaluation, FlowOutput, GetFlowsResponse, Requirements, Response,
    StructuralErrorKind, StructuralErrorResponse, SubmitFailureResponse, SubmitSuccessResponse,
    UserObject,
};
use crate::domain::validation::validate_step_fields;

/// Dispatches a command to the appropriate handler.
pub(crate) fn handle_command(command: Command, requirements: &Requirements) -> Response {
    match command {
        Command::GetFlows { user } => handle_get_flows(user, requirements),
        Command::Submit {
            user,
            flow,
            step,
            data,
        } => handle_submit(user, flow, step, data, requirements),
    }
}

/// Handles the get_flows command: evaluates all flows for the user.
fn handle_get_flows(user: UserObject, requirements: &Requirements) -> Response {
    let user_id = user.id.clone();
    let evals = evaluate_flows(&user, requirements);
    Response::GetFlows(GetFlowsResponse {
        user: user_id,
        flows: to_flow_outputs(evals),
    })
}

/// Handles the submit command with the full error priority chain.
fn handle_submit(
    user: UserObject,
    flow_name: String,
    step_name: String,
    data: Map<String, Value>,
    requirements: &Requirements,
) -> Response {
    let user_id = user.id.clone();

    // Error priority 1: flow_not_found (undefined OR conditions don't match)
    let Some(flow) = requirements.flows.get(&flow_name) else {
        return structural_error(user_id, flow_name, StructuralErrorKind::FlowNotFound);
    };
    if !matches_conditions(&user, &flow.conditions) {
        return structural_error(user_id, flow_name, StructuralErrorKind::FlowNotFound);
    }

    // Error priority 2: step_not_found (step not in THIS flow's step list)
    if !flow.steps.contains(&step_name) {
        return structural_error(user_id, flow_name, StructuralErrorKind::StepNotFound);
    }

    // Error priority 3: not_next_step
    let next_step = find_next_step(&user, flow, requirements);
    let is_next = next_step.as_ref().is_some_and(|ns| ns.name == step_name);
    if !is_next {
        return structural_error(user_id, flow_name, StructuralErrorKind::NotNextStep);
    }

    // Error priority 4: field validation
    let Some(step) = requirements.steps.get(&step_name) else {
        return structural_error(user_id, flow_name, StructuralErrorKind::StepNotFound);
    };

    let errors = validate_step_fields(&step.fields, &data, &requirements.data_types);
    if !errors.is_empty() {
        let evals = evaluate_flows(&user, requirements);
        return Response::SubmitFailure(SubmitFailureResponse {
            user: user_id,
            submission_result: "failure",
            submission_errors: errors,
            flows: to_flow_outputs(evals),
        });
    }

    // Success: filtered merge, then re-evaluate with merged user
    let merged_user = merge_step_data(&user, &data, &step.fields);
    let evals = evaluate_flows(&merged_user, requirements);
    Response::SubmitSuccess(SubmitSuccessResponse {
        user: user_id,
        submission_result: "success",
        flows: to_flow_outputs(evals),
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

/// Converts a vec of domain evaluations into output representations.
fn to_flow_outputs(evals: Vec<FlowEvaluation>) -> Vec<FlowOutput> {
    evals
        .into_iter()
        .map(FlowEvaluation::into_flow_output)
        .collect()
}
