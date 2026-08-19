/**
 * Pure rules for the thread-first session draft.
 *
 * The pane owns the interaction and the route owns the side effects. This
 * module is deliberately plain data only: it validates the draft, groups the
 * provider configuration already known to the shell, and assembles the one
 * request sent on the first message.
 */

import { sessionTitleFromPrompt } from '../sessionStrip.ts';

export type ThreadStartProvider = 'codex' | 'claude' | 'antigravity';

export type ThreadStartProviderConfig = {
  provider: string;
  model: string | null;
  availableModels: string[];
  reasoningEffort: string | null;
  availableEfforts: string[];
  approvalPolicy: string | null;
  availableApprovalPolicies: string[];
};

export type ThreadStartModel = {
  id: string;
  label: string;
  hint: string;
  available: boolean;
  unavailableReason: string | null;
};

export type ThreadStartModelGroup = {
  provider: ThreadStartProvider;
  label: string;
  models: ThreadStartModel[];
};

export type ThreadStartPickerState = {
  prompt: string;
  provider: ThreadStartProvider;
  model: string;
  effort: string;
  access: string;
  projectPath: string;
  cwd: string;
  branch: string;
  createNewWorktree: boolean;
};

export type ThreadStartProblem = {
  field: 'prompt' | 'project' | 'branch' | 'worktree';
  message: string;
};

export type ThreadStartRequest = {
  prompt: string;
  provider: ThreadStartProvider;
  model: string | null;
  reasoningEffort: string | null;
  approvalPolicy: string | null;
  projectPath: string;
  cwd: string;
  branch: string;
  createNewWorktree: boolean;
  title: string;
};

export type ThreadStartGitRef = {
  name: string;
  checkoutPath: string | null;
};

export type ThreadStartProject = {
  path: string;
  name: string;
};

const PROVIDER_ORDER: readonly ThreadStartProvider[] = ['codex', 'claude', 'antigravity'];

const PROVIDER_LABELS: Record<ThreadStartProvider, string> = {
  codex: 'OpenAI',
  claude: 'Anthropic',
  antigravity: 'Antigravity'
};

/**
 * What each provider is known to offer, for a draft with no session to ask.
 *
 * The lists a running session reports win whenever there is one. These are
 * the ids the providers actually take — Claude's adapter names its models
 * `sonnet`/`opus`/`haiku` and its access levels `acceptEdits`, not the
 * spellings a draft once made up and had refused at the first send.
 */
const FALLBACK_MODELS: Record<ThreadStartProvider, readonly string[]> = {
  codex: ['gpt-5.6-luna', 'gpt-5.6-sol', 'gpt-5.6-terra'],
  claude: ['default', 'sonnet', 'haiku', 'opus', 'claude-opus-5', 'claude-fable-5'],
  // Antigravity names a model by the display name its own `models` command
  // prints, and its speed tiers are separate models rather than an effort
  // setting. The first is the one it starts on when nothing is chosen.
  antigravity: [
    'Gemini 3.7 Flash (High)',
    'Gemini 3.7 Flash (Medium)',
    'Gemini 3.7 Flash (Low)',
    'Gemini 3.1 Pro (High)',
    'Gemini 3.1 Pro (Low)'
  ]
};

const MODEL_HINTS: Record<string, string> = {
  'gpt-5.6-luna': 'balanced build work',
  'gpt-5.6-sol': 'deep review',
  'gpt-5.6-terra': 'fast iteration',
  opus: 'complex work',
  sonnet: 'everyday tasks',
  haiku: 'quick lookups'
};

const FALLBACK_EFFORTS: Record<ThreadStartProvider, string> = {
  codex: 'max',
  claude: 'medium',
  antigravity: ''
};

const FALLBACK_EFFORT_CHOICES: Record<ThreadStartProvider, readonly string[]> = {
  codex: ['low', 'medium', 'high', 'xhigh', 'max'],
  claude: ['low', 'medium', 'high', 'max'],
  antigravity: []
};

const FALLBACK_ACCESS: Record<ThreadStartProvider, string> = {
  codex: 'on-request',
  claude: 'acceptEdits',
  // Antigravity has one setting and no way to change it: its adapter cannot
  // pass a permission question on, so it runs everything without asking.
  antigravity: 'bypassPermissions'
};

const FALLBACK_ACCESS_CHOICES: Record<ThreadStartProvider, readonly string[]> = {
  codex: ['untrusted', 'on-request', 'never'],
  claude: ['default', 'acceptEdits', 'plan', 'dontAsk', 'bypassPermissions'],
  antigravity: ['bypassPermissions']
};

function providerFor(value: string): value is ThreadStartProvider {
  return PROVIDER_ORDER.includes(value as ThreadStartProvider);
}

function tidy(value: string | null | undefined): string {
  return typeof value === 'string' ? value.trim() : '';
}

function unique(values: readonly string[]): string[] {
  return [...new Set(values.map(tidy).filter(Boolean))];
}

