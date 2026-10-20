import {
  evaluateFlows,
  matchesConditions,
  findNextStep,
  mergeStepData,
  validateStepFields,
  assertNever,
} from '../domain/index.js';
import type {
  Command,
  CreateUserCommand,
  GetFlowsCommand,
  SubmitCommand,
  Requirements,
  UserObject,
  UserStore,
  ErrorType,
  SubmissionError,
  FlowEvaluation,
} from '../domain/index.js';

export function handleCommand(
  command: Command,
  requirements: Requirements,
  store: UserStore
): object {
  switch (command.command) {
    case 'create_user':
      return handleCreateUser(command, store);
    case 'get_flows':
      return handleGetFlows(command, requirements, store);
    case 'submit':
      return handleSubmit(command, requirements, store);
    default:
      return assertNever(command);
  }
}

function handleCreateUser(command: CreateUserCommand, store: UserStore): object {
  const { id, country } = command.user;

  if (!country) {
    return buildCreateUserFailure({ field: 'country', rule: 'required' });
  }
  if (store.has(id)) {
    return buildCreateUserFailure({ field: 'user_id', rule: 'duplicate' });
  }

  const user: UserObject = { id, country };
  store.set(id, user);
  return buildSubmitSuccess(id);
}

function handleGetFlows(
  command: GetFlowsCommand,
  requirements: Requirements,
  store: UserStore
): object {
  const userId = command.user_id;
  const user = store.get(userId);
  if (!user) {
    return buildUserNotFound(userId);
  }
  const evals = evaluateFlows(user, requirements);
  return buildGetFlowsResponse(userId, evals);
}

function handleSubmit(
  command: SubmitCommand,
  requirements: Requirements,
  store: UserStore
): object {
  const userId = command.user_id;
  const flowName = command.flow;
  const stepName = command.step;

  // Error priority 1: user_not_found
  const user = store.get(userId);
  if (!user) {
    return buildStructuralError(userId, flowName, 'user_not_found');
  }

  // Error priority 2: flow_not_found (undefined, prototype pollution, OR conditions don't match)
  if (!Object.hasOwn(requirements.flows, flowName)) {
    return buildStructuralError(userId, flowName, 'flow_not_found');
  }
  const flow = requirements.flows[flowName]!;
  if (!matchesConditions(user, flow.conditions)) {
    return buildStructuralError(userId, flowName, 'flow_not_found');
  }

  // Error priority 3: step_not_found (step not in THIS flow's step list)
  if (!flow.steps.includes(stepName)) {
    return buildStructuralError(userId, flowName, 'step_not_found');
  }

  // Error priority 4: not_next_step (a partially filled step is still the next step)
  const nextStep = findNextStep(user, flow, requirements);
  if (!nextStep || nextStep.name !== stepName) {
    return buildStructuralError(userId, flowName, 'not_next_step');
  }

  if (!Object.hasOwn(requirements.steps, stepName)) {
    return buildStructuralError(userId, flowName, 'step_not_found');
  }
  const step = requirements.steps[stepName]!;

  // Error priority 5: field validation. Partial submissions are allowed, so only the
  // step fields present in data are validated, but at least one must be present.
  const providedFields = step.fields.filter((field) => Object.hasOwn(command.data, field));
  if (providedFields.length === 0) {
    return buildSubmitFailure(userId, [{ field: 'data', rule: 'required' }]);
  }
  const errors = validateStepFields(providedFields, command.data, requirements.data_types);
  if (errors.length > 0) {
    return buildSubmitFailure(userId, errors);
  }

  // Success: filtered merge into the stored user (a failed submit stores nothing)
  store.set(userId, mergeStepData(user, command.data, step.fields));
  return buildSubmitSuccess(userId);
}

// Response builders (exact key ordering per spec)

function buildGetFlowsResponse(userId: string, evals: FlowEvaluation[]): object {
  // Completed flows are omitted from get_flows output
  const flows: object[] = [];
  for (const evaluation of evals) {
    if (evaluation.nextStep) {
      flows.push(buildPendingFlow(evaluation.name, evaluation.nextStep));
    }
  }
  return {
    user: userId,
    flows,
  };
}

function buildUserNotFound(userId: string): object {
  return {
    user: userId,
    error: 'user_not_found' satisfies ErrorType,
  };
}

function buildCreateUserFailure(error: SubmissionError): object {
  return {
    submission_result: 'failure',
    submission_errors: [error],
  };
}

function buildSubmitSuccess(userId: string): object {
  return {
    user: userId,
    submission_result: 'success',
  };
}

function buildSubmitFailure(userId: string, errors: SubmissionError[]): object {
  return {
    user: userId,
    submission_result: 'failure',
    submission_errors: errors,
  };
}

function buildStructuralError(userId: string, flowName: string, error: ErrorType): object {
  return {
    user: userId,
    flow: flowName,
    error,
  };
}

function buildPendingFlow(name: string, nextStep: { name: string; fields: string[] }): object {
  return {
    name,
    status: 'pending',
    next_step: {
      name: nextStep.name,
      fields: nextStep.fields,
    },
  };
}
