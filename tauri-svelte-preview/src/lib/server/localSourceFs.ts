import { mkdir, open, readdir, readFile, rename, rm, stat, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import path from 'node:path';

import {
  findSourceDefinitionTargets,
  findSourceReferenceTargets,
  findSourceSearchMatches,
  previewFromContent,
  type SourceDefinitionTarget,
  type SourcePreview,
  type SourceRecord,
  type SourceReferenceCountResult,
  type SourceReferenceTarget,
  type SourceScanStats,
  type SourceScanResult,
  type SourceSearchMatch
} from '../sourceData.ts';

const defaultSourceListLimit = 10_000;
const maxSourceListLimit = 25_000;
const maxPreviewBytes = 512 * 1024;
const maxIntelligenceReadCount = 2_000;
const maxSkippedDirectorySamples = 16;
const claudeGenericSessionTitle = 'Claude session';
const claudeSessionFileLimit = 512;
const claudeSessionTailBytes = 256 * 1024;
/**
 * How far into a transcript to look for the entrypoint that launched it. The
 * field rides on every `user`, `assistant` and `attachment` record, so it turns
 * up early; across 1091 transcripts here 64 KB reaches it in 1050 of them, and
 * the rest are kept and settled when the file is parsed in full.
 */
const claudeSessionLaunchProbeBytes = 64 * 1024;
const codexGenericSessionTitle = 'Codex session';
const codexUntitledIndexTitle = 'Untitled Codex session';
const codexSessionFileLimit = 512;
/**
 * The first user message of a Codex session sits behind the opening metadata
 * record, which carries the whole system prompt and can run past 40 KB. On this
 * machine 256 KB reaches the first typed prompt in 59 of 67 top-level sessions;
 * the rest fall back to whatever the tail window offers.
 */
const codexSessionHeadBytes = 256 * 1024;
const codexSessionTailBytes = 256 * 1024;
/**
 * Enough to hold the opening `session_meta` line whole. Measured across 687
 * rollout files: median 27 KB, largest 44 KB.
 */
const codexSessionMetaProbeBytes = 64 * 1024;
const cmuxSessionResultHeadroom = 256;
const agentSessionResultLimit =
  claudeSessionFileLimit + codexSessionFileLimit + cmuxSessionResultHeadroom;

type LocalSourceScanInput = {
  root: string;
  query?: string | null;
  limit?: number | null;
};

type LocalSourceScanStats = SourceScanStats & {
  requestedLimit: number;
  returnedFiles: number;
  collectionLimit: number;
  collectionLimitReached: boolean;
};

export type LocalProjectRootValidationResult = {
  path: string;
  exists: boolean;
  isDirectory: boolean;
  isGitRepository: boolean;
  gitRoot: string | null;
  message: string;
};

export type LocalAgentSessionRecord = {
  provider: string;
  id: string;
  title: string;
  description: string | null;
  model: string | null;
  projectPath: string | null;
  lastActivity: string | null;
  resumeCommands: string[];
  /**
   * What the scan worked out about the session from its own title, folder and
   * resume command — the branch it is on, the task it belongs to, the pull
   * request it opened, and a short "who and where" label. Filled in once, at the
   * end of the scan, so everything that builds a record along the way leaves
   * them empty; a row draws a chip only for the ones that are there.
   */
  branchHint?: string | null;
  taskId?: string | null;
  pullRequestHint?: string | null;
  sourceLabel?: string | null;
};

export type LocalAgentSessionDerivedMetadata = {
  branchHint: string | null;
  taskId: string | null;
  pullRequestHint: string | null;
  linkHint: string | null;
  sourceLabel: string;
};

/**
 * The same hints the Rust scanner derives, from the same three places in the
 * same order: the session's title, then its folder, then its resume command.
 *
 * Kept deliberately in step with `derive_agent_session_metadata` in
 * `core/src/scanners/sessions.rs` — both scanners fill the same rail, so a row
 * has to read the same whichever one produced it. The test file pins this with
 * the fixture strings the Rust tests use.
 */
export function deriveAgentSessionMetadata(
  record: LocalAgentSessionRecord
): LocalAgentSessionDerivedMetadata {
  return {
    branchHint: firstAgentSessionHint(record, branchHintFromText),
    taskId: firstAgentSessionHint(record, taskIdFromText),
    pullRequestHint: firstAgentSessionHint(record, pullRequestHintFromText),
    linkHint: firstAgentSessionHint(record, linkHintFromText),
    sourceLabel: agentSessionSourceLabel(record)
  };
}

/**
 * Writes those hints onto every record, so the shell reads them off the row it
 * already has instead of parsing titles again on the other side of the bridge.
 *
 * Run once over the finished list rather than at each construction site: a
 * session is described by several files, and only the merged record has the
 * title, folder and resume command the hints are read from.
 */
function withDerivedAgentSessionMetadata(records: LocalAgentSessionRecord[]) {
  for (const record of records) {
    const metadata = deriveAgentSessionMetadata(record);
    record.branchHint = metadata.branchHint;
    record.taskId = metadata.taskId;
    record.pullRequestHint = metadata.pullRequestHint;
    record.sourceLabel = metadata.sourceLabel;
  }

  return records;
}

export async function validateLocalProjectRoot(root: string): Promise<LocalProjectRootValidationResult> {
  const normalizedRoot = normalizeRootPath(root);
  const rootStats = await stat(normalizedRoot).catch(() => null);

  if (!rootStats) {
    return {
      path: normalizedRoot,
      exists: false,
      isDirectory: false,
      isGitRepository: false,
      gitRoot: null,
      message: 'Project path not found'
    };
  }

  if (!rootStats.isDirectory()) {
    return {
      path: normalizedRoot,
      exists: true,
      isDirectory: false,
      isGitRepository: false,
      gitRoot: null,
      message: 'Project path points to a file. Choose the repository folder instead.'
    };
  }

  const gitRoot = await findGitRoot(normalizedRoot);
  const isGitRepository = gitRoot === normalizedRoot;

  return {
    path: normalizedRoot,
    exists: true,
    isDirectory: true,
    isGitRepository,
    gitRoot,
    message: isGitRepository
      ? 'Project root ready'
      : gitRoot
        ? `Folder is inside a Git repository. Add ${gitRoot} for full project context.`
      : 'Folder is not a Git repository. Source browsing will work, but Git/worktree panels may be unavailable.'
  };
}

export async function scanLocalAgentSessions(homeRoot = homedir()): Promise<LocalAgentSessionRecord[]> {
  const homePath = homeRoot.trim();
  if (!homePath) return [];

  const records: LocalAgentSessionRecord[] = [];

  // Sort the rollout files into "the user started this" and "Codex spawned this
  // for itself" in one pass, keeping the ids of the second kind. Dropping
  // sub-agent threads here, BEFORE the file budget, is for the same reason the
  // Claude path drops its subagents first: they outnumber the user's own
  // sessions nine to one, so a budget applied first would evict the sessions the
  // rail exists to show.
  const codexRollouts: string[] = [];
  const codexSubagentIds = new Set<string>();
  for (const filePath of await jsonlFiles(path.join(homePath, '.codex', 'sessions'))) {
    const marker = await codexRolloutFileThreadMarker(filePath);
    // No marker, or nothing readable: keep the file. Older Codex builds predate
    // the field, and losing a session is the worse mistake.
    if (!marker?.spawnedByCodex) codexRollouts.push(filePath);
    else if (marker.id) codexSubagentIds.add(marker.id);
  }

  // Codex eventually moves a thread's rollout file into `archived_sessions` but
  // leaves its index row behind, so for those threads the archive is the only
  // evidence left of what kind of thread it was.
  for (const filePath of await jsonlFiles(path.join(homePath, '.codex', 'archived_sessions'))) {
    const marker = await codexRolloutFileThreadMarker(filePath);
    if (marker?.spawnedByCodex && marker.id) codexSubagentIds.add(marker.id);
  }

  const codexIndex = path.join(homePath, '.codex', 'session_index.jsonl');
  const codexIndexContents = await readFile(codexIndex, 'utf8').catch(() => '');
  const codexRecords = dropCodexSubagentSessions(
    codexIndexContents ? parseCodexIndexJsonl(codexIndexContents) : [],
    codexSubagentIds
  );

  const codexFiles = await sortFilesByModifiedDesc(codexRollouts);
  const codexMetadata: LocalAgentSessionRecord[] = [];
  for (const filePath of codexFiles.slice(0, codexSessionFileLimit)) {
    const contents = await readHeadAndTailUtf8(
      filePath,
      codexSessionHeadBytes,
      codexSessionTailBytes
    ).catch(() => '');
    if (contents) codexMetadata.push(...parseCodexRolloutJsonl(contents));
  }
  records.push(...mergeCodexSessionMetadata(codexRecords, codexMetadata));

  const cmuxRoot = path.join(homePath, '.cmuxterm');
  for (const { agent, filePath } of await cmuxHookSessionFiles(cmuxRoot)) {
    const contents = await readFile(filePath, 'utf8').catch(() => '');
    if (contents) records.push(...parseCmuxHookSessionsJson(agent, contents));
  }

  // Drop subagent transcripts and helper runs BEFORE the file budget is
  // applied: they outnumber real sessions on a busy machine and would otherwise
  // evict them.
  const claudeTranscripts: string[] = [];
  for (const filePath of await jsonlFiles(path.join(homePath, '.claude', 'projects'))) {
    if (isClaudeSubagentTranscriptPath(filePath)) continue;
    if (await claudeTranscriptFileIsAgentLaunched(filePath)) continue;
    claudeTranscripts.push(filePath);
  }
  const claudeFiles = await sortFilesByModifiedDesc(claudeTranscripts);
  for (const filePath of claudeFiles.slice(0, claudeSessionFileLimit)) {
    const projectPath = decodeClaudeProjectDir(path.basename(path.dirname(filePath))) ?? '';
    const contents = await readTailUtf8(filePath, claudeSessionTailBytes).catch(() => '');
    if (contents) records.push(...parseClaudeJsonl(contents, projectPath));
  }

  return withDerivedAgentSessionMetadata(
    mergeAgentSessionRecords(records)
      .sort((left, right) => compareNullableStringsDescending(left.lastActivity, right.lastActivity))
      .slice(0, agentSessionResultLimit)
  );
}

async function findGitRoot(root: string): Promise<string | null> {
  let current = normalizeRootPath(root);

  while (true) {
    if (await stat(path.join(current, '.git')).catch(() => null)) {
      return current;
    }

    const parent = path.dirname(current);
    if (parent === current) return null;
    current = parent;
  }
}

export function parseCodexIndexJsonl(input: string): LocalAgentSessionRecord[] {
  return parseJsonLines(input).flatMap((value) => {
    const id = optionalString(value.id);
    if (!id) return [];

    return [
      {
        provider: 'codex',
        id,
        title: optionalString(value.thread_name) ?? codexUntitledIndexTitle,
        description: null,
        model: modelFromValue(value),
        projectPath: null,
        lastActivity: optionalString(value.updated_at),
        resumeCommands: [`codex resume ${id}`]
      }
    ];
  });
}

export function parseCodexRolloutJsonl(input: string): LocalAgentSessionRecord[] {
  const records: LocalAgentSessionRecord[] = [];
  const firstPrompts = new Map<string, string>();

  for (const value of parseJsonLines(input)) {
    const type = optionalString(value.type);
    if (type === 'session_meta') {
      const payload = objectValue(value.payload);
      if (payload && isCodexSubagentMeta(payload)) return [];
      const id = optionalString(payload?.id);
      if (!payload || !id) continue;

      const record: LocalAgentSessionRecord = {
        provider: 'codex',
        id,
        title: codexGenericSessionTitle,
        description: null,
        model: modelFromValue(payload),
        projectPath: optionalString(payload.cwd),
        lastActivity: optionalString(value.timestamp) ?? optionalString(payload.timestamp),
        resumeCommands: [`codex resume ${id}`]
      };
      upsertCodexSessionRecord(records, record);
      continue;
    }

    if (type === 'turn_context') {
      const payload = objectValue(value.payload);
      if (!payload) continue;
      updateLatestCodexRecord(
        records,
        optionalString(payload.cwd),
        null,
        modelFromValue(payload),
        optionalString(value.timestamp)
      );
      continue;
    }

    if (type === 'response_item') {
      const latest = records[records.length - 1];
      const prompt = codexUserPromptText(value);
      if (latest && prompt && !firstPrompts.has(latest.id)) firstPrompts.set(latest.id, prompt);

      const cwd = codexResponseItemWorkdir(value);
      const description = codexResponseItemDescription(value);
      if (!cwd && !description) continue;
      updateLatestCodexRecord(records, cwd, description, null, optionalString(value.timestamp));
    }
  }

  for (const record of records) {
    record.title = codexDisplayTitle(record.title, firstPrompts.get(record.id) ?? null);
  }

  return records;
}

/**
 * Codex spawns its own helper threads and writes each one as a rollout file next
 * to the user's, so nine of every ten files under `~/.codex/sessions` are threads
 * nobody opened. The opening `session_meta` record says which it is: a helper
 * carries `thread_source: "subagent"`, and/or a `source` object whose only key is
 * `subagent` (a session the user started has `source` as a plain string — `cli`,
 * `vscode`, `exec`).
 *
 * Verified 2026-07-28 across 687 rollout files on this machine: 619 helper
 * threads, 68 sessions the user started, and the two markers disagreed on a
 * single file — so both are checked and either one is enough.
 *
 * A file with neither marker is kept. Older Codex versions predate the field (15
 * files here), and losing one of the user's sessions is worse than listing a
 * helper thread.
 */
function isCodexSubagentMeta(payload: Record<string, unknown>) {
  if (optionalString(payload.thread_source) === 'subagent') return true;

  const source = objectValue(payload.source);
  return source ? 'subagent' in source : false;
}

/**
 * What a rollout file's opening metadata record says about its thread. The id
 * can be missing while the verdict is still known, so the two are separate.
 */
export type CodexThreadMarker = { id: string | null; spawnedByCodex: boolean };

/**
 * Read from a bounded head, so the scan can sort the files before it spends its
 * read budget on them.
 */
async function codexRolloutFileThreadMarker(filePath: string): Promise<CodexThreadMarker | null> {
  const head = await readHeadUtf8(filePath, codexSessionMetaProbeBytes).catch(() => '');
  return codexRolloutHeadThreadMarker(head);
}

/**
 * Null when the head holds no readable metadata record — a truncated line, a
 * file that opens with something else, an empty file. The caller keeps those.
 */
export function codexRolloutHeadThreadMarker(head: string): CodexThreadMarker | null {
  const line = head.split(/\r?\n/, 1)[0];
  const value = parseJsonObject(line ?? '');
  if (!value || optionalString(value.type) !== 'session_meta') return null;

  const payload = objectValue(value.payload);
  if (!payload) return null;

  return { id: optionalString(payload.id), spawnedByCodex: isCodexSubagentMeta(payload) };
}

export function codexRolloutHeadIsSubagentThread(head: string) {
  return codexRolloutHeadThreadMarker(head)?.spawnedByCodex ?? false;
}

/**
 * Index rows naming threads Codex spawned for itself.
 *
 * `~/.codex/session_index.jsonl` is a flat list of id, name and timestamp with
 * nothing in it saying what kind of thread a row describes, so the rollout files
 * are the only evidence — and the index is read whole, with none of the
 * filtering the rollout files get. On this machine 119 of its 330 rows are
 * sub-agent threads, 117 of them already archived, and every one of those was
 * reaching the rail under a plausible name ("Audit inventory gaps", "Review
 * mobile steppers") that gave the user no way to tell it apart from their own
 * work.
 *
 * A row whose thread has no rollout file left anywhere is kept: 150 of them
 * here, all genuinely the user's, and absence of evidence is not evidence.
 */
export function dropCodexSubagentSessions(
  records: LocalAgentSessionRecord[],
  subagentIds: Set<string>
): LocalAgentSessionRecord[] {
  return records.filter((record) => !subagentIds.has(record.id));
}

/**
 * The first thing the user actually typed. Codex opens every session with
 * machine-written turns sent as the user — the repository's `AGENTS.md`, the
 * environment block, the plugin list, the file list — and each has a shape we can
 * recognise, so the title comes from the first turn that is none of them.
 */
function codexUserPromptText(value: Record<string, unknown>) {
  const payload = objectValue(value.payload);
  if (!payload || optionalString(payload.type) !== 'message') return null;
  if (optionalString(payload.role) !== 'user') return null;

  const text = valueToText(payload.content)?.trim();
  if (!text) return null;
  if (codexInjectedPromptPrefixes.some((prefix) => text.startsWith(prefix))) return null;

  return compactText(text, 140);
}

/**
 * Openings that mean Codex wrote this turn, not the user. `<` covers every tagged
 * block it injects (`<environment_context>`, `<recommended_plugins>`,
 * `<user_shell_command>`); the two headings are the repository instructions and
 * the attached-file list.
 */
const codexInjectedPromptPrefixes = [
  '<',
  '# AGENTS.md instructions',
  '# Files mentioned by the user'
];

/**
 * A Codex rollout file carries no title of its own, so a rail built from these
 * files alone reads as hundreds of rows all saying "Codex session". The first
 * typed prompt is the only description of the session in the file, so it becomes
 * the title; the generic label survives only when the scanned window held no
 * typed prompt at all.
 */
function codexDisplayTitle(derived: string, firstPrompt: string | null) {
  if (derived !== codexGenericSessionTitle) return derived;
  return firstPrompt ? compactText(firstPrompt, 60) : codexGenericSessionTitle;
}

function isGenericCodexTitle(title: string) {
  return title === codexGenericSessionTitle || title === codexUntitledIndexTitle;
}

/**
 * One Codex session can be described twice: the index file may name it, and the
 * rollout file offers the opening prompt. Whichever record looks newer is beside
 * the point — a placeholder must never displace a real title, and the index name
 * describes the whole session where the prompt only opens it, so the title
 * already in hand wins when both are real.
 */
function betterCodexTitle(existing: string, candidate: string) {
  if (!isGenericCodexTitle(existing)) return existing;
  if (!isGenericCodexTitle(candidate)) return candidate;
  return existing;
}

function upsertCodexSessionRecord(
  records: LocalAgentSessionRecord[],
  record: LocalAgentSessionRecord
) {
  const existing = records.find(
    (candidate) => candidate.provider === record.provider && candidate.id === record.id
  );
  if (!existing) {
    records.push(record);
    return;
  }

  const previousTitle = existing.title;
  const candidateTitle = record.title;
  mergeAgentSessionRecord(existing, record);
  existing.title = betterCodexTitle(previousTitle, candidateTitle);
}

function updateLatestCodexRecord(
  records: LocalAgentSessionRecord[],
  cwd: string | null,
  description: string | null,
  model: string | null,
  lastActivity: string | null
) {
  const record = records[records.length - 1];
  if (!record) return;

  mergeAgentSessionRecord(record, {
    provider: 'codex',
    id: record.id,
    title: record.title,
    description,
    model,
    projectPath: cwd,
    lastActivity,
    resumeCommands: [`codex resume ${record.id}`]
  });
}

function codexResponseItemWorkdir(value: Record<string, unknown>) {
  const payload = objectValue(value.payload);
  if (!payload || optionalString(payload.type) !== 'function_call') return null;

  const rawArguments = optionalString(payload.arguments);
  if (!rawArguments) return null;

  const parsed = parseJsonObject(rawArguments);
  return parsed ? optionalString(parsed.workdir) : null;
}

function codexResponseItemDescription(value: Record<string, unknown>) {
  const payload = objectValue(value.payload);
  if (!payload || optionalString(payload.type) !== 'message') return null;
  if (optionalString(payload.role) !== 'user') return null;

  const text = valueToText(payload.content);
  return text ? compactText(text, 140) : null;
}

export function mergeCodexSessionMetadata(
  indexed: LocalAgentSessionRecord[],
  metadata: LocalAgentSessionRecord[]
) {
  for (const record of metadata) {
    upsertCodexSessionRecord(indexed, record);
  }
  return indexed;
}

function parseCmuxHookSessionsJson(agent: string, input: string): LocalAgentSessionRecord[] {
  const value = parseJsonObject(input);
  const sessions = objectValue(value?.sessions);
  const normalizedAgent = agent.trim().toLowerCase();
  if (!sessions || !normalizedAgent) return [];

  return Object.entries(sessions).flatMap(([key, rawSession]) => {
    const session = objectValue(rawSession);
    if (!session) return [];

    const id = optionalString(session.sessionId) ?? key.trim();
    if (!id) return [];

    const launchCommand = objectValue(session.launchCommand);
    const cwd = optionalString(session.cwd) ?? optionalString(launchCommand?.workingDirectory);
    const lastActivity =
      timestampishString(session.updatedAt)
      ?? timestampishString(session.startedAt)
      ?? timestampishString(launchCommand?.capturedAt);
    const status = optionalString(session.runtimeStatus) ?? optionalString(session.agentLifecycle);

    return [
      {
        provider: `cmux-${normalizedAgent}`,
        id,
        title: cmuxSessionTitle(normalizedAgent, session, status, cwd),
        description: cmuxSessionDescription(session),
        model: modelFromValue(session),
        projectPath: cwd,
        lastActivity,
        resumeCommands: cmuxResumeCommands(normalizedAgent, id, cwd)
      }
    ];
  });
}

export function parseClaudeJsonl(input: string, projectPath: string): LocalAgentSessionRecord[] {
  const records: LocalAgentSessionRecord[] = [];
  const aiTitles = new Map<string, string>();
  const firstPrompts = new Map<string, string>();

  for (const value of parseJsonLines(input)) {
    // One such entry condemns the whole transcript: these files are what they
    // are end to end, so anything already collected from one is an agent's
    // turn, not a session the user can resume. Both checks answer the same
    // question — "is this a top-level session?" — from different evidence: a
    // subagent's sidechain flag, or a helper's entrypoint.
    if (isClaudeSidechainEntry(value) || isClaudeAgentLaunchedEntry(value)) return [];

    const aiTitle = claudeAiTitle(value);
    if (aiTitle) aiTitles.set(aiTitle[0], aiTitle[1]); // a later line is the newer title
    const prompt = claudeUserPromptText(value);
    if (prompt && !firstPrompts.has(prompt[0])) firstPrompts.set(prompt[0], prompt[1]);

    const id = optionalString(value.sessionId) ?? optionalString(value.session_id);
    if (!id) continue;

    const cwd = optionalString(value.cwd) ?? projectPath;
    upsertAgentSessionRecord(records, {
      provider: 'claude',
      id,
      title: titleFromClaudeMessage(value) ?? claudeGenericSessionTitle,
      description: claudeSessionDescription(value),
      model: modelFromValue(objectValue(value.message)) ?? modelFromValue(value),
      projectPath: cwd || null,
      lastActivity: optionalString(value.timestamp) ?? optionalString(value.created_at),
      resumeCommands: [
        `claude --resume ${id}`,
        cwd ? `cd ${shellQuote(cwd)} && claude --resume ${id}` : `claude --resume ${id}`
      ]
    });
  }

  for (const record of records) {
    record.title = claudeDisplayTitle(
      record.title,
      aiTitles.get(record.id) ?? null,
      firstPrompts.get(record.id) ?? null,
      record.projectPath
    );
  }

  return records;
}

/**
 * A Claude Code SUBAGENT transcript is stored as its own `.jsonl` and is never
 * resumable — `claude --resume <id>` on one is meaningless — so it must never
 * reach the rail. Two independent discriminators, because each covers what the
 * other cannot: the path is free and also keeps subagents out of the scan's file
 * budget, the content is authoritative and still catches a flat layout.
 *
 * Today Claude Code writes them under `<project>/<session>/subagents/`.
 */
function isClaudeSubagentTranscriptPath(filePath: string) {
  return filePath.split(path.sep).includes('subagents');
}

/**
 * Every entry of a subagent transcript carries `"isSidechain": true`; every entry
 * of a real session carries `false`. Verified across 2180 transcripts on a live
 * machine: 1177 sidechain files, all of them pure, zero mixed files.
 */
function isClaudeSidechainEntry(value: Record<string, unknown>) {
  return value.isSidechain === true;
}

/**
 * Entrypoint values that mean "a program started this run, not the user".
 * Prefixes, so a future `sdk-node` is covered without a code change; adding a
 * new family is a one-line edit here.
 */
const agentLaunchEntrypointPrefixes = ['sdk'];

/**
 * Helper-agent transcripts — a team lead's dispatched teammates, and any other
 * SDK-driven run — land flat in the same project directory as the user's own
 * sessions, with `isSidechain: false`, `userType: "external"` and no agent name
 * anywhere, so neither discriminator above sees them. What they do carry is how
 * they were launched: every `user`, `assistant` and `attachment` record repeats
 * an `entrypoint`, and a programmatic run is always `sdk-…` (`sdk-cli`,
 * `sdk-py`) where a session the user typed into is `cli` or `claude-vscode`.
 *
 * Verified 2026-07-28 across 1073 transcripts on this machine: every one of the
 * 888 dispatched helper runs was `sdk-…`, every human session was `cli` or
 * `claude-vscode`, and no transcript ever mixed the two families. 1072 of the
 * 1073 carry the field within the 256 KB tail this scanner reads.
 *
 * This deliberately replaces the "does the first message read like a dispatch
 * prompt?" idea: the user's real sessions often open with pasted logs and
 * instruction-shaped text, and the scanner reads a tail window that usually
 * does not even contain the first message.
 */
export function isAgentLaunchEntrypoint(entrypoint: string) {
  return agentLaunchEntrypointPrefixes.some((prefix) => entrypoint.startsWith(prefix));
}

/**
 * Reads that entrypoint off one transcript line. A line without the field says
 * nothing either way — older transcripts predate it — so it is not evidence of
 * a helper and the transcript is kept.
 */
function isClaudeAgentLaunchedEntry(value: Record<string, unknown>) {
  const entrypoint = optionalString(value.entrypoint);
  return entrypoint ? isAgentLaunchEntrypoint(entrypoint) : false;
}

/**
 * The same question asked of a file instead of a parsed record, so the scan can
 * skip helper runs before it spends its read budget on them.
 *
 * Excluding them at parse time was never enough. A batch of API work can write
 * hundreds of transcripts in an afternoon — 880 of the 1091 here came from one
 * document-extraction job, all in a temporary directory — and the scan reads
 * only the newest 512 files. Sorted by modification time, that batch pushed the
 * user's own sessions out of the scan entirely: 6 of 24 survived. Nothing was
 * wrong with the rule; it just ran too late to matter.
 *
 * Reads a bounded head rather than the whole file. Anything unreadable, or whose
 * entrypoint sits past the probe, is kept and settled when the file is parsed in
 * full — the same direction to fail as every other check here.
 */
async function claudeTranscriptFileIsAgentLaunched(filePath: string) {
  const head = await readHeadUtf8(filePath, claudeSessionLaunchProbeBytes).catch(() => '');
  return claudeTranscriptHeadIsAgentLaunched(head);
}

export function claudeTranscriptHeadIsAgentLaunched(head: string) {
  // Stops at the first record that answers, rather than parsing the whole
  // window: this runs once per transcript on every scan.
  for (const line of head.split(/\r?\n/)) {
    const value = parseJsonObject(line);
    if (value && isClaudeAgentLaunchedEntry(value)) return true;
  }

  return false;
}

/**
 * Claude Code records its own generated session title on a `type: "ai-title"`
 * line. It is the best title available — it describes the whole session rather
 * than whichever message happened to be last — so it wins outright.
 */
function claudeAiTitle(value: Record<string, unknown>): [string, string] | null {
  if (optionalString(value.type) !== 'ai-title') return null;

  const id = optionalString(value.sessionId);
  const title = optionalString(value.aiTitle);
  return id && title ? [id, title] : null;
}

/**
 * The first genuine user prompt in the scanned window. Tool results are
 * `type: "user"` too, so plain `content` text is required and `tool_result` items
 * are dropped; slash-command wrappers (`<command-name>…`) and the resume caveat
 * are skipped because neither says what the session is about.
 */
function claudeUserPromptText(value: Record<string, unknown>): [string, string] | null {
  if (optionalString(value.type) !== 'user' || value.isMeta === true) return null;

  const id = optionalString(value.sessionId);
  const content = objectValue(value.message)?.content;
  if (!id) return null;

  const text =
    typeof content === 'string'
      ? content
      : Array.isArray(content)
        ? content
            .flatMap((item) => {
              const object = objectValue(item);
              return optionalString(object?.type) === 'text'
                ? optionalString(object?.text) ?? []
                : [];
            })
            .join(' ')
        : null;

  const trimmed = text?.trim();
  if (!trimmed || trimmed.startsWith('<') || trimmed.startsWith('Caveat:')) return null;
  return [id, trimmed];
}

/**
 * Title preference for a Claude row: the session's own AI title, then whatever a
 * message yielded, then `<project folder> — <first user prompt>`. The generic
 * label survives only when nothing else exists — a rail full of "Claude session"
 * rows tells the user nothing about which session to resume.
 */
function claudeDisplayTitle(
  derived: string,
  aiTitle: string | null,
  firstPrompt: string | null,
  projectPath: string | null
) {
  if (aiTitle) return compactText(aiTitle, 80);
  if (derived !== claudeGenericSessionTitle) return derived;
  if (!firstPrompt) return claudeGenericSessionTitle;

  const folder = pathDisplayName(projectPath);
  return compactText(folder ? `${folder} — ${firstPrompt}` : firstPrompt, 60);
}

function mergeAgentSessionRecords(records: LocalAgentSessionRecord[]) {
  const merged: LocalAgentSessionRecord[] = [];
  for (const record of records) {
    upsertAgentSessionRecord(merged, record);
  }
  return merged;
}

function upsertAgentSessionRecord(records: LocalAgentSessionRecord[], record: LocalAgentSessionRecord) {
  const existing = records.find(
    (candidate) => candidate.provider === record.provider && candidate.id === record.id
  );
  if (existing) {
    mergeAgentSessionRecord(existing, record);
  } else {
    records.push(record);
  }
}

function mergeAgentSessionRecord(existing: LocalAgentSessionRecord, candidate: LocalAgentSessionRecord) {
  if (!existing.description) existing.description = candidate.description;
  if (!existing.model) existing.model = candidate.model;
  if (!existing.projectPath) existing.projectPath = candidate.projectPath;

  const candidateIsNewer =
    candidate.lastActivity !== null
    && (existing.lastActivity === null || candidate.lastActivity > existing.lastActivity);
  if (candidateIsNewer) {
    existing.title = candidate.title;
    existing.description = candidate.description ?? existing.description;
    existing.model = candidate.model ?? existing.model;
    existing.projectPath = candidate.projectPath ?? existing.projectPath;
    existing.lastActivity = candidate.lastActivity;
  }

  for (const command of candidate.resumeCommands) {
    if (!existing.resumeCommands.includes(command)) existing.resumeCommands.push(command);
  }
}

async function jsonlFiles(root: string): Promise<string[]> {
  const entries = await readdir(root, { withFileTypes: true }).catch(() => []);
  const files: string[] = [];

  for (const entry of entries) {
    const entryPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...await jsonlFiles(entryPath));
    } else if (entry.isFile() && path.extname(entry.name) === '.jsonl') {
      files.push(entryPath);
    }
  }

  return files;
}