export function deriveThreadStartProjects(paths: readonly string[]): ThreadStartProject[] {
  return unique(paths)
    .filter((path) => path.startsWith('/'))
    .map((path) => ({
      path: path === '/' ? path : path.replace(/\/+$/, ''),
      name: path.split('/').filter(Boolean).at(-1) ?? path
    }))
    .filter((project, index, projects) =>
      projects.findIndex((candidate) => candidate.path === project.path) === index
    );
}

export function filterThreadStartGitRefs<T extends Pick<ThreadStartGitRef, 'name'>>(
  refs: readonly T[],
  search: string,
  limit = 100
): { visible: T[]; total: number } {
  const query = tidy(search).toLocaleLowerCase();
  const matches = query
    ? refs.filter((ref) => ref.name.toLocaleLowerCase().includes(query))
    : [...refs];
  return { visible: matches.slice(0, Math.max(0, limit)), total: matches.length };
}

export function canSelectThreadStartGitRef(
  ref: ThreadStartGitRef,
  canCreateWorktree: boolean
): boolean {
  return Boolean(ref.checkoutPath) || canCreateWorktree;
}

function modelLabel(model: string): string {
  const parts = model
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map((part) => (/^\d/.test(part) ? part : `${part[0].toUpperCase()}${part.slice(1)}`));
  if (parts.length > 1 && /^(gpt|claude|anthropic|openai)$/i.test(parts[0])) parts.shift();
  return parts.join(' ') || model;
}

function modelHint(model: string): string {
  const key = model.toLowerCase();
  return MODEL_HINTS[key]
    ?? (key.includes('fast') || key.includes('haiku') ? 'quick lookups' : 'general build work');
}

/**
 * Convert the config snapshots already loaded for existing sessions into the
 * provider-grouped menu used by a fresh draft. Current values are included
 * even when a provider's advertised list has changed between sessions.
 */
export function groupProviderModels(
  configs: readonly ThreadStartProviderConfig[]
): ThreadStartModelGroup[] {
  const byProvider = new Map<ThreadStartProvider, string[]>();
  const currentByProvider = new Map<ThreadStartProvider, string>();
  for (const provider of PROVIDER_ORDER) byProvider.set(provider, []);

  for (const config of configs) {
    if (!providerFor(config.provider)) continue;
    const models = byProvider.get(config.provider) ?? [];
    const currentModel = tidy(config.model);
    if (currentModel) currentByProvider.set(config.provider, currentModel);
    const next = unique([...(currentModel ? [currentModel] : []), ...config.availableModels]);
    byProvider.set(config.provider, unique([...models, ...next]));
  }

  return PROVIDER_ORDER.map((provider) => {
    const configured = byProvider.get(provider) ?? [];
    const current = currentByProvider.get(provider);
    const snapshots = configs.filter((config) => config.provider === provider);
    const advertised = new Set(snapshots.flatMap((config) => unique(config.availableModels)));
    const availabilityKnown = advertised.size > 0;
    // A remembered choice with no session behind it is one model, not a list:
    // the known list goes with it until a session reports its own.
    const models = availabilityKnown
      ? unique([current ?? '', ...configured])
      : unique([current ?? '', ...configured, ...FALLBACK_MODELS[provider]]);
    return {
      provider,
      label: PROVIDER_LABELS[provider],
      models: models.map((id) => {
        const available = !availabilityKnown || advertised.has(id);
        return {
          id,
          label: modelLabel(id),
          hint: modelHint(id),
          available,
          unavailableReason: available ? null : 'Unavailable in the current session service.'
        };
      })
    };
  });
}

function configForProvider(
  provider: ThreadStartProvider,
  configs: readonly ThreadStartProviderConfig[]
): ThreadStartProviderConfig | null {
  return configs.find((config) => config.provider === provider) ?? null;
}

function firstConfiguredModel(
  provider: ThreadStartProvider,
  configs: readonly ThreadStartProviderConfig[]
): string {
  const config = configForProvider(provider, configs);
  const available = unique(config?.availableModels ?? []);
  const current = tidy(config?.model);
  return (current && (!available.length || available.includes(current)) ? current : '')
    || available[0]
    || current
    || FALLBACK_MODELS[provider][0]
    || '';
}

/** The values painted when the pane first opens. No session is created here. */
export function defaultThreadStartState(input: {
  projectPath: string;
  cwd?: string;
  branch?: string;
  provider?: ThreadStartProvider;
  providerConfigs?: readonly ThreadStartProviderConfig[];
}): ThreadStartPickerState {
  const configs = input.providerConfigs ?? [];
  const provider: ThreadStartProvider = input.provider ?? 'codex';
  const config = configForProvider(provider, configs);
  return {
    prompt: '',
    provider,
    model: firstConfiguredModel(provider, configs),
    // A remembered value the provider does not know — a spelling an earlier
    // build made up, or another provider's — is not offered back: it would be
    // refused at the first send and start nothing.
    effort: choiceAmong(config?.reasoningEffort, effortChoicesFor(provider, configs))
      || FALLBACK_EFFORTS[provider],
    access: choiceAmong(config?.approvalPolicy, accessChoicesFor(provider, configs))
      || FALLBACK_ACCESS[provider],
    projectPath: tidy(input.projectPath),
    cwd: tidy(input.cwd) || tidy(input.projectPath),
    branch: tidy(input.branch),
    createNewWorktree: false
  };
}

