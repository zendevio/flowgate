use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

// --- Requirements (deserialized from requirements.json) ---

/// Supported field data types for validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DataType {
    String,
    Number,
    Date,
}

impl DataType {
    /// Returns the string representation used in submission error rules.
    pub(crate) fn as_rule_str(&self) -> &'static str {
        match self {
            DataType::String => "must_be_string",
            DataType::Number => "must_be_number",
            DataType::Date => "must_be_date",
        }
    }
}

/// Configuration for a single onboarding step.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StepConfig {
    pub fields: Vec<String>,
}

/// Configuration for an onboarding flow with conditions and ordered steps.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FlowConfig {
    pub conditions: IndexMap<String, String>,
    pub steps: Vec<String>,
}

/// Top-level requirements defining data types, steps, and flows.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Requirements {
    pub data_types: IndexMap<String, DataType>,
    pub steps: IndexMap<String, StepConfig>,
    pub flows: IndexMap<String, FlowConfig>,
}

// --- User Object ---

/// A user with a required id, also stored alongside the other user fields.
#[derive(Debug, Clone)]
pub(crate) struct UserObject {
    pub id: String,
    pub fields: Map<String, Value>,
}

impl UserObject {
    /// Looks up a user property, including `id`, in the stored fields.
    pub(crate) fn get_field(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }
}

/// Users persisted for one batch of commands.
pub(crate) type UserStore = IndexMap<String, UserObject>;

/// User creation input with country validated by the command handler.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateUserInput {
    pub id: String,
    pub country: Option<String>,
}

// --- Commands (deserialized from stdin) ---

/// Input command discriminated by the "command" field.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub(crate) enum Command {
    CreateUser {
        user: CreateUserInput,
    },
    GetFlows {
        user_id: String,
    },
    Submit {
        user_id: String,
        flow: String,
        step: String,
        data: Map<String, Value>,
    },
}

// --- Response types (serialized to stdout) ---

/// Top-level response enum. Each variant serializes directly as its inner struct.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub(crate) enum Response {
    GetFlows(GetFlowsResponse),
    UserNotFound(UserNotFoundResponse),
    CreateUserFailure(CreateUserFailureResponse),
    SubmitSuccess(SubmitSuccessResponse),
    SubmitFailure(SubmitFailureResponse),
    StructuralError(StructuralErrorResponse),
}

/// Shape 1: get_flows response with user and matching flows.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct GetFlowsResponse {
    pub user: String,
    pub flows: Vec<FlowOutput>,
}

/// Shape 5: unknown user response for get_flows.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct UserNotFoundResponse {
    pub user: String,
    pub error: StructuralErrorKind,
}

/// Shape 6: failed creation response without a user key.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct CreateUserFailureResponse {
    pub submission_result: &'static str,
    pub submission_errors: Vec<SubmissionError>,
}

/// Shape 2: successful creation or submission.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SubmitSuccessResponse {
    pub user: String,
    pub submission_result: &'static str,
}

/// Shape 3: failed submission with validation errors.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SubmitFailureResponse {
    pub user: String,
    pub submission_result: &'static str,
    pub submission_errors: Vec<SubmissionError>,
}

/// Shape 4: structural error (user/flow/step not found, wrong step order).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct StructuralErrorResponse {
    pub user: String,
    pub flow: String,
    pub error: StructuralErrorKind,
}

/// Structural error categories, including user_not_found for get_flows and submit.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StructuralErrorKind {
    UserNotFound,
    FlowNotFound,
    StepNotFound,
    NotNextStep,
}

/// A single flow in the response output.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct FlowOutput {
    pub name: String,
    pub status: FlowStatus,
    pub next_step: NextStepOutput,
}

/// Only pending flows are returned; completed flows are omitted.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FlowStatus {
    Pending,
}

/// Next step details included when a flow is pending.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct NextStepOutput {
    pub name: String,
    pub fields: Vec<String>,
}

/// A field validation error in a submit response.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SubmissionError {
    pub field: String,
    pub rule: String,
}

// --- Internal intermediate types ---

/// Domain representation of a flow evaluation result before conversion to output.
pub(crate) struct FlowEvaluation {
    pub name: String,
    pub next_step: Option<NextStep>,
}

/// Domain representation of the next incomplete step.
pub(crate) struct NextStep {
    pub name: String,
    pub fields: Vec<String>,
}