async function cmuxHookSessionFiles(root: string) {
  const entries = await readdir(root, { withFileTypes: true }).catch(() => []);
  return entries
    .filter((entry) => entry.isFile() && entry.name.endsWith('-hook-sessions.json'))
    .map((entry) => ({
      agent: entry.name.slice(0, -'-hook-sessions.json'.length),
      filePath: path.join(root, entry.name)
    }))
    .filter((entry) => entry.agent.length > 0);
}

async function sortFilesByModifiedDesc(files: string[]) {
  const entries = await Promise.all(
    files.map(async (filePath) => ({
      filePath,
      modifiedAt: (await stat(filePath).catch(() => null))?.mtimeMs ?? 0
    }))
  );
  return entries.sort((left, right) => right.modifiedAt - left.modifiedAt).map((entry) => entry.filePath);
}

export async function scanLocalSourceFiles(input: LocalSourceScanInput): Promise<SourceScanResult> {
  const root = normalizeRootPath(input.root);
  const rootStats = await stat(root).catch((error: unknown) => {
    throw new Error(`Could not read source root metadata: ${errorMessage(error)}`);
  });

  if (!rootStats.isDirectory()) {
    throw new Error('Source root is not a directory');
  }

  const limit = clampSourceLimit(input.limit);
  const collectLimit = sourceCollectionLimit(limit);
  const query = input.query?.trim().toLowerCase() || null;
  const records: SourceRecord[] = [];
  const stats = createSourceScanStats(limit, collectLimit);
  await collectSourceFiles(root, root, collectLimit, query, records, stats);

  records.sort(compareSourceRecords);
  const collectedFileCount = records.length;
  const truncated = collectedFileCount > limit;
  records.splice(limit);
  stats.returnedFiles = records.length;
  stats.collectionLimitReached = collectedFileCount >= collectLimit;

  return {
    records,
    limit,
    truncated,
    stats
  };
}

