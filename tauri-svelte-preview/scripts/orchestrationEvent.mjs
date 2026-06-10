import { randomUUID } from 'node:crypto';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export const orchestrationSchemaVersion = 1;
export const orchestrationEventStoreEnv = 'MAC_COMMAND_BAR_ORCHESTRATION_EVENTS';

const explicitKeyMap = new Map([
  ['run-id', 'runId'],
  ['project-id', 'projectID'],
  ['task-id', 'taskID']
]);

export function defaultOrchestrationEventStorePath(homeDirectory = os.homedir()) {
  return path.join(
    homeDirectory,
    'Library',
    'Application Support',
    'MacCommandBar',
    'orchestration-events.jsonl'
  );
}

export function orchestrationEventStorePath(env = process.env) {
  const configuredPath = env[orchestrationEventStoreEnv]?.trim();
  return configuredPath || defaultOrchestrationEventStorePath();
}

export function normalizeOrchestrationEvent(input) {
  const runId = String(input.runId ?? '').trim();
  if (!runId) {
    throw new Error('runId is required');
  }

  return {
    schemaVersion: Number(input.schemaVersion ?? orchestrationSchemaVersion),
    id: String(input.id ?? `evt-${Date.now()}-${randomUUID().slice(0, 8)}`),
    runId,
    timestamp: String(input.timestamp ?? new Date().toISOString()),
    kind: String(input.kind ?? 'run.updated'),
    status: String(input.status ?? 'running'),
    title: optionalString(input.title),
    message: optionalString(input.message),
    projectID: optionalString(input.projectID),
    projectName: optionalString(input.projectName),
    projectPath: optionalString(input.projectPath),
    rootLabel: optionalString(input.rootLabel),
    taskID: optionalString(input.taskID),
    agentId: optionalString(input.agentId),
    agentProvider: optionalString(input.agentProvider),
    agentRole: optionalString(input.agentRole),
    stepId: optionalString(input.stepId),
    stepKind: optionalString(input.stepKind),
    artifactId: optionalString(input.artifactId),
    artifactKind: optionalString(input.artifactKind),
    artifactPath: optionalString(input.artifactPath),
    artifactUrl: optionalString(input.artifactUrl),
    linkKind: optionalString(input.linkKind),
    linkLabel: optionalString(input.linkLabel),
    linkUrl: optionalString(input.linkUrl)
  };
}

export async function appendOrchestrationEvent(input, options = {}) {
  const event = normalizeOrchestrationEvent(input);
  const storePath = options.storePath ?? orchestrationEventStorePath(options.env);
  await fs.mkdir(path.dirname(storePath), { recursive: true });
  await fs.appendFile(storePath, `${JSON.stringify(event)}\n`, 'utf8');
  return { event, storePath };
}

export function parseOrchestrationEventArgs(args) {
  const event = {};
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === '--help' || arg === '-h') {
      event.help = true;
      continue;
    }
    if (arg === '--json') {
      const value = args[index + 1];
      if (!value) {
        throw new Error('--json requires a JSON object value');
      }
      Object.assign(event, JSON.parse(value));
      index += 1;
      continue;
    }
    if (!arg.startsWith('--')) {
      throw new Error(`Unexpected argument: ${arg}`);
    }

    const key = arg.slice(2);
    const value = args[index + 1];
    if (value === undefined || value.startsWith('--')) {
      throw new Error(`${arg} requires a value`);
    }
    event[argumentKeyToEventKey(key)] = value;
    index += 1;
  }

  return event;
}

function argumentKeyToEventKey(key) {
  if (explicitKeyMap.has(key)) {
    return explicitKeyMap.get(key);
  }
  return key.replace(/-([a-z])/g, (_, character) => character.toUpperCase());
}

function optionalString(value) {
  if (value === undefined || value === null) {
    return null;
  }
  const text = String(value).trim();
  return text ? text : null;
}

function printHelp() {
  console.log(`Usage:
  node scripts/orchestrationEvent.mjs --run-id run-tsk-127 --kind test.started --status running
  node scripts/orchestrationEvent.mjs --json '{"runId":"run-tsk-127","kind":"run.created"}'

Writes one JSONL event to:
  $${orchestrationEventStoreEnv}, or
  ${defaultOrchestrationEventStorePath()}`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  try {
    const event = parseOrchestrationEventArgs(process.argv.slice(2));
    if (event.help) {
      printHelp();
      process.exit(0);
    }
    const result = await appendOrchestrationEvent(event);
    console.log(JSON.stringify(result, null, 2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
