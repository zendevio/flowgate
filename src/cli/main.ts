import { readFileSync } from 'node:fs';
import process from 'node:process';
import { RequirementsSchema, InputSchema } from './schemas.js';
import { handleCommand } from './handlers.js';
import { validateRequirementsIntegrity, type UserStore } from '../domain/index.js';

// EPIPE handlers (must be registered at startup)
process.stdout.on('error', (err: NodeJS.ErrnoException) => {
  if (err.code === 'EPIPE') {
    process.exit(0);
  }
  throw err;
});

process.stderr.on('error', (err: NodeJS.ErrnoException) => {
  if (err.code === 'EPIPE') {
    return;
  }
  throw err;
});

async function main(): Promise<void> {
  // TTY check: if stdin is a terminal, print usage and exit
  if (process.stdin.isTTY) {
    process.stderr.write(
      'Error: No input on stdin. Usage: ./onboard requirements.json < input.json\n'
    );
    process.exit(2);
  }

  // Read requirements file from CLI argument
  const requirementsPath = process.argv[2];
  if (!requirementsPath) {
    process.stderr.write(
      'Error: Missing requirements file argument. Usage: ./onboard requirements.json < input.json\n'
    );
    process.exit(2);
  }

  let requirementsRaw: string;
  try {
    requirementsRaw = readFileSync(requirementsPath, 'utf-8');
  } catch {
    process.stderr.write(`Error: Cannot read requirements file '${requirementsPath}'\n`);
    process.exit(1);
  }

  const requirements = RequirementsSchema.parse(JSON.parse(requirementsRaw));
  validateRequirementsIntegrity(requirements);

  // Read stdin
  const chunks: string[] = [];
  process.stdin.setEncoding('utf-8');
  for await (const chunk of process.stdin) {
    chunks.push(chunk as string);
  }
  const stdinRaw = chunks.join('').replace(/^\ufeff/, '');

  // Parse commands and dispatch
  const commands = InputSchema.parse(JSON.parse(stdinRaw));
  const store: UserStore = new Map();
  const results = commands.map((cmd) => handleCommand(cmd, requirements, store));

  process.stdout.write(JSON.stringify(results, null, 2) + '\n');
}

main().catch((err: unknown) => {
  const message = err instanceof Error ? err.message : String(err);
  process.stderr.write(`Error: ${message}\n`);
  process.exit(1);
});