export async function readLocalSourceFile(filePath: string): Promise<SourcePreview> {
  const resolvedPath = normalizeFilePath(filePath);
  const fileStats = await stat(resolvedPath).catch((error: unknown) => {
    throw new Error(`Could not read source metadata: ${errorMessage(error)}`);
  });

  if (!fileStats.isFile()) {
    throw new Error('Source path is not a file');
  }

  if (!isSourceFile(resolvedPath)) {
    throw new Error('Source path is not a supported source file');
  }

  if (fileStats.size > maxPreviewBytes) {
    throw new Error('Source file is too large to preview');
  }

  const content = await readFile(resolvedPath, 'utf8').catch((error: unknown) => {
    throw new Error(`Could not read source file as UTF-8: ${errorMessage(error)}`);
  });

  return previewFromContent(sourceRecordForPath(resolvedPath, fileStats.size), content);
}

export async function writeLocalSourceFile(filePath: string, content: string): Promise<SourcePreview> {
  const resolvedPath = normalizeFilePath(filePath);
  const fileStats = await stat(resolvedPath).catch((error: unknown) => {
    throw new Error(`Could not read source metadata: ${errorMessage(error)}`);
  });

  if (!fileStats.isFile()) {
    throw new Error('Source path is not a file');
  }

  if (!isSourceFile(resolvedPath)) {
    throw new Error('Source path is not a supported source file');
  }

  const tempPath = `${resolvedPath}.mcb-${process.pid}-${Date.now()}.tmp`;
  await mkdir(path.dirname(resolvedPath), { recursive: true });

  try {
    await writeFile(tempPath, content, 'utf8');
    await rename(tempPath, resolvedPath);
  } catch (error) {
    await rm(tempPath, { force: true }).catch(() => undefined);
    throw new Error(`Could not write source file: ${errorMessage(error)}`);
  }

  return readLocalSourceFile(resolvedPath);
}

