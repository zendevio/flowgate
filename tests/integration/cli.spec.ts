import { describe, it, expect } from '@jest/globals';
import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { readFileSync } from 'node:fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const PROJECT_ROOT = resolve(__dirname, '..', '..', '..');

function runRaw(commands: unknown[]): string {
  return execSync('./onboard requirements.json', {
    cwd: PROJECT_ROOT,
    input: JSON.stringify(commands),
    encoding: 'utf-8',
  });
}

function run(commands: unknown[]): unknown[] {
  return JSON.parse(runRaw(commands)) as unknown[];
}

describe('extension examples', () => {
  it('produces output byte-identical to output.json, including key order', () => {
    const input = JSON.parse(readFileSync(resolve(PROJECT_ROOT, 'input.json'), 'utf-8'));
    const expected = readFileSync(resolve(PROJECT_ROOT, 'output.json'), 'utf-8');
    expect(runRaw(input)).toBe(expected);
  });
});

describe('create_user', () => {
  it('succeeds with valid id and country', () => {
    const results = run([{ command: 'create_user', user: { id: 'cu1', country: 'DE' } }]);
    expect(results).toEqual([{ user: 'cu1', submission_result: 'success' }]);
  });

  it('fails when country is missing', () => {
    const results = run([{ command: 'create_user', user: { id: 'cu2' } }]);
    expect(results).toEqual([
      {
        submission_result: 'failure',
        submission_errors: [{ field: 'country', rule: 'required' }],
      },
    ]);
  });

  it('fails when user id is duplicate', () => {
    const results = run([
      { command: 'create_user', user: { id: 'cu3', country: 'US' } },
      { command: 'create_user', user: { id: 'cu3', country: 'US' } },
    ]);
    expect(results[1]).toEqual({
      submission_result: 'failure',
      submission_errors: [{ field: 'user_id', rule: 'duplicate' }],
    });
  });
});

describe('get_flows with user_id', () => {
  it('returns flows for created user', () => {
    const results = run([
      { command: 'create_user', user: { id: 'gf1', country: 'DE' } },
      { command: 'get_flows', user_id: 'gf1' },
    ]);
    expect(results[1]).toEqual({
      user: 'gf1',
      flows: [
        {
          name: 'de_onboarding',
          status: 'pending',
          next_step: {
            name: 'personal_information',
            fields: ['full_name', 'date_of_birth', 'city'],
          },
        },
      ],
    });
  });

  it('returns empty flows for user with no matching country', () => {
    const results = run([
      { command: 'create_user', user: { id: 'gf2', country: 'FR' } },
      { command: 'get_flows', user_id: 'gf2' },
    ]);
    expect(results[1]).toEqual({ user: 'gf2', flows: [] });
  });

  it('excludes completed flows from output', () => {
    const results = run([
      { command: 'create_user', user: { id: 'gf3', country: 'DE' } },
      {
        command: 'submit',
        user_id: 'gf3',
        flow: 'de_onboarding',
        step: 'personal_information',
        data: { full_name: 'Alice', date_of_birth: '1990-01-01', city: 'Berlin' },
      },
      { command: 'get_flows', user_id: 'gf3' },
    ]);
    expect(results[2]).toEqual({ user: 'gf3', flows: [] });
  });
});

describe('submit with persistence', () => {
  it('persists data across commands', () => {
    const results = run([
      { command: 'create_user', user: { id: 'sp1', country: 'DE' } },
      {
        command: 'submit',
        user_id: 'sp1',
        flow: 'de_onboarding',
        step: 'personal_information',
        data: { full_name: 'Alice' },
      },
      { command: 'get_flows', user_id: 'sp1' },
    ]);
    const flows = (results[2] as { flows: { next_step: { fields: string[] } }[] }).flows;
    expect(flows[0]!.next_step.fields).toEqual(['date_of_birth', 'city']);
  });

  it('allows partial step submission and accumulates fields', () => {
    const results = run([
      { command: 'create_user', user: { id: 'sp2', country: 'DE' } },
      {
        command: 'submit',
        user_id: 'sp2',
        flow: 'de_onboarding',
        step: 'personal_information',
        data: { full_name: 'Alice', date_of_birth: '1990-01-01' },
      },
      {
        command: 'submit',
        user_id: 'sp2',
        flow: 'de_onboarding',
        step: 'personal_information',
        data: { city: 'Berlin' },
      },
      { command: 'get_flows', user_id: 'sp2' },
    ]);
    expect(results[1]).toEqual({ user: 'sp2', submission_result: 'success' });
    expect(results[2]).toEqual({ user: 'sp2', submission_result: 'success' });
    expect(results[3]).toEqual({ user: 'sp2', flows: [] });
  });

  it('returns success without flows on submit', () => {
    const results = run([
      { command: 'create_user', user: { id: 'sp3', country: 'DE' } },
      {
        command: 'submit',
        user_id: 'sp3',
        flow: 'de_onboarding',
        step: 'personal_information',
        data: { full_name: 'Alice', date_of_birth: '1990-01-01', city: 'Berlin' },
      },
    ]);
    expect(results[1]).toEqual({ user: 'sp3', submission_result: 'success' });
    expect(results[1]).not.toHaveProperty('flows');
  });
});

