// String union constants and derived types

export const DATA_TYPES = ['string', 'number', 'date'] as const;
export type DataType = (typeof DATA_TYPES)[number];

// Completed flows are omitted from output, so only pending is ever emitted
export const FLOW_STATUSES = ['pending'] as const;
export type FlowStatus = (typeof FLOW_STATUSES)[number];

export const COMMAND_TYPES = ['create_user', 'get_flows', 'submit'] as const;
export type CommandType = (typeof COMMAND_TYPES)[number];

export const ERROR_TYPES = [
  'user_not_found',
  'flow_not_found',
  'step_not_found',
  'not_next_step',
] as const;
export type ErrorType = (typeof ERROR_TYPES)[number];

export const SUBMISSION_RESULTS = ['success', 'failure'] as const;
export type SubmissionResult = (typeof SUBMISSION_RESULTS)[number];

// Domain interfaces

export interface StepConfig {
  fields: string[];
}

export interface FlowConfig {
  conditions: Record<string, string>;
  steps: string[];
}

export interface Requirements {
  data_types: Record<string, DataType>;
  steps: Record<string, StepConfig>;
  flows: Record<string, FlowConfig>;
}

export interface UserObject {
  id: string;
  [key: string]: unknown;
}

export interface SubmissionError {
  field: string;
  rule: string;
}

// In-memory persistence, scoped to a single CLI run
export type UserStore = Map<string, UserObject>;

// Command interfaces

export interface CreateUserCommand {
  command: 'create_user';
  // Optional at the type level: a missing country is a per-command business error, not a parse failure
  user: { id: string; country?: string | null };
}

export interface GetFlowsCommand {
  command: 'get_flows';
  user_id: string;
}

export interface SubmitCommand {
  command: 'submit';
  user_id: string;
  flow: string;
  step: string;
  data: Record<string, unknown>;
}

export type Command = CreateUserCommand | GetFlowsCommand | SubmitCommand;

// Exhaustive check utility

export function assertNever(value: never, message?: string): never {
  throw new Error(message ?? `Unhandled value: ${JSON.stringify(value)}`);
}