export async function searchLocalSourceFiles(
  records: SourceRecord[],
  query: string,
  limit = 50
): Promise<SourceSearchMatch[]> {
  const previews = await readLocalSourcePreviews(records);
  return findSourceSearchMatches(previews, query, limit);
}

export async function findLocalSourceDefinitions(
  records: SourceRecord[],
  symbolName: string,
  limit = 20
): Promise<SourceDefinitionTarget[]> {
  const previews = await readLocalSourcePreviews(records);
  return findSourceDefinitionTargets(previews, symbolName, limit);
}

export async function findLocalSourceReferences(
  records: SourceRecord[],
  symbolName: string,
  limit = 50
): Promise<SourceReferenceTarget[]> {
  const previews = await readLocalSourcePreviews(records);
  return findSourceReferenceTargets(previews, symbolName, limit);
}

/**
 * The browser preview's answer to the desktop app's batched margin counts:
 * count how many lines mention each symbol, reading the project once for all of
 * them, and give up when the time budget runs out.
 */
export async function countLocalSourceReferences(
  root: string,
  symbolNames: string[],
  deadlineMs: number | null
): Promise<SourceReferenceCountResult> {
  const startedAt = Date.now();
  const budgetMs = Math.min(Math.max(deadlineMs ?? 400, 1), 10_000);
  const names = normalizedReferenceCountSymbols(symbolNames);
  const counts: Record<string, number> = {};
  for (const name of names) counts[name] = 0;

  if (names.length === 0) {
    return { counts, approximate: false, scannedFiles: 0, elapsedMs: Date.now() - startedAt };
  }

  const scan = await scanLocalSourceFiles({ root, query: null, limit: null });
  const patterns = names.map(
    (name) => [name, new RegExp(`\\b${escapeRegExpValue(name)}\\b`)] as const
  );
  let approximate = scan.stats?.collectionLimitReached === true;
  let scannedFiles = 0;

  for (const record of scan.records) {
    if (Date.now() - startedAt >= budgetMs) {
      approximate = true;
      break;
    }

    if (record.byteCount > maxPreviewBytes) continue;
    const content = await readFile(record.path, 'utf8').catch(() => null);
    if (content === null) continue;
    scannedFiles += 1;

    for (const line of content.split(/\r\n|\r|\n/)) {
      for (const [name, pattern] of patterns) {
        if (pattern.test(line)) counts[name] += 1;
      }
    }
  }

  return { counts, approximate, scannedFiles, elapsedMs: Date.now() - startedAt };
}