describe('submit validation', () => {
  it('returns must_be_ prefixed validation errors', () => {
    const results = run([
      { command: 'create_user', user: { id: 'sv1', country: 'DE' } },
      {
        command: 'submit',
        user_id: 'sv1',
        flow: 'de_onboarding',
        step: 'personal_information',
        data: { full_name: 123 },
      },
    ]);
    const result = results[1] as { submission_errors: unknown[] };
    expect(result.submission_errors).toEqual([{ field: 'full_name', rule: 'must_be_string' }]);
  });

  it('rejects invalid date with must_be_date rule', () => {
    const results = run([
      { command: 'create_user', user: { id: 'sv2', country: 'US' } },
      {
        command: 'submit',
        user_id: 'sv2',
        flow: 'us_onboarding',
        step: 'personal_information',
        data: { date_of_birth: 'January 1st 1990' },
      },
    ]);
    const result = results[1] as { submission_errors: unknown[] };
    expect(result.submission_errors).toEqual([{ field: 'date_of_birth', rule: 'must_be_date' }]);
  });

  it('rejects numeric string for number field', () => {
    const results = run([
      { command: 'create_user', user: { id: 'sv3', country: 'US' } },
      {
        command: 'submit',
        user_id: 'sv3',
        flow: 'us_onboarding',
        step: 'personal_information',
        data: { full_name: 'Alice', date_of_birth: '1990-01-01', city: 'NYC' },
      },
      {
        command: 'submit',
        user_id: 'sv3',
        flow: 'us_onboarding',
        step: 'identity_verification',
        data: { passport_number: '12345' },
      },
    ]);
    const result = results[2] as { submission_errors: unknown[] };
    expect(result.submission_errors).toEqual([
      { field: 'passport_number', rule: 'must_be_number' },
    ]);
  });

  it('returns flow_not_found when user country does not match', () => {
    const results = run([
      { command: 'create_user', user: { id: 'sv4', country: 'DE' } },
      {
        command: 'submit',
        user_id: 'sv4',
        flow: 'us_onboarding',
        step: 'personal_information',
        data: {},
      },
    ]);
    expect(results[1]).toEqual({
      user: 'sv4',
      flow: 'us_onboarding',
      error: 'flow_not_found',
    });
  });

  it('returns not_next_step when submitting wrong step', () => {
    const results = run([
      { command: 'create_user', user: { id: 'sv5', country: 'US' } },
      {
        command: 'submit',
        user_id: 'sv5',
        flow: 'us_onboarding',
        step: 'identity_verification',
        data: { passport_number: 12345 },
      },
    ]);
    expect(results[1]).toEqual({
      user: 'sv5',
      flow: 'us_onboarding',
      error: 'not_next_step',
    });
  });
});

