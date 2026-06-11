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
  const expandedInput = applyOrchestrationEventPreset(input);
  const runId = String(expandedInput.runId ?? '').trim();
  if (!runId) {
    throw new Error('runId is required');
  }

  return {
    schemaVersion: Number(expandedInput.schemaVersion ?? orchestrationSchemaVersion),
    id: String(expandedInput.id ?? `evt-${Date.now()}-${randomUUID().slice(0, 8)}`),
    runId,
    timestamp: String(expandedInput.timestamp ?? new Date().toISOString()),
    kind: String(expandedInput.kind ?? 'run.updated'),
    status: String(expandedInput.status ?? 'running'),
    title: optionalString(expandedInput.title),
    message: optionalString(expandedInput.message),
    projectID: optionalString(expandedInput.projectID),
    projectName: optionalString(expandedInput.projectName),
    projectPath: optionalString(expandedInput.projectPath),
    rootLabel: optionalString(expandedInput.rootLabel),
    taskID: optionalString(expandedInput.taskID),
    agentId: optionalString(expandedInput.agentId),
    agentProvider: optionalString(expandedInput.agentProvider),
    agentRole: optionalString(expandedInput.agentRole),
    stepId: optionalString(expandedInput.stepId),
    stepKind: optionalString(expandedInput.stepKind),
    artifactId: optionalString(expandedInput.artifactId),
    artifactKind: optionalString(expandedInput.artifactKind),
    artifactPath: optionalString(expandedInput.artifactPath),
    artifactUrl: optionalString(expandedInput.artifactUrl),
    linkKind: optionalString(expandedInput.linkKind),
    linkLabel: optionalString(expandedInput.linkLabel),
    linkUrl: optionalString(expandedInput.linkUrl)
  };
}

export function applyOrchestrationEventPreset(input) {
  const preset = optionalString(input.preset)?.toLowerCase();
  if (!preset) {
    return input;
  }

  const scenario = optionalString(input.scenario);
  const issueId = optionalString(input.issueId);
  const subject = issueId ?? scenario;
  const counts = orchestrationPresetCountMessage(input);
  const presets = {
    'run-started': {
      kind: 'run.started',
      status: 'running',
      title: 'Run started'
    },
    'run-completed': {
      kind: 'run.completed',
      status: 'succeeded',
      title: 'Run completed'
    },
    'scenario-started': {
      kind: 'scenario.started',
      status: 'running',
      stepKind: 'scenario',
      title: formatPresetTitle('Scenario started', scenario),
      message: scenario
    },
    'scenario-passed': {
      kind: 'scenario.passed',
      status: 'succeeded',
      stepKind: 'scenario',
      title: formatPresetTitle('Scenario passed', scenario),
      message: scenario
    },
    'test-started': {
      kind: 'test.started',
      status: 'running',
      stepKind: 'test',
      title: formatPresetTitle('Test started', subject),
      message: scenario
    },
    'test-failed': {
      kind: 'test.failed',
      status: 'failed',
      stepKind: 'test',
      title: formatPresetTitle('Test failed', subject),
      message: counts ?? scenario
    },
    'fix-started': {
      kind: 'fix.started',
      status: 'running',
      stepKind: 'fix',
      title: formatPresetTitle('Fix started', subject),
      message: scenario
    },
    'fix-resolved': {
      kind: 'fix.resolved',
      status: 'succeeded',
      stepKind: 'fix',
      title: formatPresetTitle('Fix resolved', subject),
      message: counts ?? scenario
    },
    'retest-started': {
      kind: 'retest.started',
      status: 'running',
      stepKind: 'retest',
      title: formatPresetTitle('Retest started', subject),
      message: scenario
    },
    'retest-passed': {
      kind: 'retest.passed',
      status: 'succeeded',
      stepKind: 'retest',
      title: formatPresetTitle('Retest passed', subject),
      message: counts ?? scenario
    },
    'retest-failed': {
      kind: 'retest.failed',
      status: 'failed',
      stepKind: 'retest',
      title: formatPresetTitle('Retest failed', subject),
      message: counts ?? scenario
    },
    'approval-required': {
      kind: 'approval.required',
      status: 'waiting-for-approval',
      stepKind: 'approval',
      title: 'Approval required'
    },
    handoff: {
      kind: 'handoff.available',
      status: 'succeeded',
      stepKind: 'handoff',
      artifactKind: 'handoff',
      title: 'Handoff available'
    }
  };
  const defaults = presets[preset];
  if (!defaults) {
    throw new Error(`Unknown orchestration event preset: ${input.preset}`);
  }

  return {
    ...defaults,
    ...input,
    kind: input.kind ?? defaults.kind,
    status: input.status ?? defaults.status,
    title: input.title ?? defaults.title,
    message: input.message ?? defaults.message,
    stepKind: input.stepKind ?? defaults.stepKind,
    artifactKind: input.artifactKind ?? defaults.artifactKind
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

function formatPresetTitle(prefix, subject) {
  return subject ? `${prefix}: ${subject}` : prefix;
}

function orchestrationPresetCountMessage(input) {
  const parts = [
    formatPresetCount(input.issueCount, 'issue'),
    formatPresetCount(input.resolvedCount, 'resolved'),
    formatPresetCount(input.failedCount, 'failed')
  ].filter(Boolean);

  return parts.length ? parts.join(' · ') : null;
}

function formatPresetCount(value, label) {
  const text = optionalString(value);
  return text ? `${text} ${label}` : null;
}

function printHelp() {
  console.log(`Usage:
  node scripts/orchestrationEvent.mjs --run-id run-tsk-127 --kind test.started --status running
  node scripts/orchestrationEvent.mjs --run-id run-tsk-127 --preset scenario-started --scenario "Trading partner setup"
  node scripts/orchestrationEvent.mjs --run-id run-tsk-127 --preset approval-required --message "Needs deletion sign-off"
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
