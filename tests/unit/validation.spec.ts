import { describe, it, expect } from '@jest/globals';
import {
  isValidDate,
  validateFieldType,
  validateStepFields,
  validateRequirementsIntegrity,
} from '../../src/domain/index.js';
import type { DataType, Requirements } from '../../src/domain/index.js';

describe('isValidDate', () => {
  it.each([
    ['2026-01-15', true],
    ['2024-02-29', true],
    ['2000-01-01', true],
    ['1985-12-12', true],
    ['01-15-2026', false],
    ['2026/01/15', false],
    ['2026-1-5', false],
    ['2026-01', false],
    ['not-a-date', false],
    ['', false],
    ['2025-02-29', false],
    ['2026-02-30', false],
    ['2026-04-31', false],
    ['2026-13-01', false],
    ['2026-00-15', false],
    ['2026-01-00', false],
    ['2026-01-32', false],
  ])('isValidDate(%s) returns %s', (input, expected) => {
    expect(isValidDate(input)).toBe(expected);
  });
});

describe('validateFieldType', () => {
  it.each<[DataType, unknown, boolean]>([
    ['string', 'hello', true],
    ['string', '', true],
    ['string', 42, false],
    ['string', null, false],
    ['string', true, false],
    ['string', undefined, false],
    ['number', 42, true],
    ['number', 0, true],
    ['number', -5, true],
    ['number', 3.14, true],
    ['number', '42', false],
    ['number', NaN, false],
    ['number', Infinity, false],
    ['number', true, false],
    ['number', null, false],
    ['date', '2026-01-15', true],
    ['date', '2024-02-29', true],
    ['date', '2025-02-29', false],
    ['date', '2026-02-30', false],
    ['date', '01-15-2026', false],
    ['date', 12345, false],
    ['date', null, false],
    ['string', [1, 2], false],
    ['string', { a: 1 }, false],
    ['number', [1], false],
    ['date', {}, false],
  ])('validateFieldType(%s, %s) returns %s', (dataType, value, expected) => {
    expect(validateFieldType(dataType, value)).toBe(expected);
  });
});

describe('validateStepFields', () => {
  it('returns empty array when all fields valid', () => {
    const errors = validateStepFields(['name'], { name: 'Alice' }, { name: 'string' });
    expect(errors).toEqual([]);
  });

  it('returns errors for invalid fields in step field definition order', () => {
    const errors = validateStepFields(
      ['age', 'name'],
      { age: 'not-a-number', name: 42 },
      { age: 'number', name: 'string' }
    );
    expect(errors).toEqual([
      { field: 'age', rule: 'number' },
      { field: 'name', rule: 'string' },
    ]);
  });

  it('returns error for missing fields with expected data_type as rule', () => {
    const errors = validateStepFields(['dob'], {}, { dob: 'date' });
    expect(errors).toEqual([{ field: 'dob', rule: 'date' }]);
  });

  it('ignores extra fields in data not defined in step', () => {
    const errors = validateStepFields(
      ['name'],
      { name: 'Alice', extra: 'ignored' },
      { name: 'string' }
    );
    expect(errors).toEqual([]);
  });

  it('returns multiple errors when multiple fields fail', () => {
    const errors = validateStepFields(
      ['a', 'b', 'c'],
      { a: 1, b: 2, c: 3 },
      { a: 'string', b: 'string', c: 'date' }
    );
    expect(errors).toHaveLength(3);
  });

  it('returns error with rule number when string submitted for number field', () => {
    const errors = validateStepFields(['count'], { count: '5' }, { count: 'number' });
    expect(errors).toEqual([{ field: 'count', rule: 'number' }]);
  });
});

describe('validateRequirementsIntegrity', () => {
  it('does not throw for valid requirements', () => {
    const req: Requirements = {
      data_types: { name: 'string' },
      steps: { identity: { fields: ['name'] } },
      flows: { kyc: { conditions: {}, steps: ['identity'] } },
    };
    expect(() => validateRequirementsIntegrity(req)).not.toThrow();
  });

  it('throws when step references field not in data_types', () => {
    const req: Requirements = {
      data_types: {},
      steps: { identity: { fields: ['name'] } },
      flows: {},
    };
    expect(() => validateRequirementsIntegrity(req)).toThrow(
      "Step 'identity' references field 'name' not defined in data_types"
    );
  });

  it('throws when flow references step not in steps', () => {
    const req: Requirements = {
      data_types: {},
      steps: {},
      flows: { kyc: { conditions: {}, steps: ['missing_step'] } },
    };
    expect(() => validateRequirementsIntegrity(req)).toThrow(
      "Flow 'kyc' references step 'missing_step' not defined in steps"
    );
  });
});