describe('multi-step flow completion', () => {
  it('completes US onboarding across multiple submits', () => {
    const results = run([
      { command: 'create_user', user: { id: 'ms1', country: 'US' } },
      {
        command: 'submit',
        user_id: 'ms1',
        flow: 'us_onboarding',
        step: 'personal_information',
        data: { full_name: 'Alice', date_of_birth: '1990-01-01', city: 'NYC' },
      },
      {
        command: 'submit',
        user_id: 'ms1',
        flow: 'us_onboarding',
        step: 'identity_verification',
        data: { passport_number: 12345 },
      },
      { command: 'get_flows', user_id: 'ms1' },
    ]);
    expect(results[1]).toEqual({ user: 'ms1', submission_result: 'success' });
    expect(results[2]).toEqual({ user: 'ms1', submission_result: 'success' });
    expect(results[3]).toEqual({ user: 'ms1', flows: [] });
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

describe('unknown users', () => {
  it('returns user_not_found for get_flows on a user that was never created', () => {
    const results = run([{ command: 'get_flows', user_id: 'ghost' }]);
    expect(results).toEqual([{ user: 'ghost', error: 'user_not_found' }]);
  });

  it('emits user_not_found keys in contract order', () => {
    const raw = runRaw([
      { command: 'get_flows', user_id: 'ghost' },
      { command: 'submit', user_id: 'ghost', flow: 'de_onboarding', step: 'x', data: {} },
    ]);
    const keyOrders = (JSON.parse(raw) as object[]).map((r) => Object.keys(r));
    expect(keyOrders).toEqual([
      ['user', 'error'],
      ['user', 'flow', 'error'],
    ]);
  });

  it('returns user_not_found before any flow check on submit', () => {
    const results = run([
      {
        command: 'submit',
        user_id: 'ghost',
        flow: 'no_such_flow',
        step: 'personal_information',
        data: { full_name: 'Alice' },
      },
    ]);
    expect(results).toEqual([{ user: 'ghost', flow: 'no_such_flow', error: 'user_not_found' }]);
  });

  it('does not create a user from a failed create_user', () => {
    const results = run([
      { command: 'create_user', user: { id: 'nu1' } },
      { command: 'get_flows', user_id: 'nu1' },
    ]);
    expect(results[1]).toEqual({ user: 'nu1', error: 'user_not_found' });
  });
});

describe('create_user edge cases', () => {
  it('leaves the stored user untouched on a duplicate create_user', () => {
    const results = run([
      { command: 'create_user', user: { id: 'ce3', country: 'DE' } },
      {
        command: 'submit',
        user_id: 'ce3',
        flow: 'de_onboarding',
        step: 'personal_information',
        data: { full_name: 'Alice' },
      },
      { command: 'create_user', user: { id: 'ce3', country: 'US' } },
      { command: 'get_flows', user_id: 'ce3' },
    ]);
    expect(results[3]).toEqual({
      user: 'ce3',
      flows: [
        {
          name: 'de_onboarding',
          status: 'pending',
          next_step: { name: 'personal_information', fields: ['date_of_birth', 'city'] },
        },
      ],
    });
  });

  it('treats an empty country as missing', () => {
    const results = run([{ command: 'create_user', user: { id: 'ce1', country: '' } }]);
    expect(results).toEqual([
      {
        submission_result: 'failure',
        submission_errors: [{ field: 'country', rule: 'required' }],
      },
    ]);
  });

  it('treats a null country as missing without aborting the batch', () => {
    const results = run([
      { command: 'create_user', user: { id: 'ce4', country: null } },
      { command: 'get_flows', user_id: 'ce4' },
    ]);
    expect(results).toEqual([
      {
        submission_result: 'failure',
        submission_errors: [{ field: 'country', rule: 'required' }],
      },
      { user: 'ce4', error: 'user_not_found' },
    ]);
  });

  it('keeps processing the batch after a failed create_user', () => {
    const results = run([
      { command: 'create_user', user: { id: 'ce2' } },
      { command: 'create_user', user: { id: 'ce2', country: 'DE' } },
    ]);
    expect(results[1]).toEqual({ user: 'ce2', submission_result: 'success' });
  });
});

describe('submit edge cases', () => {
  const submitPersonal = (userId: string, data: Record<string, unknown>) => ({
    command: 'submit',
    user_id: userId,
    flow: 'de_onboarding',
    step: 'personal_information',
    data,
  });

  it('rejects a submission with no fields for the step', () => {
    const results = run([
      { command: 'create_user', user: { id: 'se1', country: 'DE' } },
      submitPersonal('se1', {}),
    ]);
    expect(results[1]).toEqual({
      user: 'se1',
      submission_result: 'failure',
      submission_errors: [{ field: 'data', rule: 'required' }],
    });
  });

  it('rejects a submission that only carries fields outside the step', () => {
    const results = run([
      { command: 'create_user', user: { id: 'se2', country: 'DE' } },
      submitPersonal('se2', { passport_number: 123, extra: 'x' }),
    ]);
    expect(results[1]).toEqual({
      user: 'se2',
      submission_result: 'failure',
      submission_errors: [{ field: 'data', rule: 'required' }],
    });
  });

  it('ignores extra fields alongside valid step fields', () => {
    const results = run([
      { command: 'create_user', user: { id: 'se3', country: 'DE' } },
      submitPersonal('se3', { full_name: 'Alice', extra_field: 'ignored' }),
    ]);
    expect(results[1]).toEqual({ user: 'se3', submission_result: 'success' });
  });

  it('stores nothing when any provided field is invalid', () => {
    const results = run([
      { command: 'create_user', user: { id: 'se4', country: 'DE' } },
      submitPersonal('se4', { full_name: 'Alice', date_of_birth: 'bad' }),
      { command: 'get_flows', user_id: 'se4' },
    ]);
    const flows = (results[2] as { flows: { next_step: { fields: string[] } }[] }).flows;
    expect(flows[0]!.next_step.fields).toEqual(['full_name', 'date_of_birth', 'city']);
  });

  it('returns multiple validation errors in step field definition order', () => {
    const results = run([
      { command: 'create_user', user: { id: 'se5', country: 'DE' } },
      submitPersonal('se5', { city: 456, full_name: 123, date_of_birth: 'bad' }),
    ]);
    expect((results[1] as { submission_errors: unknown[] }).submission_errors).toEqual([
      { field: 'full_name', rule: 'must_be_string' },
      { field: 'date_of_birth', rule: 'must_be_date' },
      { field: 'city', rule: 'must_be_string' },
    ]);
  });

  it('rejects null for a provided field', () => {
    const results = run([
      { command: 'create_user', user: { id: 'se6', country: 'DE' } },
      submitPersonal('se6', { full_name: null }),
    ]);
    expect((results[1] as { submission_errors: unknown[] }).submission_errors).toEqual([
      { field: 'full_name', rule: 'must_be_string' },
    ]);
  });

  it('rejects an impossible calendar date', () => {
    const results = run([
      { command: 'create_user', user: { id: 'se7', country: 'DE' } },
      submitPersonal('se7', { date_of_birth: '2026-02-30' }),
    ]);
    expect((results[1] as { submission_errors: unknown[] }).submission_errors).toEqual([
      { field: 'date_of_birth', rule: 'must_be_date' },
    ]);
  });

  it('accepts an empty string as a valid string field', () => {
    const results = run([
      { command: 'create_user', user: { id: 'se8', country: 'DE' } },
      submitPersonal('se8', { full_name: '', city: '' }),
    ]);
    expect(results[1]).toEqual({ user: 'se8', submission_result: 'success' });
  });

  it('returns not_next_step when resubmitting a completed step', () => {
    const results = run([
      { command: 'create_user', user: { id: 'se9', country: 'US' } },
      {
        command: 'submit',
        user_id: 'se9',
        flow: 'us_onboarding',
        step: 'personal_information',
        data: { full_name: 'Alice', date_of_birth: '1990-01-01', city: 'NYC' },
      },
      {
        command: 'submit',
        user_id: 'se9',
        flow: 'us_onboarding',
        step: 'personal_information',
        data: { full_name: 'Bob' },
      },
    ]);
    expect(results[2]).toEqual({ user: 'se9', flow: 'us_onboarding', error: 'not_next_step' });
  });

  it('returns step_not_found for a step outside the flow', () => {
    const results = run([
      { command: 'create_user', user: { id: 'se10', country: 'DE' } },
      {
        command: 'submit',
        user_id: 'se10',
        flow: 'de_onboarding',
        step: 'identity_verification',
        data: { passport_number: 123 },
      },
    ]);
    expect(results[1]).toEqual({ user: 'se10', flow: 'de_onboarding', error: 'step_not_found' });
  });

  it('returns flow_not_found for prototype pollution flow names', () => {
    const results = run([
      { command: 'create_user', user: { id: 'se11', country: 'DE' } },
      { ...submitPersonal('se11', { full_name: 'A' }), flow: '__proto__' },
      { ...submitPersonal('se11', { full_name: 'A' }), flow: 'constructor' },
    ]);
    expect(results.slice(1)).toEqual([
      { user: 'se11', flow: '__proto__', error: 'flow_not_found' },
      { user: 'se11', flow: 'constructor', error: 'flow_not_found' },
    ]);
  });
});
