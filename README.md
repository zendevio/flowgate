# FlowGate

[![tests](https://img.shields.io/github/actions/workflow/status/zendevio/flowgate/ci.yml?branch=main&label=tests&logo=github)](https://github.com/zendevio/flowgate/actions/workflows/ci.yml)
[![typescript](https://img.shields.io/badge/typescript-7.0-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![node](https://img.shields.io/badge/node-24%20LTS-339933?logo=node.js&logoColor=white)](https://nodejs.org/)
[![rust](https://img.shields.io/badge/rust-1.85%2B-orange?logo=rust)](rust/README.md)
[![license](https://img.shields.io/github/license/zendevio/flowgate)](LICENSE)

CLI tool for evaluating user onboarding flows and validating step submissions. Models a simplified KYC onboarding engine where country-specific flows determine verification requirements.

Started as a stateless CLI, then extended to add in-memory persistence, explicit user creation, and partial step submissions. The original stateless version is tagged `v1.0.0`; the extension is `v1.0.1`.

## Quick Start

**Prerequisites:** Node.js 24+ (LTS)

```bash
npm install
npm run build
./onboard requirements.json < input.json
```

To run tests:

```bash
npm test
```

To verify output matches the expected result:

```bash
npm run verify
```

## How It Works

`requirements.json` defines data types, steps (lists of fields), and flows (conditions plus an ordered list of steps). stdin is a JSON array of commands, processed in order against an in-memory user store that lives for the duration of one run. stdout is a JSON array with one result per command.

### Commands

| Command       | Input                             | Success response                  |
| ------------- | --------------------------------- | --------------------------------- |
| `create_user` | `user: { id, country }`           | `{ user, submission_result }`     |
| `get_flows`   | `user_id`                         | `{ user, flows }` (pending flows) |
| `submit`      | `user_id`, `flow`, `step`, `data` | `{ user, submission_result }`     |

Failure responses:

| Situation                         | Response                                                        |
| --------------------------------- | --------------------------------------------------------------- |
| `create_user` with no country     | `{ submission_result, submission_errors: [country/required] }`  |
| `create_user` with a duplicate id | `{ submission_result, submission_errors: [user_id/duplicate] }` |
| Unknown user on `get_flows`       | `{ user, error: 'user_not_found' }`                             |
| Structural error on `submit`      | `{ user, flow, error }`                                         |
| Invalid or missing submit data    | `{ user, submission_result, submission_errors }`                |

### Submit error priority

1. `user_not_found`: the user was never created
2. `flow_not_found`: the flow does not exist, or its conditions do not match the user
3. `step_not_found`: the step is not part of this flow
4. `not_next_step`: the step is not the current step (a partially filled step is still current)
5. Field validation: `must_be_string`, `must_be_number`, `must_be_date`, ordered by step field definition

## Architecture

Two-layer clean architecture with inward-only dependencies:

```
src/
  domain/       Pure logic, zero external deps, zero I/O
    types.ts      String unions, interfaces, UserStore type, assertNever
    engine.ts     Flow evaluation, condition matching, step progression, data merging
    validation.ts Field type validation, date checks, requirements integrity
    index.ts      Barrel export
  cli/          I/O, Zod schemas, system boundary
    main.ts       EPIPE handlers, TTY check, arg parsing, stdin, store creation, dispatch, stdout
    handlers.ts   Command dispatch, response builders
    schemas.ts    Zod schemas for requirements and input validation
    index.ts      Barrel export
```

The domain layer never imports from cli/ and holds no state: every domain function takes the user and requirements as arguments. State lives in a single `Map` created in `main.ts` and passed to the handlers, so each run starts from an empty store and tests stay isolated.

**Data flow:**

```
stdin JSON -> Zod parse -> new UserStore -> for each command:
  -> handleCommand (dispatch on command type)
    -> create_user: country check -> duplicate check -> store
    -> get_flows:   store lookup -> evaluateFlows -> pending flows only
    -> submit:      store lookup -> flow/step checks -> validate provided fields -> merge and store
  -> JSON.stringify(results) -> stdout
```

**Why 2 layers, not 4?** A full clean architecture (domain/application/infrastructure/cli) would produce two single-file layers at this size. Two directories with a clear boundary shows the same principle without the ceremony.

## Design Decisions

### Injected store, not module state

The user store is created per run in `main.ts` and passed into `handleCommand`. A module-level `Map` would also work for a single CLI invocation, but it makes the handlers impure, leaks state between in-process callers, and hides the dependency. Swapping the `Map` for a database-backed repository later only changes the type passed in.

### Country is optional in the schema on purpose

The spec requires a per-command `{ field: 'country', rule: 'required' }` response when country is missing. Marking `country` as required in the Zod schema would make the whole input fail to parse, aborting the batch instead of answering that one command. So the schema validates shape, and the handler enforces the business rule. The same reasoning applies to any rule that must produce a structured per-command error.

### Partial submissions, but never empty ones

A submit validates only the step fields present in `data`, merges them into the stored user, and leaves the step pending until every field is filled. A submit that contains none of the step's fields is rejected with `{ field: 'data', rule: 'required' }`: accepting it would report success while storing nothing. A failed submit never stores anything, even the valid fields in it.

### Filtered merge

Only fields defined in the step are merged onto the user. A raw `{ ...user, ...data }` spread would let fields like `id` or `country` be overwritten. Key presence is checked with `Object.hasOwn`, never `in`, so inherited names such as `constructor` cannot match.

### Zod at the boundary, domain types inside

Zod validates external input (requirements file and stdin commands) at the CLI layer. The domain layer uses plain TypeScript interfaces with no runtime validation library, which keeps it pure and portable (the Rust port mirrors it one to one).

### Response builders over error classes

Each response shape is built by a dedicated function with exact key ordering. At this size, builder functions are simpler than a class hierarchy, and they guarantee the JSON key order the contract requires. The example test compares raw stdout byte for byte, so key order is enforced by tests, not just by convention.

### Proportionality

I dropped several production patterns that would be disproportionate at this scale:

- **Branded types** (UserId, FlowId, StepId): worth adding now that IDs are passed around more, but still low risk at this size
- **Result types**: exceptions for real failures, response builders for expected ones
- **ESLint**: `prettier + tsc --noEmit` covers formatting and type safety
- **4-layer architecture**: two layers are enough when application/infrastructure would each be one file

## Edge Cases and Assumptions

| Ambiguity                                 | Decision                                             | Rationale                                                          |
| ----------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------ |
| Command for a user that was never created | `user_not_found` (checked first on submit)           | Distinguishes "no such user" from "no matching flow"               |
| Submit with no fields for the step        | Failure: `{ field: 'data', rule: 'required' }`       | Success must mean something was stored                             |
| Field present with `null` value           | Validation error for that field                      | `null` is not a valid string, number, or date                      |
| Empty string for country                  | Treated as missing                                   | An empty country cannot match any flow                             |
| Both country missing and id duplicate     | Country error reported                               | Validate the request before checking state                         |
| "Valid YYYY-MM-DD" meaning                | Full calendar validation (leap years, days in month) | "Valid" implies a real date, not just a regex match                |
| Conditions matching semantics             | AND (all must match), missing key = no match         | Multiple keys in a conditions object naturally implies conjunction |
| Re-submitting a completed step            | `not_next_step`                                      | A completed step is not the current step                           |
| Extra fields in submit data               | Silently ignored                                     | Validation iterates step fields, not data keys                     |
| Completed flows in `get_flows`            | Omitted                                              | Required by the extension spec                                     |

## Testing

117 tests across 3 suites:

- **Unit: validation** (56 tests): table-driven `it.each` for `isValidDate`, `validateFieldType` across string/number/date including arrays and objects, plus `validateStepFields` and `validateRequirementsIntegrity`
- **Unit: engine** (23 tests): `matchesConditions`, `isStepComplete`, `findNextStep` (remaining fields), `mergeStepData`, `evaluateFlows`
- **Integration: CLI** (38 tests): byte-identical comparison against `output.json`, plus `create_user` rules, unknown users, partial submissions across commands, empty and invalid submissions, failed commands leaving state untouched, error priority, prototype-like flow names, and response key order

```bash
npm test                # Run all tests
npm run test:coverage   # Run with coverage report
npm run verify          # Compare output against expected
```

## What I'd Add With More Time

**Persistent storage.** Replace the `Map` with a repository interface backed by a database. Because the store is already injected, this is a change at the edge, not in the domain.

**Compound conditions.** Support AND/OR operators and range checks in flow conditions, keeping the current flat format as shorthand for equality.

**New field types.** Phone numbers, document references. The `validateFieldType` switch makes this one case per type; a validator registry would formalize it.

**Structured logging.** JSON log lines with correlation IDs per command, and an audit log of every submit attempt (user ID and flow name only, no submitted values), which matters for compliance in KYC systems.

**Input size limits.** A configurable max stdin buffer to reject oversized input before parsing.

**Schema versioning.** A `version` field in requirements.json, validated at startup.

## AI Usage

I used AI coding assistants throughout, and every design decision was mine to make and defend:

- **Original version:** Claude Code for researching the spec and its ambiguities, drafting source, tests, and configuration from my architectural decisions, and reviewing against the spec.
- **Extension:** implemented with Claude Code, with me directing the approach (test-first, types first, then handlers) and reviewing the result.
- **Cleanup:** Claude Code and OpenAI Codex for injecting the store, adding `user_not_found` and empty-submission handling, restoring edge-case tests, porting the extension to Rust, and independent code review of each change.

I referenced public open-source TypeScript codebases for conventions and layered on additional strictness appropriate for a backend CLI.

## Technologies

| Technology | Version | Purpose                              |
| ---------- | ------- | ------------------------------------ |
| TypeScript | 7.0     | Language (strict mode, ESM)          |
| Node.js    | 24 LTS  | Runtime                              |
| Zod        | 4.6     | Schema validation at system boundary |
| Jest       | 30.5    | Testing (E2E + unit)                 |
| Prettier   | 3.9     | Code formatting                      |

## Rust Implementation

A Rust implementation with the same behaviour and byte-identical output is in `rust/`. See [`rust/README.md`](rust/README.md) for build instructions and details.
