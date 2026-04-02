import { describe, it, expect } from '@jest/globals';
import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { readFileSync } from 'node:fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const PROJECT_ROOT = resolve(__dirname, '..', '..', '..');

function run(commands: unknown[]): unknown[] {
  const stdout = execSync('./onboard requirements.json', {
    cwd: PROJECT_ROOT,
    input: JSON.stringify(commands),
    encoding: 'utf-8',
  });
  return JSON.parse(stdout) as unknown[];
}

describe('spec examples', () => {
  it('produces output matching output.json for all 9 commands', () => {
    const input = JSON.parse(readFileSync(resolve(PROJECT_ROOT, 'input.json'), 'utf-8'));
    const expected = JSON.parse(readFileSync(resolve(PROJECT_ROOT, 'output.json'), 'utf-8'));
    const results = run(input);
    expect(results).toEqual(expected);
  });
});

describe('get_flows edge cases', () => {
  it('returns empty flows for user with no country', () => {
    const results = run([{ command: 'get_flows', user: { id: 'u1' } }]);
    expect(results).toEqual([{ user: 'u1', flows: [] }]);
  });

  it('returns complete flow when all steps are done', () => {
    const results = run([{
      command: 'get_flows',
      user: {
        id: 'u1',
        country: 'US',
        full_name: 'Alice',
        date_of_birth: '1990-01-01',
        city: 'NYC',
        passport_number: 12345,
      },
    }]);
    expect(results).toEqual([{
      user: 'u1',
      flows: [{ name: 'us_onboarding', status: 'complete' }],
    }]);
  });
});