function escapeRegExpValue(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function normalizedReferenceCountSymbols(symbolNames: string[]): string[] {
  const names: string[] = [];
  const seen = new Set<string>();

  for (const symbolName of symbolNames) {
    const trimmed = symbolName.trim();
    if (!trimmed || seen.has(trimmed)) continue;
    seen.add(trimmed);
    names.push(trimmed);
    if (names.length >= 256) break;
  }

  return names;
}

async function readLocalSourcePreviews(records: SourceRecord[]): Promise<SourcePreview[]> {
  const previews: SourcePreview[] = [];

  for (const record of records.slice(0, maxIntelligenceReadCount)) {
    try {
      previews.push(await readLocalSourcePreviewForRecord(record));
    } catch {
      // Missing, binary, oversized, or unreadable files should not break broad search.
    }
  }

  return previews;
}

async function readLocalSourcePreviewForRecord(record: SourceRecord): Promise<SourcePreview> {
  const fileStats = await stat(record.path);
  if (!fileStats.isFile() || !isSourceFile(record.path) || fileStats.size > maxPreviewBytes) {
    throw new Error('Source file is not readable for preview');
  }

  const content = await readFile(record.path, 'utf8');
  return previewFromContent(
    {
      ...record,
      byteCount: fileStats.size
    },
    content
  );
}

async function collectSourceFiles(
  root: string,
  current: string,
  limit: number,
  query: string | null,
  records: SourceRecord[],
  scanStats: SourceScanStats
) {
  if (records.length >= limit) return;

  const entries = await readdir(current, { withFileTypes: true }).catch((error: unknown) => {
    throw new Error(`Could not read source directory: ${errorMessage(error)}`);
  });

  entries.sort((left, right) => compareSourceWalkEntries(root, current, left, right));

  for (const entry of entries) {
    if (records.length >= limit) return;

    scanStats.visitedEntries += 1;
    const entryPath = path.join(current, entry.name);
    if (entry.isDirectory()) {
      const skipReason = skipDirReason(entry.name);
      if (skipReason) {
        recordSkippedDirectory(root, entryPath, entry.name, skipReason, scanStats);
      } else {
        await collectSourceFiles(root, entryPath, limit, query, records, scanStats);
      }
      continue;
    }

    if (!entry.isFile()) {
      scanStats.unsupportedFiles += 1;
      continue;
    }

    if (!isSourceFile(entryPath)) {
      scanStats.unsupportedFiles += 1;
      continue;
    }

    const relativePath = normalizeRelativePath(path.relative(root, entryPath));
    if (!sourceFileMatchesQuery(relativePath, entry.name, query)) {
      continue;
    }

    const fileStats = await stat(entryPath).catch(() => null);
    if (!fileStats?.isFile()) {
      scanStats.unreadableEntries += 1;
      continue;
    }

    records.push({
      path: entryPath,
      relativePath,
      fileName: entry.name,
      language: detectLanguage(entryPath),
      byteCount: fileStats.size
    });
    scanStats.matchedFiles += 1;
  }
}

function createSourceScanStats(requestedLimit: number, collectionLimit: number): LocalSourceScanStats {
  return {
    requestedLimit,
    returnedFiles: 0,
    collectionLimit,
    collectionLimitReached: false,
    visitedEntries: 0,
    matchedFiles: 0,
    skippedDirectories: 0,
    unsupportedFiles: 0,
    unreadableEntries: 0
  };
}

function recordSkippedDirectory(
  root: string,
  directoryPath: string,
  name: string,
  reason: string,
  scanStats: SourceScanStats
) {
  scanStats.skippedDirectories += 1;
  if ((scanStats.skippedDirectorySamples?.length ?? 0) >= maxSkippedDirectorySamples) return;

  const samples = scanStats.skippedDirectorySamples ?? [];
  samples.push({
    path: normalizeRelativePath(path.relative(root, directoryPath)),
    name,
    reason
  });
  scanStats.skippedDirectorySamples = samples;
}

function normalizeRootPath(root: string) {
  const normalizedRoot = root.trim();
  if (!normalizedRoot) {
    throw new Error('Source root is required');
  }

  return path.resolve(normalizedRoot);
}

function normalizeFilePath(filePath: string) {
  const normalizedPath = filePath.trim();
  if (!normalizedPath) {
    throw new Error('Source path is required');
  }

  return path.resolve(normalizedPath);
}

function sourceRecordForPath(filePath: string, byteCount: number): SourceRecord {
  return {
    path: filePath,
    relativePath: path.basename(filePath),
    fileName: path.basename(filePath),
    language: detectLanguage(filePath),
    byteCount
  };
}

function normalizeRelativePath(relativePath: string) {
  return relativePath.split(path.sep).join('/');
}

function clampSourceLimit(limit: number | null | undefined) {
  if (!Number.isFinite(limit) || !limit) return defaultSourceListLimit;
  return Math.min(maxSourceListLimit, Math.max(0, Math.trunc(limit)));
}

function sourceCollectionLimit(limit: number) {
  return Math.min(maxSourceListLimit, Math.max(defaultSourceListLimit, limit)) + 1;
}

function isSourceFile(filePath: string) {
  return detectLanguage(filePath) !== 'plain';
}

function detectLanguage(filePath: string) {
  const extension = path.extname(filePath).slice(1).toLowerCase();
  const fileName = path.basename(filePath).toLowerCase();

  if (fileName === 'dockerfile') return 'dockerfile';
  if (fileName === 'makefile') return 'makefile';

  switch (extension) {
    case 'cs':
      return 'csharp';
    case 'ts':
      return 'typescript';
    case 'tsx':
      return 'tsx';
    case 'js':
    case 'mjs':
    case 'cjs':
      return 'javascript';
    case 'jsx':
      return 'jsx';
    case 'svelte':
      return 'svelte';
    case 'json':
    case 'jsonc':
      return 'json';
    case 'md':
      return 'markdown';
    case 'mdx':
      return 'mdx';
    case 'yml':
    case 'yaml':
      return 'yaml';
    case 'toml':
      return 'toml';
    case 'xml':
    case 'xsd':
    case 'csproj':
    case 'fsproj':
    case 'vbproj':
    case 'props':
    case 'targets':
    case 'xaml':
      return 'xml';
    case 'astro':
    case 'html':
    case 'htm':
    case 'vue':
      return 'html';
    case 'css':
      return 'css';
    case 'scss':
      return 'scss';
    case 'less':
      return 'less';
    case 'ini':
    case 'env':
      return 'ini';
    case 'rs':
      return 'rust';
    case 'swift':
      return 'swift';
    case 'sh':
    case 'bash':
    case 'zsh':
      return 'shell';
    case 'ps1':
    case 'psm1':
      return 'powershell';
    case 'py':
      return 'python';
    case 'rb':
      return 'ruby';
    case 'php':
      return 'php';
    case 'dart':
      return 'dart';
    case 'lua':
      return 'lua';
    case 'go':
      return 'go';
    case 'java':
      return 'java';
    case 'kt':
    case 'kts':
      return 'kotlin';
    case 'c':
    case 'cc':
    case 'cpp':
    case 'cxx':
    case 'h':
    case 'hh':
    case 'hpp':
    case 'hxx':
      return 'cpp';
    case 'tf':
    case 'tfvars':
      return 'hcl';
    case 'proto':
      return 'protobuf';
    case 'sql':
      return 'sql';
    case 'graphql':
    case 'gql':
      return 'graphql';
    case 'fs':
    case 'fsx':
      return 'fsharp';
    case 'razor':
      return 'razor';
    default:
      return 'plain';
  }
}

function skipDirReason(name: string) {
  const normalizedName = name.toLowerCase();
  if (normalizedName.endsWith('_files')) return 'saved web page asset directory';

  if (['.git', '.hg', '.svn'].includes(normalizedName)) {
    return 'version-control metadata directory';
  }

  if (
    [
      '.agents',
      '.claude',
      '.codex',
      '.dev',
      '.history',
      '.idea',
      '.omx',
      '.playwright',
      '.playwright-cli',
      '.run',
      '.slots',
      '.vscode',
      '.zed'
    ].includes(normalizedName)
  ) {
    return 'agent/tool state directory';
  }

  if (
    [
      '__pycache__',
      '.cache',
      '.build',
      '.gradle',
      '.next',
      '.nuxt',
      '.parcel-cache',
      '.pytest_cache',
      '.svelte-kit',
      '.tmp',
      '.turbo',
      '.vite',
      'bin',
      'build',
      'coverage',
      'deriveddata',
      'dist',
      'obj',
      'target',
      'testresults'
    ].includes(normalizedName)
  ) {
    return 'build output/cache directory';
  }

  if (normalizedName === '.merge-backups') return 'merge backup directory';
  if (['node_modules', 'pods', 'vendor'].includes(normalizedName)) return 'dependency directory';
  if (normalizedName === 'worktrees') return 'session worktree directory';

  return null;
}

function sourceFileMatchesQuery(relativePath: string, fileName: string, query: string | null) {
  if (!query) return true;
  return relativePath.toLowerCase().includes(query) || fileName.toLowerCase().includes(query);
}

function compareSourceWalkEntries(
  root: string,
  current: string,
  left: { name: string; isDirectory(): boolean; isFile(): boolean },
  right: { name: string; isDirectory(): boolean; isFile(): boolean }
) {
  const leftPath = path.join(current, left.name);
  const rightPath = path.join(current, right.name);
  const leftRelativePath = normalizeRelativePath(path.relative(root, leftPath));
  const rightRelativePath = normalizeRelativePath(path.relative(root, rightPath));
  const leftScore = sourcePathPriority(leftRelativePath, left.isFile() ? detectLanguage(leftPath) : '', left.isDirectory(), root);
  const rightScore = sourcePathPriority(rightRelativePath, right.isFile() ? detectLanguage(rightPath) : '', right.isDirectory(), root);

  return leftScore - rightScore || localizedPathCompare(leftRelativePath, rightRelativePath);
}

function compareSourceRecords(left: SourceRecord, right: SourceRecord) {
  const leftScore = sourcePathPriority(left.relativePath, left.language, false, '');
  const rightScore = sourcePathPriority(right.relativePath, right.language, false, '');

  return leftScore - rightScore || localizedPathCompare(left.relativePath, right.relativePath);
}

function sourcePathPriority(
  relativePath: string,
  language: string,
  isDirectory: boolean,
  root: string
) {
  const normalizedPath = relativePath.split(path.sep).join('/');
  const segments = normalizedPath
    .toLowerCase()
    .split('/')
    .filter(Boolean);
  const firstSegment = segments[0] ?? '';
  const projectName = path.basename(root).toLowerCase();
  let score = isDirectory ? -6 : 0;

  score += sourceRootPriority(firstSegment, projectName);
  score += sourceLanguagePriority(language);
  score += sourcePathSegmentAdjustment(segments);

  if (firstSegment.startsWith('.')) score += 80;

  return score;
}

function sourceRootPriority(firstSegment: string, projectName: string) {
  if (!firstSegment) return 100;

  const projectPrefix = projectName ? `${projectName.toLowerCase()}.` : '';
  if (projectName && (firstSegment === projectName || firstSegment.startsWith(projectPrefix))) {
    return projectRootSegmentAdjustment(firstSegment);
  }

  if (['src', 'source', 'sources', 'lib', 'app', 'apps', 'packages'].includes(firstSegment)) {
    return 0;
  }

  if (looksLikeSourceProjectSegment(firstSegment)) return 8 + projectRootSegmentAdjustment(firstSegment);
  if (firstSegment.includes('test') || firstSegment.includes('spec')) return 26;
  if (['scripts', 'tools'].includes(firstSegment)) return 34;
  if (['config', 'deploy', 'infra', 'infrastructure', '.config', '.github'].includes(firstSegment)) return 60;
  if (['docs', 'doc', 'documentation'].includes(firstSegment)) return 75;

  return 45;
}

function looksLikeSourceProjectSegment(segment: string) {
  return (
    segment.includes('.') ||
    segment.endsWith('-web') ||
    segment.endsWith('-api') ||
    segment.endsWith('-core') ||
    segment.endsWith('-engine') ||
    segment.endsWith('-data') ||
    segment === 'web' ||
    segment === 'client' ||
    segment === 'server'
  );
}

function projectRootSegmentAdjustment(segment: string) {
  if (segment.endsWith('.core') || segment.endsWith('-core')) return -8;
  if (segment.endsWith('.api') || segment.endsWith('-api')) return -6;
  if (segment.endsWith('.engine') || segment.endsWith('-engine')) return -5;
  if (segment.endsWith('.data') || segment.endsWith('-data')) return -4;
  if (segment.endsWith('-web') || segment === 'web' || segment === 'client') return -3;
  if (segment.includes('test') || segment.includes('spec')) return 18;
  return 0;
}

function sourceLanguagePriority(language: string) {
  if (
    [
      'csharp',
      'typescript',
      'tsx',
      'svelte',
      'javascript',
      'jsx',
      'rust',
      'swift',
      'go',
      'python',
      'java',
      'kotlin',
      'cpp',
      'dart',
      'fsharp',
      'razor'
    ].includes(language)
  ) {
    return 0;
  }

  if (
    [
      'json',
      'yaml',
      'toml',
      'xml',
      'shell',
      'powershell',
      'sql',
      'graphql',
      'protobuf',
      'hcl',
      'dockerfile',
      'makefile'
    ].includes(language)
  ) {
    return 28;
  }

  if (language === 'markdown' || language === 'mdx') return 55;
  return 80;
}

function sourcePathSegmentAdjustment(segments: string[]) {
  let score = 0;
  const segmentSet = new Set(segments);

  if (
    ['services', 'controllers', 'routes', 'components', 'pages', 'models', 'entities', 'features'].some(
      (segment) => segmentSet.has(segment)
    )
  ) {
    score -= 7;
  }

  if (
    ['migrations', 'generated', 'snapshots', 'fixtures', 'samples', 'docs', 'documentation'].some(
      (segment) => segmentSet.has(segment)
    )
  ) {
    score += 22;
  }

  return score;
}

function localizedPathCompare(left: string, right: string) {
  const lowerLeft = left.toLowerCase();
  const lowerRight = right.toLowerCase();
  if (lowerLeft < lowerRight) return -1;
  if (lowerLeft > lowerRight) return 1;
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}

async function readTailUtf8(filePath: string, maxBytes: number) {
  const handle = await open(filePath, 'r');
  try {
    const fileStats = await handle.stat();
    const start = Math.max(0, fileStats.size - maxBytes);
    const buffer = Buffer.alloc(fileStats.size - start);
    await handle.read(buffer, 0, buffer.length, start);
    return buffer.toString('utf8');
  } finally {
    await handle.close();
  }
}

async function readHeadUtf8(filePath: string, maxBytes: number) {
  const handle = await open(filePath, 'r');
  try {
    const fileStats = await handle.stat();
    const buffer = Buffer.alloc(Math.min(fileStats.size, maxBytes));
    await handle.read(buffer, 0, buffer.length, 0);
    return buffer.toString('utf8');
  } finally {
    await handle.close();
  }
}

async function readHeadAndTailUtf8(filePath: string, headBytes: number, tailBytes: number) {
  const handle = await open(filePath, 'r');
  try {
    const fileStats = await handle.stat();
    if (fileStats.size <= headBytes + tailBytes) {
      return readFile(filePath, 'utf8');
    }

    const head = Buffer.alloc(headBytes);
    await handle.read(head, 0, head.length, 0);

    const tailStart = Math.max(0, fileStats.size - tailBytes);
    const tail = Buffer.alloc(fileStats.size - tailStart);
    await handle.read(tail, 0, tail.length, tailStart);

    return `${head.toString('utf8')}\n${tail.toString('utf8')}`;
  } finally {
    await handle.close();
  }
}

function parseJsonLines(input: string): Record<string, unknown>[] {
  return input
    .split(/\r?\n/)
    .flatMap((line) => {
      const parsed = parseJsonObject(line);
      return parsed ? [parsed] : [];
    });
}

function parseJsonObject(input: unknown): Record<string, unknown> | null {
  if (input && typeof input === 'object' && !Array.isArray(input)) {
    return input as Record<string, unknown>;
  }

  if (typeof input !== 'string' || !input.trim()) return null;

  try {
    const parsed = JSON.parse(input);
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
      ? parsed as Record<string, unknown>
      : null;
  } catch {
    return null;
  }
}

function objectValue(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function optionalString(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function modelFromValue(value: Record<string, unknown> | null): string | null {
  if (!value) return null;
  return optionalString(value.model) ?? optionalString(value.model_slug) ?? optionalString(value.modelSlug);
}

function titleFromClaudeMessage(value: Record<string, unknown>) {
  const message = objectValue(value.message);
  const text = valueToText(message?.content) ?? valueToText(value.summary);
  if (!text?.trim()) return null;
  return compactText(text, 80);
}

function valueToText(value: unknown): string | null {
  if (typeof value === 'string') return value;
  if (!Array.isArray(value)) return null;

  const text = value
    .flatMap((item) => {
      const object = objectValue(item);
      return optionalString(object?.text) ?? optionalString(object?.content) ?? [];
    })
    .join(' ');
  return text || null;
}

function cmuxSessionTitle(
  agent: string,
  value: Record<string, unknown>,
  status: string | null,
  cwd: string | null
) {
  const agentLabel = agentDisplayLabel(agent);
  const detail =
    optionalString(value.title)
    ?? optionalString(value.name)
    ?? optionalString(value.threadName)
    ?? optionalString(value.conversationTitle)
    ?? optionalString(value.taskTitle)
    ?? optionalString(value.lastSubtitle)
    ?? status
    ?? pathDisplayName(cwd);

  return detail ? `${agentLabel} · ${compactText(detail, 72)}` : `${agentLabel} session`;
}

function cmuxSessionDescription(value: Record<string, unknown>) {
  const text =
    optionalString(value.lastBody)
    ?? optionalString(value.lastMessage)
    ?? optionalString(value.summary);
  return text ? compactText(text, 140) : null;
}

function claudeSessionDescription(value: Record<string, unknown>) {
  const message = objectValue(value.message);
  const text = valueToText(message?.content) ?? valueToText(value.summary);
  return text ? compactText(text, 140) : null;
}

/**
 * The three places a hint can come from, tried in that order: the title, then
 * the folder, then the resume command.
 */
function firstAgentSessionHint(
  record: LocalAgentSessionRecord,
  derive: (text: string) => string | null
) {
  const fromRecord = derive(record.title) ?? (record.projectPath ? derive(record.projectPath) : null);
  if (fromRecord) return fromRecord;

  for (const command of record.resumeCommands) {
    const found = derive(command);
    if (found) return found;
  }

  return null;
}

function agentSessionSourceLabel(record: LocalAgentSessionRecord) {
  const provider = agentSessionProviderLabel(record.provider);
  const title = record.title.trim();
  const detail = pathDisplayName(record.projectPath) ?? (title ? compactText(title, 48) : null);
  return detail ? `${provider} · ${detail}` : provider;
}

function agentSessionProviderLabel(provider: string) {
  const trimmed = provider.trim();
  return trimmed.startsWith('cmux-')
    ? `CMUX ${agentDisplayLabel(trimmed.slice('cmux-'.length))}`
    : agentDisplayLabel(trimmed);
}

/**
 * The name after the word "branch", however it was written — `branch:`,
 * `branch=`, or the bare word. The word has to stand on its own: `rebranch` is
 * not a branch marker.
 */
function branchHintFromText(text: string): string | null {
  const lowerText = asciiLowerCase(text);
  for (const marker of ['branch:', 'branch=', 'branch ']) {
    let searchStart = 0;
    for (;;) {
      const index = lowerText.indexOf(marker, searchStart);
      if (index < 0) break;
      if (index > 0 && isAsciiAlphanumeric(lowerText[index - 1])) {
        searchStart = index + marker.length;
        continue;
      }

      const branch = gitRefTokenFromText(text.slice(index + marker.length));
      if (branch) return branch;
      searchStart = index + marker.length;
    }
  }

  return null;
}

/** A branch name, with whatever quoting it was written inside taken off. */
function gitRefTokenFromText(text: string): string | null {
  let start = 0;
  while (start < text.length && (isWhitespace(text[start]) || '`"\''.includes(text[start]))) {
    start += 1;
  }

  let end = start;
  while (
    end < text.length
    && (isAsciiAlphanumeric(text[end]) || '._-/'.includes(text[end]))
  ) {
    end += 1;
  }

  const token = text.slice(start, end);
  if (!token || [...token].every((character) => '._-/'.includes(character))) return null;
  return token;
}

/** `tsk` plus digits, in any of the shapes people write it. Digits are what
 * makes it an id, so `tsk-abc` is not one. */
function taskIdFromText(text: string): string | null {
  const lowerText = asciiLowerCase(text);
  let searchStart = 0;
  for (;;) {
    const index = lowerText.indexOf('tsk', searchStart);
    if (index < 0) break;
    searchStart = index + 3;
    if (index > 0 && isAsciiAlphanumeric(lowerText[index - 1])) continue;

    const digits = leadingDigits(trimStartOf(lowerText.slice(index + 3), '-_/#[ :'));
    if (digits) return `TSK-${digits}`;
  }

  return null;
}

/** A pull request number, from the words or from a GitHub link. */
function pullRequestHintFromText(text: string): string | null {
  const number =
    pullRequestNumberAfterMarker(text, 'pull request')
    ?? pullRequestNumberAfterMarker(text, 'pr')
    ?? githubPullRequestNumberFromUrl(linkHintFromText(text));
  return number ? `PR #${number}` : null;
}

function pullRequestNumberAfterMarker(text: string, marker: string): string | null {
  const lowerText = asciiLowerCase(text);
  let searchStart = 0;
  for (;;) {
    const index = lowerText.indexOf(marker, searchStart);
    if (index < 0) break;
    searchStart = index + marker.length;
    if (index > 0 && isAsciiAlphanumeric(lowerText[index - 1])) continue;

    const digits = leadingDigits(trimStartOf(text.slice(index + marker.length), ' #-:'));
    if (digits) return digits;
  }

  return null;
}

/** The first link in the text, with the sentence punctuation around it left off. */
function linkHintFromText(text: string): string | null {
  for (const scheme of ['https://', 'http://']) {
    let searchStart = 0;
    for (;;) {
      const index = text.indexOf(scheme, searchStart);
      if (index < 0) break;

      const suffix = text.slice(index);
      const whitespace = suffix.search(/\s/);
      const link = trimEndOf(
        suffix.slice(0, whitespace < 0 ? suffix.length : whitespace),
        '.,;:)]}"'
      );
      if (link) return link;

      searchStart = index + scheme.length;
    }
  }

  return null;
}

function githubPullRequestNumberFromUrl(url: string | null): string | null {
  if (!url) return null;
  const lowerUrl = asciiLowerCase(url);
  if (!lowerUrl.includes('github.com/')) return null;

  const index = lowerUrl.indexOf('/pull/');
  if (index < 0) return null;
  return leadingDigits(url.slice(index + '/pull/'.length));
}

/**
 * ASCII-only, because the Rust side lowercases ASCII only. A general
 * `toLowerCase` can change a string's length on some letters, and every one of
 * these helpers indexes back into the original text with an offset found in the
 * lowercased copy.
 */
function asciiLowerCase(value: string) {
  return value.replace(/[A-Z]/g, (character) => character.toLowerCase());
}

function isAsciiAlphanumeric(character: string) {
  return /^[0-9A-Za-z]$/.test(character);
}

function isWhitespace(character: string) {
  return /^\s$/.test(character);
}

function leadingDigits(value: string) {
  const digits = /^[0-9]+/.exec(value);
  return digits ? digits[0] : null;
}

function trimStartOf(value: string, characters: string) {
  let start = 0;
  while (start < value.length && characters.includes(value[start])) start += 1;
  return value.slice(start);
}

function trimEndOf(value: string, characters: string) {
  let end = value.length;
  while (end > 0 && characters.includes(value[end - 1])) end -= 1;
  return value.slice(0, end);
}

function agentDisplayLabel(agent: string) {
  switch (agent) {
    case 'codex':
      return 'Codex';
    case 'claude':
      return 'Claude';
    case 'gemini':
      return 'Gemini';
    case 'opencode':
      return 'OpenCode';
    case 'cursor':
    case 'cursor-agent':
      return 'Cursor';
    case 'antigravity':
    case 'agy':
      return 'Antigravity';
    case 'rovo':
    case 'acli':
      return 'Rovo';
    default:
      return agent ? `${agent.slice(0, 1).toUpperCase()}${agent.slice(1)}` : 'Agent';
  }
}

function cmuxResumeCommands(agent: string, id: string, cwd: string | null) {
  const command = (() => {
    switch (agent) {
      case 'codex':
        return `codex resume ${id}`;
      case 'claude':
        return `claude --resume ${id}`;
      case 'gemini':
        return `gemini --resume ${id}`;
      case 'opencode':
        return `opencode --session ${id}`;
      case 'amp':
        return `amp threads continue ${id}`;
      case 'antigravity':
      case 'agy':
        return `agy --conversation ${id}`;
      case 'rovo':
      case 'acli':
        return `acli rovodev run --restore ${id}`;
      case 'cursor':
      case 'cursor-agent':
        return `cursor-agent --resume ${id}`;
      default:
        return `${agent} --resume ${id}`;
    }
  })();

  return cwd ? [command, `cd ${shellQuote(cwd)} && ${command}`] : [command];
}

function timestampishString(value: unknown): string | null {
  if (typeof value === 'number') return unixTimestampNumberToIso(value);
  if (typeof value !== 'string') return null;

  const trimmed = value.trim();
  if (!trimmed) return null;

  const numericValue = Number(trimmed);
  return Number.isFinite(numericValue) ? unixTimestampNumberToIso(numericValue) ?? trimmed : trimmed;
}

function unixTimestampNumberToIso(value: number): string | null {
  if (!Number.isFinite(value) || value < 0) return null;
  const millis = value >= 1_000_000_000_000 ? value : value * 1000;
  return new Date(millis).toISOString();
}

function decodeClaudeProjectDir(name: string) {
  if (!name.trim()) return null;
  const segments = name.split('-').filter(Boolean);
  return segments.length > 0 ? `/${segments.join('/')}` : null;
}

function compactText(value: string, maxChars: number) {
  const trimmed = value.split(/\s+/).filter(Boolean).join(' ');
  return trimmed.length <= maxChars ? trimmed : `${trimmed.slice(0, Math.max(0, maxChars - 1))}...`;
}

function pathDisplayName(value: string | null) {
  if (!value?.trim()) return null;
  return path.basename(value.trim()) || null;
}

function shellQuote(value: string) {
  return `'${value.replace(/'/g, `'\\''`)}'`;
}

function compareNullableStringsDescending(left: string | null, right: string | null) {
  if (left === right) return 0;
  if (left === null) return 1;
  if (right === null) return -1;
  return right.localeCompare(left);
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
