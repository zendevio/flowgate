# FlowGate (Rust)

Rust implementation of the FlowGate onboarding flow engine, including the stateful extension (`create_user`, in-memory user store, partial submissions). Produces byte-identical JSON output to the TypeScript version.

## Quick Start

**Prerequisites:** Rust 1.85+ (stable)

Build and run:

    cargo build --release && cp target/release/onboard .
    ./onboard requirements.json < input.json

Or install system-wide:

    cargo install --path .
    onboard requirements.json < input.json

## Verify

    ./onboard requirements.json < input.json | diff - output.json

## Architecture

Single binary crate with two module trees separating pure domain logic from I/O and CLI concerns:

- `domain/`: pure logic, zero I/O: types, flow evaluation engine, field validation
- `cli/`: command dispatch, response construction
- `main.rs`: entry point, arg parsing (clap), stdin/stdout, user store creation, error reporting

The user store is a map created once in `main.rs` and passed to every handler by mutable reference. There is no global or static state, so each run starts empty and the handlers stay testable in isolation.

What Rust brings to this problem:

- **Exhaustive match** on command and data type enums: the compiler enforces every variant is handled
- **`Option<T>`** makes absent-vs-present explicit at the type level, which is exactly the distinction partial submissions depend on
- **Ownership** makes the store's single writer explicit: only handlers given `&mut` access can change it
- **serde derives** handle JSON parsing and validation declaratively
- **No prototype pollution surface**: `IndexMap::get()` is always an own-key lookup by design
- **Struct field order** guarantees JSON key ordering at compile time

See the [root README](../README.md) for the command contract, error priority, and design rationale.

## Testing

```bash
cargo test
```

120 tests total:

- **79 unit tests** inline in `engine.rs`, `validation.rs`, and `handlers.rs` using `rstest` for parameterized cases
- **41 integration tests** in `tests/cli.rs` using `assert_cmd` to run the binary as a subprocess

The integration suite covers every TypeScript integration scenario plus Rust-specific parsing cases, and the example test compares raw stdout bytes against `output.json`, so key order and formatting are enforced.

## Known Differences From the TypeScript Version

Both implementations produce identical output on every case in both test suites and on a set of adversarial inputs used during review. The known differences are all inputs that are not realistic for this tool, where `serde_json` is stricter than `JSON.parse`:

| Input                                        | TypeScript                                    | Rust                          |
| -------------------------------------------- | --------------------------------------------- | ----------------------------- |
| Number outside the f64 range (e.g. `1e400`)  | Parsed as `Infinity`, then a validation error | Whole input rejected, exit 1  |
| Lone UTF-16 surrogate escape (`"\ud800"`)    | Accepted                                      | Whole input rejected, exit 1  |
| Nesting deeper than 128 levels               | Accepted                                      | Whole input rejected, exit 1  |
| Invalid UTF-8 bytes on stdin                 | Replaced with U+FFFD and processed            | Whole input rejected, exit 1  |
| Duplicate top-level key in requirements.json | Last value wins                               | Requirements rejected, exit 1 |

## Why Rust?

I implemented this in Rust to explore how Rust's type system and ownership model apply to this domain. Each response shape is its own struct inside an untagged enum, so a response cannot carry fields from another shape (for example `flows` on a submit result), and struct field order fixes the JSON key order at compile time.

## Dependencies

| Crate              | Purpose                                                   |
| ------------------ | --------------------------------------------------------- |
| serde + serde_json | JSON serialization with `preserve_order` for key ordering |
| indexmap           | Insertion-order maps for flow definition order            |
| clap               | CLI arg parsing with `--help` and exit code 2 for free    |
| thiserror          | Error type derives                                        |