describe('submit edge cases', () => {
  it('returns not_next_step when submitting a step that is not next', () => {
    const results = run([{
      command: 'submit',
      user: { id: 'u1', country: 'US' },
      flow: 'us_onboarding',
      step: 'identity_verification',
      data: { passport_number: 12345 },
    }]);
    expect(results).toEqual([{
      user: 'u1',
      flow: 'us_onboarding',
      error: 'not_next_step',
    }]);
  });

  it('returns not_next_step when resubmitting a completed step', () => {
    const results = run([{
      command: 'submit',
      user: {
        id: 'u1',
        country: 'US',
        full_name: 'Alice',
        date_of_birth: '1990-01-01',
        city: 'NYC',
      },
      flow: 'us_onboarding',
      step: 'personal_information',
      data: { full_name: 'Bob', date_of_birth: '1991-01-01', city: 'LA' },
    }]);
    expect(results).toEqual([{
      user: 'u1',
      flow: 'us_onboarding',
      error: 'not_next_step',
    }]);
  });

  it('returns flow_not_found when user country does not match flow conditions', () => {
    const results = run([{
      command: 'submit',
      user: { id: 'u1', country: 'DE' },
      flow: 'us_onboarding',
      step: 'personal_information',
      data: {},
    }]);
    expect(results).toEqual([{
      user: 'u1',
      flow: 'us_onboarding',
      error: 'flow_not_found',
    }]);
  });

  it('returns validation errors for missing required fields', () => {
    const results = run([{
      command: 'submit',
      user: { id: 'u1', country: 'DE' },
      flow: 'de_onboarding',
      step: 'personal_information',
      data: {},
    }]);
    expect(results).toEqual([{
      user: 'u1',
      submission_result: 'failure',
      submission_errors: [
        { field: 'full_name', rule: 'string' },
        { field: 'date_of_birth', rule: 'date' },
        { field: 'city', rule: 'string' },
      ],
      flows: [{
        name: 'de_onboarding',
        status: 'pending',
        next_step: {
          name: 'personal_information',
          fields: ['full_name', 'date_of_birth', 'city'],
        },
      }],
    }]);
  });

  it('returns multiple validation errors in step field definition order', () => {
    const results = run([{
      command: 'submit',
      user: { id: 'u1', country: 'DE' },
      flow: 'de_onboarding',
      step: 'personal_information',
      data: { full_name: 123, date_of_birth: 'bad', city: 456 },
    }]);
    const result = results[0] as { submission_errors: unknown[] };
    expect(result.submission_errors).toEqual([
      { field: 'full_name', rule: 'string' },
      { field: 'date_of_birth', rule: 'date' },
      { field: 'city', rule: 'string' },
    ]);
  });

  it('rejects numeric string for number field', () => {
    const results = run([{
      command: 'submit',
      user: {
        id: 'u1',
        country: 'US',
        full_name: 'Alice',
        date_of_birth: '1990-01-01',
        city: 'NYC',
      },
      flow: 'us_onboarding',
      step: 'identity_verification',
      data: { passport_number: '12345' },
    }]);
    const result = results[0] as { submission_errors: unknown[] };
    expect(result.submission_errors).toEqual([
      { field: 'passport_number', rule: 'number' },
    ]);
  });

  it('silently ignores extra fields in submit data', () => {
    const results = run([{
      command: 'submit',
      user: { id: 'u1', country: 'DE' },
      flow: 'de_onboarding',
      step: 'personal_information',
      data: {
        full_name: 'Alice',
        date_of_birth: '1990-01-01',
        city: 'Berlin',
        extra_field: 'ignored',
      },
    }]);
    const result = results[0] as { submission_result: string };
    expect(result.submission_result).toBe('success');
  });

  it('rejects invalid date format', () => {
    const results = run([{
      command: 'submit',
      user: { id: 'u1', country: 'DE' },
      flow: 'de_onboarding',
      step: 'personal_information',
      data: { full_name: 'Alice', date_of_birth: '04-01-2000', city: 'Berlin' },
    }]);
    const result = results[0] as { submission_errors: unknown[] };
    expect(result.submission_errors).toEqual([
      { field: 'date_of_birth', rule: 'date' },
    ]);
  });

  it('rejects impossible calendar date', () => {
    const results = run([{
      command: 'submit',
      user: { id: 'u1', country: 'DE' },
      flow: 'de_onboarding',
      step: 'personal_information',
      data: { full_name: 'Alice', date_of_birth: '2026-02-30', city: 'Berlin' },
    }]);
    const result = results[0] as { submission_errors: unknown[] };
    expect(result.submission_errors).toEqual([
      { field: 'date_of_birth', rule: 'date' },
    ]);
  });

  it('accepts empty string as valid string field', () => {
    const results = run([{
      command: 'submit',
      user: { id: 'u1', country: 'DE' },
      flow: 'de_onboarding',
      step: 'personal_information',
      data: { full_name: '', date_of_birth: '1990-01-01', city: '' },
    }]);
    const result = results[0] as { submission_result: string };
    expect(result.submission_result).toBe('success');
  });

  it('completes multi-step flow after all steps submitted', () => {
    const results = run([{
      command: 'submit',
      user: {
        id: 'u1',
        country: 'US',
        full_name: 'Alice',
        date_of_birth: '1990-01-01',
        city: 'NYC',
      },
      flow: 'us_onboarding',
      step: 'identity_verification',
      data: { passport_number: 12345 },
    }]);
    expect(results).toEqual([{
      user: 'u1',
      submission_result: 'success',
      flows: [{ name: 'us_onboarding', status: 'complete' }],
    }]);
  });
});

describe('empty input', () => {
  it('returns empty array for empty commands array', () => {
    const results = run([]);
    expect(results).toEqual([]);
  });
});

describe('error handling', () => {
  it('exits with code 1 for malformed JSON on stdin', () => {
    try {
      execSync('./onboard requirements.json', {
        cwd: PROJECT_ROOT,
        input: 'not json',
        encoding: 'utf-8',
        stdio: ['pipe', 'pipe', 'pipe'],
      });
      throw new Error('Expected command to fail');
    } catch (err: unknown) {
      const error = err as { status: number };
      expect(error.status).toBe(1);
    }
  });

  it('exits with code 2 when requirements file argument is missing', () => {
    try {
      execSync('node dist/cli/main.js', {
        cwd: PROJECT_ROOT,
        input: '[]',
        encoding: 'utf-8',
        stdio: ['pipe', 'pipe', 'pipe'],
      });
      throw new Error('Expected command to fail');
    } catch (err: unknown) {
      const error = err as { status: number };
      expect(error.status).toBe(2);
    }
  });
});
