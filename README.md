# FlowGate

CLI tool for evaluating user onboarding flows and validating step submissions. Models a simplified KYC onboarding engine where country-specific flows determine verification requirements.

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

## Architecture

Two-layer clean architecture with inward-only dependencies:

```
src/
  domain/       Pure logic, zero external deps, zero I/O
    types.ts      String unions, interfaces, assertNever
    engine.ts     Flow evaluation, condition matching, step progression, data merging
    validation.ts Field type validation, date checks, requirements integrity
    index.ts      Barrel export
  cli/          I/O, Zod schemas, system boundary
    main.ts       EPIPE handlers, TTY check, arg parsing, stdin, dispatch, stdout
    handlers.ts   Command dispatch, response builders (4 shapes)
    schemas.ts    Zod schemas for requirements and input validation
    index.ts      Barrel export
```

The domain layer never imports from cli/. The system is a pure function: given the same user state and requirements, it always produces the same output.

**Data flow:**

```
stdin JSON -> Zod parse -> for each command:
  -> handleCommand (dispatch on command type)
    -> get_flows: evaluateFlows -> buildGetFlowsResponse
    -> submit:    flow/step/validation checks -> merge or error -> response builder
  -> JSON.stringify(results) -> stdout
```

**Why 2 layers, not 4?** A full clean architecture (domain/application/infrastructure/cli) would produce two single-file layers for ~450 LOC. Two directories with a clear boundary shows the same principle without the ceremony. The application layer would just delegate, and the infrastructure layer would just read files. I mention this as a scaling pattern in "What I'd Add."

## Design Decisions

### E2E-first testing

Integration tests run the actual `./onboard` binary as a subprocess and compare JSON output. Unit tests supplement these for algorithmic code only (date validation, type checking, condition matching). This validates real behavior rather than implementation details.

### Filtered merge on submit success

On a successful submit, only fields defined in the step are merged onto the user object. A raw `{ ...user, ...data }` spread would let extra fields like `country` or `id` overwrite user properties. The merge iterates `stepFields` and picks only matching keys from data.

### Zod at the boundary, domain types inside

Zod validates external input (requirements file and stdin commands) at the CLI layer. The domain layer uses plain TypeScript interfaces with no runtime validation library. This keeps the domain pure and portable.

### Response builders over error classes

Four response shapes, each built by a dedicated function with exact key ordering. For ~450 LOC, builder functions are simpler than a class hierarchy. The builders guarantee JSON key order matches the spec, which matters for output matching.

### Proportionality

I dropped several production patterns that would be disproportionate at this scale:

- **Branded types** (FlowId, StepId): No ID-swap risk in a stateless CLI
- **Result types**: Exceptions are simpler here; Result<T, E> is a scaling pattern
- **ESLint**: `prettier + tsc --noEmit` covers formatting and type safety
- **4-layer architecture**: Two layers are enough when application/infrastructure would each be one file

Each of these is worth adding as the codebase grows.

## Edge Cases and Assumptions

| Ambiguity                         | Decision                                                          | Rationale                                                               |
| --------------------------------- | ----------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Missing fields in submit data     | Validation error with expected data_type as `rule`                | `undefined` fails every type check; success always means step completed |
| `not_next_step` response shape    | Structural error: `{ user, flow, error }`                         | Pattern-matches the other structural errors (zero examples in spec)     |
| User object field type validation | Presence-only for step completion; type validation only on submit | Spec scopes type rules under the "submit > Rules" section               |
| "Valid YYYY-MM-DD" meaning        | Full calendar validation (leap years, days in month)              | "Valid" implies a real date, not just a regex match                     |
| Conditions matching semantics     | AND (all must match), missing key = no match                      | Multiple keys in a conditions object naturally implies conjunction      |
| Re-submitting a completed step    | Returns `not_next_step`                                           | A completed step is not the current next step                           |
| Extra fields in submit data       | Silently ignored                                                  | Validation iterates step fields, not data keys                          |

## Testing

92 tests across 3 suites:

- **Unit: validation** (52 tests) - Table-driven `it.each` for `isValidDate` (17 cases), `validateFieldType` (26 cases across string/number/date including arrays and objects), plus `validateStepFields` and `validateRequirementsIntegrity`
- **Unit: engine** (21 tests) - `matchesConditions`, `isStepComplete`, `findNextStep`, `mergeStepData`, `evaluateFlows`, null field handling
- **Integration: CLI** (19 tests) - Full example output comparison, plus edge cases: `not_next_step`, missing fields, numeric strings, extra fields, invalid dates, impossible calendar dates, empty strings, multi-step completion, prototype pollution guards, null field values, error exit codes

```bash
npm test                # Run all tests
npm run test:coverage   # Run with coverage report
npm run verify          # Compare output against expected
```

## What I'd Add With More Time

**Compound conditions.** Support AND/OR operators and range checks in flow conditions. The `matchesConditions` function currently only handles exact equality on all keys. I would introduce a condition evaluator that supports `{ operator: 'gte', field: 'age', value: 18 }` style rules, keeping the current flat format as syntactic sugar for `{ operator: 'eq' }`.

**New field types.** Phone number patterns, SSN format, document upload references. The `validateFieldType` switch makes adding types straightforward: one new case per type with its validation logic. A validator registry pattern would formalize this.

**Structured logging.** Pino for JSON log lines with severity, timestamps, and correlation IDs per command. Audit logging for every submit attempt (success or failure) with user ID and flow name, which matters for compliance in KYC systems.

**Input size limits.** Configurable max stdin buffer (default 10MB) to prevent out-of-memory on malformed input. Reject before buffering with a clear error message.

**Schema versioning.** A `version` field in requirements.json, validated at startup. Reject unsupported versions with a migration message rather than producing confusing parse errors.

**Docker distribution.** Multi-stage Dockerfile: build stage with `npm ci && npm run build`, runtime stage with only `dist/` and production dependencies. Keeps the image minimal.

## AI Usage

I used Claude Code as a development assistant throughout this project. Specifically:

- **Research and planning:** Analyzing the spec, identifying edge cases, resolving ambiguities, researching TypeScript/Jest/Zod ecosystem compatibility
- **Code generation:** Drafting source files, tests, and configuration based on my architectural decisions
- **Validation:** Reviewing code against the spec, checking for correctness issues, verifying edge case handling

All architectural decisions, design trade-offs, and proportionality judgments are my own. I referenced public open-source TypeScript codebases for conventions and layered on additional strictness appropriate for a backend CLI. Every line of code is defensible and I can walk through any part of the implementation.

## Technologies

| Technology | Version | Purpose                              |
| ---------- | ------- | ------------------------------------ |
| TypeScript | 6.0.2   | Language (strict mode, ESM)          |
| Node.js    | 24 LTS  | Runtime                              |
| Zod        | 4.3.6   | Schema validation at system boundary |
| Jest       | 30.3.0  | Testing (E2E + unit)                 |
| Prettier   | 3.5+    | Code formatting                      |

## Rust Implementation

A Rust implementation producing byte-identical output is available in `rust/`. See [`rust/README.md`](rust/README.md) for build instructions and details.
