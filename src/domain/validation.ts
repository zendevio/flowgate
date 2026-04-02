import type { DataType, Requirements, SubmissionError } from './types.js';
import { assertNever } from './types.js';

export function isValidDate(value: string): boolean {
  const regex = /^\d{4}-\d{2}-\d{2}$/;
  if (!regex.test(value)) {
    return false;
  }

  const [yearStr, monthStr, dayStr] = value.split('-');
  const year = Number(yearStr);
  const month = Number(monthStr);
  const day = Number(dayStr);

  if (month < 1 || month > 12) {
    return false;
  }

  if (day < 1) {
    return false;
  }

  const daysInMonth = new Date(year, month, 0).getDate();
  return day <= daysInMonth;
}

export function validateFieldType(dataType: DataType, value: unknown): boolean {
  switch (dataType) {
    case 'string':
      return typeof value === 'string';
    case 'number':
      return typeof value === 'number' && Number.isFinite(value);
    case 'date':
      return typeof value === 'string' && isValidDate(value);
    default:
      return assertNever(dataType);
  }
}

export function validateStepFields(
  stepFields: string[],
  data: Record<string, unknown>,
  dataTypes: Record<string, DataType>
): SubmissionError[] {
  const errors: SubmissionError[] = [];

  for (const field of stepFields) {
    const expectedType = dataTypes[field];
    if (!expectedType) {
      continue;
    }
    const value = data[field];
    if (!validateFieldType(expectedType, value)) {
      errors.push({ field, rule: expectedType });
    }
  }

  return errors;
}

export function validateRequirementsIntegrity(requirements: Requirements): void {
  for (const [stepName, step] of Object.entries(requirements.steps)) {
    for (const field of step.fields) {
      if (!(field in requirements.data_types)) {
        throw new Error(`Step '${stepName}' references field '${field}' not defined in data_types`);
      }
    }
  }

  for (const [flowName, flow] of Object.entries(requirements.flows)) {
    for (const stepName of flow.steps) {
      if (!(stepName in requirements.steps)) {
        throw new Error(`Flow '${flowName}' references step '${stepName}' not defined in steps`);
      }
    }
  }
}
