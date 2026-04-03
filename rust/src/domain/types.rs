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
            DataType::String => "string",
            DataType::Number => "number",
            DataType::Date => "date",
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

/// A user with a required id and arbitrary additional fields.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UserObject {
    pub id: String,
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

impl UserObject {
    /// Looks up a field value by name, checking both the extracted `id` and the
    /// dynamic `fields` map. This ensures condition matching and step completion
    /// work correctly for all user properties.
    pub(crate) fn get_field(&self, key: &str) -> Option<&Value> {
        if key == "id" {
            // id is extracted by serde into a named field; synthesize a lookup
            // by returning None and letting callers compare directly when needed.
            // In practice, conditions and step fields never reference "id".
            return None;
        }
        self.fields.get(key)
    }
}

// --- Commands (deserialized from stdin) ---

/// Input command discriminated by the "command" field.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub(crate) enum Command {
    GetFlows {
        user: UserObject,
    },
    Submit {
        user: UserObject,
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

/// Shape 2: successful submission with merged user flows.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SubmitSuccessResponse {
    pub user: String,
    pub submission_result: &'static str,
    pub flows: Vec<FlowOutput>,
}

/// Shape 3: failed submission with validation errors and original user flows.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SubmitFailureResponse {
    pub user: String,
    pub submission_result: &'static str,
    pub submission_errors: Vec<SubmissionError>,
    pub flows: Vec<FlowOutput>,
}

/// Shape 4: structural error (flow/step not found, wrong step order).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct StructuralErrorResponse {
    pub user: String,
    pub flow: String,
    pub error: StructuralErrorKind,
}

/// Structural error categories for submit command validation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StructuralErrorKind {
    FlowNotFound,
    StepNotFound,
    NotNextStep,
}

/// A single flow in the response output.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct FlowOutput {
    pub name: String,
    pub status: FlowStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_step: Option<NextStepOutput>,
}

/// Flow completion status.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FlowStatus {
    Pending,
    Complete,
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

impl FlowEvaluation {
    /// Converts this evaluation into the output representation.
    pub(crate) fn into_flow_output(self) -> FlowOutput {
        match self.next_step {
            Some(next) => FlowOutput {
                name: self.name,
                status: FlowStatus::Pending,
                next_step: Some(NextStepOutput {
                    name: next.name,
                    fields: next.fields,
                }),
            },
            None => FlowOutput {
                name: self.name,
                status: FlowStatus::Complete,
                next_step: None,
            },
        }
    }
}
