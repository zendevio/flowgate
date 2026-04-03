# FlowGate (Rust)

Rust implementation of the FlowGate onboarding flow engine. Produces byte-identical JSON output to the TypeScript version.

## Quick Start

Build and run:

    cargo build --release && cp target/release/onboard .
    ./onboard requirements.json < input.json

Or install system-wide:

    cargo install --path .
    onboard requirements.json < input.json

## Verify

    ./onboard requirements.json < input.json | diff - output.json

## Architecture

Single binary crate with two module trees mirroring the TS layering:

- `domain/` -- pure logic, zero I/O: types, flow evaluation engine, field validation
- `cli/` -- command dispatch, response construction
- `main.rs` -- entry point, arg parsing (clap), stdin/stdout, error reporting

What Rust gives us for free compared to the TS version:

- **Exhaustive match** on command and data type enums replaces `assertNever`
- **`Option<T>`** makes absent-vs-present explicit; no `== null` loose equality tricks
- **serde derives** replace the 45 LOC Zod schema layer entirely
- **No prototype pollution surface** -- `IndexMap::get()` is always an own-key lookup, so the `Object.hasOwn()` guards from the TS handlers are unnecessary
- **Struct field order guarantees** JSON key ordering at compile time, not runtime

See the [root README](../README.md) for full design rationale and spec compliance details.

## Testing

```bash
cargo test
```

89 tests total:

- **70 unit tests** inline in domain modules (`engine.rs`, `validation.rs`) using `rstest` for parameterized cases
- **19 integration tests** in `tests/cli.rs` using `assert_cmd` to run the binary as a subprocess

Coverage includes all spec examples, date calendar validation, numeric string rejection, prototype-like flow names, null field handling, error priority ordering, and exit codes.

## Why Rust?

I implemented this in Rust to explore how Rust's type system and ownership model apply to this domain. The four response shapes are modeled as an untagged enum where invalid states are unrepresentable, which is the strongest correctness improvement over the TS version.

## Dependencies

| Crate | Purpose |
|-------|---------|
| serde + serde_json | JSON serialization with `preserve_order` for key ordering |
| indexmap | Insertion-order maps for flow definition order |
| clap | CLI arg parsing with `--help` and exit code 2 for free |
| thiserror | Error type derives |
| anyhow | Error propagation in main |
