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
  GetFlowsCommand,
  SubmitCommand,
  Requirements,
  ErrorType,
  SubmissionError,
  FlowEvaluation,
} from '../domain/index.js';

export function handleCommand(command: Command, requirements: Requirements): object {
  switch (command.command) {
    case 'get_flows':
      return handleGetFlows(command, requirements);
    case 'submit':
      return handleSubmit(command, requirements);
    default:
      return assertNever(command);
  }
}

function handleGetFlows(command: GetFlowsCommand, requirements: Requirements): object {
  const userId = command.user.id;
  const evals = evaluateFlows(command.user, requirements);
  return buildGetFlowsResponse(userId, evals);
}

function handleSubmit(command: SubmitCommand, requirements: Requirements): object {
  const userId = command.user.id;
  const flowName = command.flow;
  const stepName = command.step;

  // Error priority 1: flow_not_found (undefined, prototype pollution, OR conditions don't match)
  if (!Object.hasOwn(requirements.flows, flowName)) {
    return buildStructuralError(userId, flowName, 'flow_not_found');
  }
  const flow = requirements.flows[flowName]!;
  if (!matchesConditions(command.user, flow.conditions)) {
    return buildStructuralError(userId, flowName, 'flow_not_found');
  }

  // Error priority 2: step_not_found (step not in THIS flow's step list)
  if (!flow.steps.includes(stepName)) {
    return buildStructuralError(userId, flowName, 'step_not_found');
  }

  // Error priority 3: not_next_step
  const nextStep = findNextStep(command.user, flow, requirements);
  if (!nextStep || nextStep.name !== stepName) {
    return buildStructuralError(userId, flowName, 'not_next_step');
  }

  // Error priority 4: field validation
  if (!Object.hasOwn(requirements.steps, stepName)) {
    return buildStructuralError(userId, flowName, 'step_not_found');
  }
  const step = requirements.steps[stepName]!;

  const errors = validateStepFields(step.fields, command.data, requirements.data_types);
  if (errors.length > 0) {
    const evals = evaluateFlows(command.user, requirements);
    return buildSubmitFailure(userId, errors, evals);
  }

  // Success: filtered merge, then re-evaluate
  const mergedUser = mergeStepData(command.user, command.data, step.fields);
  const evals = evaluateFlows(mergedUser, requirements);
  return buildSubmitSuccess(userId, evals);
}

// Response builders (exact key ordering per spec)

function buildGetFlowsResponse(userId: string, evals: FlowEvaluation[]): object {
  return {
    user: userId,
    flows: evals.map(toFlowOutput),
  };
}

function buildSubmitSuccess(userId: string, evals: FlowEvaluation[]): object {
  return {
    user: userId,
    submission_result: 'success',
    flows: evals.map(toFlowOutput),
  };
}

function buildSubmitFailure(
  userId: string,
  errors: SubmissionError[],
  evals: FlowEvaluation[]
): object {
  return {
    user: userId,
    submission_result: 'failure',
    submission_errors: errors,
    flows: evals.map(toFlowOutput),
  };
}

function buildStructuralError(userId: string, flowName: string, error: ErrorType): object {
  return {
    user: userId,
    flow: flowName,
    error,
  };
}

function toFlowOutput(evaluation: FlowEvaluation): object {
  if (evaluation.nextStep) {
    return buildPendingFlow(evaluation.name, evaluation.nextStep);
  }
  return buildCompleteFlow(evaluation.name);
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

function buildCompleteFlow(name: string): object {
  return {
    name,
    status: 'complete',
  };
}
