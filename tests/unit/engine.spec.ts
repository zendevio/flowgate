import { describe, it, expect } from '@jest/globals';
import {
  evaluateFlows,
  matchesConditions,
  isStepComplete,
  findNextStep,
  mergeStepData,
} from '../../src/domain/index.js';
import type { Requirements, UserObject, FlowConfig } from '../../src/domain/index.js';

const makeReqs = (overrides: Partial<Requirements> = {}): Requirements => ({
  data_types: { name: 'string', age: 'number', dob: 'date' },
  steps: {
    personal: { fields: ['name', 'age'] },
    identity: { fields: ['dob'] },
  },
  flows: {
    gb_flow: {
      conditions: { country: 'GB' },
      steps: ['personal', 'identity'],
    },
  },
  ...overrides,
});

const makeUser = (extra: Record<string, unknown> = {}): UserObject => ({ id: 'u1', ...extra });

describe('matchesConditions', () => {
  it('returns true when all conditions match user fields', () => {
    expect(matchesConditions(makeUser({ country: 'GB' }), { country: 'GB' })).toBe(true);
  });

  it('returns false when a condition value does not match', () => {
    expect(matchesConditions(makeUser({ country: 'US' }), { country: 'GB' })).toBe(false);
  });

  it('returns false when user is missing a condition key', () => {
    expect(matchesConditions(makeUser(), { country: 'GB' })).toBe(false);
  });

  it('returns true when conditions object is empty (vacuously true)', () => {
    expect(matchesConditions(makeUser(), {})).toBe(true);
  });
});

describe('isStepComplete', () => {
  it('returns true when user has all step fields', () => {
    const user = makeUser({ name: 'Alice', age: 30 });
    expect(isStepComplete(user, 'personal', makeReqs())).toBe(true);
  });

  it('returns false when user is missing a step field', () => {
    const user = makeUser({ name: 'Alice' });
    expect(isStepComplete(user, 'personal', makeReqs())).toBe(false);
  });

  it('does not count inherited properties as filled fields', () => {
    const reqs = makeReqs({
      data_types: { constructor: 'string' as const },
      steps: { proto: { fields: ['constructor'] } },
    });
    expect(isStepComplete(makeUser(), 'proto', reqs)).toBe(false);
  });

  it('returns false when step name is not in requirements', () => {
    expect(isStepComplete(makeUser(), 'nonexistent', makeReqs())).toBe(false);
  });

  it('returns false when a user field is null (null is not a valid value)', () => {
    const user = makeUser({ name: null, age: 30 });
    expect(isStepComplete(user, 'personal', makeReqs())).toBe(false);
  });
});

describe('findNextStep', () => {
  const flow: FlowConfig = { conditions: { country: 'GB' }, steps: ['personal', 'identity'] };

  it('returns first incomplete step with remaining fields only', () => {
    const result = findNextStep(makeUser(), flow, makeReqs());
    expect(result).toEqual({ name: 'personal', fields: ['name', 'age'] });
  });

  it('returns null when all steps are complete', () => {
    const user = makeUser({ name: 'Alice', age: 30, dob: '1990-01-01' });
    expect(findNextStep(user, flow, makeReqs())).toBeNull();
  });

  it('skips completed steps and returns next incomplete one', () => {
    const user = makeUser({ name: 'Alice', age: 30 });
    const result = findNextStep(user, flow, makeReqs());
    expect(result).toEqual({ name: 'identity', fields: ['dob'] });
  });

  it('returns only remaining unfilled fields for partially completed step', () => {
    const user = makeUser({ name: 'Alice' });
    const result = findNextStep(user, flow, makeReqs());
    expect(result).toEqual({ name: 'personal', fields: ['age'] });
  });
});

describe('mergeStepData', () => {
  it('merges only step-defined fields from data onto user', () => {
    const user = makeUser({ country: 'GB' });
    const result = mergeStepData(user, { name: 'Alice', age: 30 }, ['name', 'age']);
    expect(result).toEqual({ id: 'u1', country: 'GB', name: 'Alice', age: 30 });
  });

  it('ignores extra fields in data not in step definition', () => {
    const user = makeUser();
    const result = mergeStepData(user, { name: 'Alice', extra: 'ignored' }, ['name']);
    expect(result).not.toHaveProperty('extra');
    expect(result).toEqual({ id: 'u1', name: 'Alice' });
  });

  it('preserves existing user fields (id, country, etc.)', () => {
    const user = makeUser({ country: 'GB', name: 'Old' });
    const result = mergeStepData(user, { name: 'New' }, ['name']);
    expect(result.country).toBe('GB');
    expect(result.name).toBe('New');
  });

  it('does not overwrite user id even if data contains id', () => {
    const user = makeUser();
    const result = mergeStepData(user, { id: 'hacker', name: 'Alice' }, ['name']);
    expect(result.id).toBe('u1');
    expect(result).not.toHaveProperty('id', 'hacker');
  });
});

describe('evaluateFlows', () => {
  it('returns matching flows with correct next step', () => {
    const user = makeUser({ country: 'GB' });
    const results = evaluateFlows(user, makeReqs());
    expect(results).toEqual([
      { name: 'gb_flow', nextStep: { name: 'personal', fields: ['name', 'age'] } },
    ]);
  });

  it('skips flows whose conditions do not match user', () => {
    const user = makeUser({ country: 'US' });
    expect(evaluateFlows(user, makeReqs())).toEqual([]);
  });

  it('returns empty array when no flows match (e.g., FR user)', () => {
    const user = makeUser({ country: 'FR' });
    expect(evaluateFlows(user, makeReqs())).toHaveLength(0);
  });

  it('returns flow as complete (nextStep: null) when all steps done', () => {
    const user = makeUser({ country: 'GB', name: 'Alice', age: 30, dob: '1990-01-01' });
    const results = evaluateFlows(user, makeReqs());
    expect(results).toEqual([{ name: 'gb_flow', nextStep: null }]);
  });

  it('treats null user field as incomplete for step evaluation', () => {
    const user = makeUser({ country: 'GB', name: null, age: 30 });
    const results = evaluateFlows(user, makeReqs());
    expect(results).toEqual([
      { name: 'gb_flow', nextStep: { name: 'personal', fields: ['name'] } },
    ]);
  });

  it('preserves flow definition order from requirements', () => {
    const reqs = makeReqs({
      flows: {
        gb_flow: { conditions: { country: 'GB' }, steps: ['personal'] },
        gb_flow_2: { conditions: { country: 'GB' }, steps: ['identity'] },
      },
    });
    const user = makeUser({ country: 'GB' });
    const names = evaluateFlows(user, reqs).map((f) => f.name);
    expect(names).toEqual(['gb_flow', 'gb_flow_2']);
  });
});
