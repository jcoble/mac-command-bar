/**
 * Pure rules for the thread-first session draft.
 *
 * The pane owns the interaction and the route owns the side effects. This
 * module is deliberately plain data only: it validates the draft, groups the
 * provider configuration already known to the shell, and assembles the one
 * request sent on the first message.
 */

export type ThreadStartProvider = 'codex' | 'claude';

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

const PROVIDER_ORDER: readonly ThreadStartProvider[] = ['codex', 'claude'];

const PROVIDER_LABELS: Record<ThreadStartProvider, string> = {
  codex: 'OpenAI',
  claude: 'Anthropic'
};

const FALLBACK_MODELS: Record<ThreadStartProvider, readonly string[]> = {
  codex: ['gpt-5.6-luna', 'gpt-5.6-sol', 'gpt-5.6-terra'],
  claude: ['claude-sonnet', 'claude-opus', 'claude-haiku']
};

const MODEL_HINTS: Record<string, string> = {
  'gpt-5.6-luna': 'balanced build work',
  'gpt-5.6-sol': 'deep review',
  'gpt-5.6-terra': 'fast iteration',
  'claude-opus': 'complex work',
  'claude-sonnet': 'everyday tasks',
  'claude-haiku': 'quick lookups'
};

const FALLBACK_EFFORTS: Record<ThreadStartProvider, string> = {
  codex: 'max',
  claude: 'medium'
};

const FALLBACK_ACCESS: Record<ThreadStartProvider, string> = {
  codex: 'on-request',
  claude: 'acceptedits'
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
    const models = configured.length
      ? unique([current ?? '', ...configured])
      : [...FALLBACK_MODELS[provider]];
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
    || FALLBACK_MODELS[provider][0];
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
    effort: tidy(config?.reasoningEffort)
      || unique(config?.availableEfforts ?? [])[0]
      || FALLBACK_EFFORTS[provider],
    access: tidy(config?.approvalPolicy)
      || unique(config?.availableApprovalPolicies ?? [])[0]
      || FALLBACK_ACCESS[provider],
    projectPath: tidy(input.projectPath),
    cwd: tidy(input.cwd) || tidy(input.projectPath),
    branch: tidy(input.branch),
    createNewWorktree: false
  };
}

export function effortChoicesFor(
  provider: ThreadStartProvider,
  configs: readonly ThreadStartProviderConfig[]
): string[] {
  const config = configForProvider(provider, configs);
  return unique(config?.availableEfforts ?? []).length
    ? unique(config?.availableEfforts ?? [])
    : [FALLBACK_EFFORTS[provider]];
}

export function accessChoicesFor(
  provider: ThreadStartProvider,
  configs: readonly ThreadStartProviderConfig[]
): string[] {
  const config = configForProvider(provider, configs);
  return unique(config?.availableApprovalPolicies ?? []).length
    ? unique(config?.availableApprovalPolicies ?? [])
    : [FALLBACK_ACCESS[provider]];
}

/** The first line becomes the rail title, trimmed to the row's readable width. */
export function titleFromPrompt(prompt: string, projectPath: string): string {
  const fallback = tidy(projectPath).split('/').filter(Boolean).at(-1) || 'project';
  const firstLine = tidy(prompt).split(/\r?\n/, 1)[0].replace(/\s+/g, ' ').trim();
  if (!firstLine) return `Build in ${fallback}`;
  return firstLine.length > 72 ? `${firstLine.slice(0, 71).trimEnd()}…` : firstLine;
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
    approvalPolicy: tidy(state.access) || null,
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
