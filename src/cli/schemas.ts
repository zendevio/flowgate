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

const CreateUserCommandSchema = z.object({
  command: z.literal('create_user'),
  user: z.object({
    id: z.string(),
    // Nullish here so a missing or null country yields a per-command `required` error
    // instead of a ZodError that would abort the whole batch
    country: z.string().nullish(),
  }),
});

const GetFlowsCommandSchema = z.object({
  command: z.literal('get_flows'),
  user_id: z.string(),
});

const SubmitCommandSchema = z.object({
  command: z.literal('submit'),
  user_id: z.string(),
  flow: z.string(),
  step: z.string(),
  data: z.record(z.string(), z.unknown()),
});

const CommandSchema = z.discriminatedUnion('command', [
  CreateUserCommandSchema,
  GetFlowsCommandSchema,
  SubmitCommandSchema,
]);

export const InputSchema = z.array(CommandSchema);
