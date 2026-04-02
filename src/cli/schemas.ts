import { z } from 'zod';

const DataTypeSchema = z.enum(['string', 'number', 'date']);

const StepSchema = z.object({
  fields: z.array(z.string()),
});

const FlowSchema = z.object({
  conditions: z.record(z.string(), z.string()),
  steps: z.array(z.string()),
});

export const RequirementsSchema = z.object({
  data_types: z.record(z.string(), DataTypeSchema),
  steps: z.record(z.string(), StepSchema),
  flows: z.record(z.string(), FlowSchema),
});

const UserSchema = z.object({ id: z.string() }).passthrough();

const GetFlowsCommandSchema = z.object({
  command: z.literal('get_flows'),
  user: UserSchema,
});

const SubmitCommandSchema = z.object({
  command: z.literal('submit'),
  user: UserSchema,
  flow: z.string(),
  step: z.string(),
  data: z.record(z.string(), z.unknown()),
});

const CommandSchema = z.discriminatedUnion('command', [
  GetFlowsCommandSchema,
  SubmitCommandSchema,
]);

export const InputSchema = z.array(CommandSchema);
