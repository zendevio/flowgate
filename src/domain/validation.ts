import type { DataType, Requirements, SubmissionError } from './types.js';
import { assertNever } from './types.js';

const DAYS_IN_MONTH = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31] as const;

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

  // Arithmetic leap rule: Date maps years 0-99 to 1900-1999, which misjudges year 0
  const isLeapYear = (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;
  const daysInMonth = month === 2 && isLeapYear ? 29 : DAYS_IN_MONTH[month - 1]!;
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
      errors.push({ field, rule: `must_be_${expectedType}` });
    }
  }

  return errors;
}

export function validateRequirementsIntegrity(requirements: Requirements): void {
  for (const [stepName, step] of Object.entries(requirements.steps)) {
    for (const field of step.fields) {
      // The user id is identity, not step data: a submit must never overwrite it
      if (field === 'id') {
        throw new Error(`Step '${stepName}' uses reserved field 'id'`);
      }
      if (!Object.hasOwn(requirements.data_types, field)) {
        throw new Error(`Step '${stepName}' references field '${field}' not defined in data_types`);
      }
    }
  }

  for (const [flowName, flow] of Object.entries(requirements.flows)) {
    for (const stepName of flow.steps) {
      if (!Object.hasOwn(requirements.steps, stepName)) {
        throw new Error(`Flow '${flowName}' references step '${stepName}' not defined in steps`);
      }
    }
  }
}