/** `value` when it is one of `choices`, else nothing. */
function choiceAmong(value: string | null | undefined, choices: readonly string[]): string {
  const wanted = tidy(value);
  return wanted && choices.includes(wanted) ? wanted : '';
}

export function effortChoicesFor(
  provider: ThreadStartProvider,
  configs: readonly ThreadStartProviderConfig[]
): string[] {
  const config = configForProvider(provider, configs);
  const available = unique(config?.availableEfforts ?? []);
  return available.length ? available : [...FALLBACK_EFFORT_CHOICES[provider]];
}

export function accessChoicesFor(
  provider: ThreadStartProvider,
  configs: readonly ThreadStartProviderConfig[]
): string[] {
  const config = configForProvider(provider, configs);
  const available = unique(config?.availableApprovalPolicies ?? []);
  return available.length ? available : [...FALLBACK_ACCESS_CHOICES[provider]];
}

/**
 * The first line becomes the rail title.
 *
 * Shaped by the one function that decides what a prompt's first line looks like
 * as a name, because the backend writes exactly the same string when the
 * message goes out. The rail saves this row back seconds later, and a title
 * that differs by a character reads as a rename there — which would stop the
 * session from ever being given a better name after its first turn.
 */
export function titleFromPrompt(prompt: string, projectPath: string): string {
  const fallback = tidy(projectPath).split('/').filter(Boolean).at(-1) || 'project';
  return sessionTitleFromPrompt(tidy(prompt)) ?? `Build in ${fallback}`;
}

/**
 * Validate only what can be settled before a backend call. The worktree toggle
 * is intentionally rejected because this build has no create-worktree command;
 * silently using the main checkout would violate the selected location.
 */
export function validateThreadStart(state: ThreadStartPickerState): ThreadStartProblem[] {
  const problems: ThreadStartProblem[] = [];
  if (!tidy(state.prompt)) {
    problems.push({ field: 'prompt', message: 'Describe what you want to build.' });
  }
  if (!tidy(state.projectPath).startsWith('/')) {
    problems.push({ field: 'project', message: 'Choose a project workspace first.' });
  }
  if (!tidy(state.cwd).startsWith('/')) {
    problems.push({ field: 'branch', message: 'Choose an existing checkout first.' });
  }
  if (!tidy(state.branch)) {
    problems.push({ field: 'branch', message: 'Choose an existing branch first.' });
  }
  if (state.createNewWorktree) {
    problems.push({
      field: 'worktree',
      message: 'New worktree creation is not available in this build. Choose an existing checkout.'
    });
  }
  return problems;
}

/** Assemble the route-owned spawn request, or null while the draft is invalid. */
export function buildThreadStartRequest(
  state: ThreadStartPickerState
): ThreadStartRequest | null {
  if (validateThreadStart(state).length > 0) return null;
  return {
    prompt: tidy(state.prompt),
    provider: state.provider,
    model: tidy(state.model) || null,
    reasoningEffort: tidy(state.effort) || null,
    // Antigravity's access is a statement, not a choice: its adapter offers no
    // approval control, and a session asked to change one refuses the message
    // that carried the request.
    approvalPolicy: state.provider === 'antigravity' ? null : (tidy(state.access) || null),
    projectPath: tidy(state.projectPath),
    cwd: tidy(state.cwd),
    branch: tidy(state.branch),
    createNewWorktree: state.createNewWorktree,
    title: titleFromPrompt(state.prompt, state.projectPath)
  };
}

export function displayEffort(value: string | null | undefined): string {
  const normalized = tidy(value).toLowerCase();
  if (normalized === 'xhigh') return 'Extra high';
  if (normalized === 'max') return 'Max';
  if (!normalized) return 'Default';
  return normalized[0].toUpperCase() + normalized.slice(1);
}

export function displayAccess(value: string | null | undefined): string {
  const normalized = tidy(value).toLowerCase();
  if (normalized === 'acceptedits') return 'Workspace write';
  if (normalized === 'on-request') return 'Ask when needed';
  if (normalized === 'never' || normalized === 'bypasspermissions') return 'Unrestricted';
  if (normalized === 'plan') return 'Plan only';
  if (!normalized) return 'Default';
  return normalized
    .split(/[-_]+/)
    .filter(Boolean)
    .map((part) => part[0].toUpperCase() + part.slice(1))
    .join(' ');
}

export function displayProvider(provider: ThreadStartProvider): string {
  return PROVIDER_LABELS[provider];
}
