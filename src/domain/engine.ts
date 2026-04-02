import type { FlowConfig, Requirements, UserObject } from './types.js';

export interface FlowEvaluation {
  name: string;
  nextStep: { name: string; fields: string[] } | null;
}

export function evaluateFlows(user: UserObject, requirements: Requirements): FlowEvaluation[] {
  const results: FlowEvaluation[] = [];

  for (const [name, flow] of Object.entries(requirements.flows)) {
    if (!matchesConditions(user, flow.conditions)) {
      continue;
    }
    const nextStep = findNextStep(user, flow, requirements);
    results.push({ name, nextStep });
  }

  return results;
}

export function matchesConditions(user: UserObject, conditions: Record<string, string>): boolean {
  for (const [key, value] of Object.entries(conditions)) {
    if (user[key] !== value) {
      return false;
    }
  }
  return true;
}

export function isStepComplete(
  user: UserObject,
  stepName: string,
  requirements: Requirements
): boolean {
  const step = requirements.steps[stepName];
  if (!step) {
    return false;
  }
  for (const field of step.fields) {
    if (user[field] == null) {
      return false;
    }
  }
  return true;
}

export function findNextStep(
  user: UserObject,
  flow: FlowConfig,
  requirements: Requirements
): { name: string; fields: string[] } | null {
  for (const stepName of flow.steps) {
    if (!isStepComplete(user, stepName, requirements)) {
      const step = requirements.steps[stepName];
      if (!step) {
        return null;
      }
      return { name: stepName, fields: step.fields };
    }
  }
  return null;
}

export function mergeStepData(
  user: UserObject,
  data: Record<string, unknown>,
  stepFields: string[]
): UserObject {
  const filtered: Record<string, unknown> = {};
  for (const field of stepFields) {
    if (field in data) {
      filtered[field] = data[field];
    }
  }
  return { ...user, ...filtered } as UserObject;
}
