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

// Own-property lookup so inherited names like `constructor` never count as user data
function getUserField(user: UserObject, key: string): unknown {
  return Object.hasOwn(user, key) ? user[key] : undefined;
}

export function matchesConditions(user: UserObject, conditions: Record<string, string>): boolean {
  for (const [key, value] of Object.entries(conditions)) {
    if (getUserField(user, key) !== value) {
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
    if (getUserField(user, field) == null) {
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
      // Return only remaining (unfilled) fields
      const remainingFields = step.fields.filter((field) => getUserField(user, field) == null);
      return { name: stepName, fields: remainingFields };
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
    if (Object.hasOwn(data, field)) {
      filtered[field] = data[field];
    }
  }
  return { ...user, ...filtered } as UserObject;
}
