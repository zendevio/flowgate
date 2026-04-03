#![forbid(unsafe_code)]

mod cli;
mod domain;

use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

use clap::Parser;

use cli::handlers::handle_command;
use domain::types::{Command, Requirements};
use domain::validation::validate_requirements_integrity;

/// FlowGate: evaluate user onboarding flows and validate step submissions.
#[derive(Parser)]
#[command(name = "onboard", version, about)]
struct Cli {
    /// Path to the requirements JSON file.
    requirements_file: String,
}

/// CLI errors that cause the tool itself to fail (exit 1).
#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("Cannot read requirements file '{path}'")]
    RequirementsRead { path: String, source: io::Error },

    #[error("Failed to parse requirements JSON")]
    RequirementsParse(#[source] serde_json::Error),

    #[error("{0}")]
    RequirementsInvalid(String),

    #[error("Failed to read input from stdin")]
    StdinRead(#[source] io::Error),

    #[error("Failed to parse input JSON from stdin")]
    InputParse(#[source] serde_json::Error),

    #[error("Failed to write output")]
    OutputWrite(#[source] io::Error),
}

pub(crate) type Result<T> = std::result::Result<T, CliError>;

fn main() -> ExitCode {
    // TTY check: if stdin is a terminal, print usage and exit
    if io::stdin().is_terminal() {
        write_stderr("Error: No input on stdin. Usage: ./onboard requirements.json < input.json\n");
        return ExitCode::from(2);
    }

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            write_stderr(&format!("Error: {e}\n"));
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Read and parse requirements file
    let requirements_raw = std::fs::read_to_string(&cli.requirements_file).map_err(|e| {
        CliError::RequirementsRead {
            path: cli.requirements_file.clone(),
            source: e,
        }
    })?;

    let requirements: Requirements =
        serde_json::from_str(&requirements_raw).map_err(CliError::RequirementsParse)?;

    validate_requirements_integrity(&requirements).map_err(CliError::RequirementsInvalid)?;

    // Read and parse stdin
    let mut stdin_raw = String::new();
    io::stdin()
        .read_to_string(&mut stdin_raw)
        .map_err(CliError::StdinRead)?;

    let stdin_clean = stdin_raw.strip_prefix('\u{feff}').unwrap_or(&stdin_raw);

    let commands: Vec<Command> = serde_json::from_str(stdin_clean).map_err(CliError::InputParse)?;

    // Dispatch all commands and collect results
    let results: Vec<_> = commands
        .into_iter()
        .map(|cmd| handle_command(cmd, &requirements))
        .collect();

    // Write JSON output to stdout with trailing newline
    let output =
        serde_json::to_string_pretty(&results).expect("Response serialization should never fail");

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if let Err(e) = writeln!(handle, "{output}") {
        if e.kind() == io::ErrorKind::BrokenPipe {
            return Ok(());
        }
        return Err(CliError::OutputWrite(e));
    }

    Ok(())
}

/// Writes a diagnostic message to stderr, silently ignoring BrokenPipe.
fn write_stderr(msg: &str) {
    let _ = io::stderr().write_all(msg.as_bytes());
}
