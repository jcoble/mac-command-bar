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
const claudeSessionFileLimit = 512;
const claudeSessionTailBytes = 256 * 1024;
const codexSessionFileLimit = 512;
const codexSessionHeadBytes = 64 * 1024;
const codexSessionTailBytes = 256 * 1024;
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
};

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
  const codexRecords: LocalAgentSessionRecord[] = [];
  const codexIndex = path.join(homePath, '.codex', 'session_index.jsonl');
  const codexIndexContents = await readFile(codexIndex, 'utf8').catch(() => '');
  if (codexIndexContents) {
    codexRecords.push(...parseCodexIndexJsonl(codexIndexContents));
  }

  const codexFiles = await sortFilesByModifiedDesc(
    await jsonlFiles(path.join(homePath, '.codex', 'sessions'))
  );
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

  const claudeFiles = await sortFilesByModifiedDesc(
    await jsonlFiles(path.join(homePath, '.claude', 'projects'))
  );
  for (const filePath of claudeFiles.slice(0, claudeSessionFileLimit)) {
    const projectPath = decodeClaudeProjectDir(path.basename(path.dirname(filePath))) ?? '';
    const contents = await readTailUtf8(filePath, claudeSessionTailBytes).catch(() => '');
    if (contents) records.push(...parseClaudeJsonl(contents, projectPath));
  }

  return mergeAgentSessionRecords(records)
    .sort((left, right) => compareNullableStringsDescending(left.lastActivity, right.lastActivity))
    .slice(0, agentSessionResultLimit);
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

function parseCodexIndexJsonl(input: string): LocalAgentSessionRecord[] {
  return parseJsonLines(input).flatMap((value) => {
    const id = optionalString(value.id);
    if (!id) return [];

    return [
      {
        provider: 'codex',
        id,
        title: optionalString(value.thread_name) ?? 'Untitled Codex session',
        description: null,
        model: modelFromValue(value),
        projectPath: null,
        lastActivity: optionalString(value.updated_at),
        resumeCommands: [`codex resume ${id}`]
      }
    ];
  });
}

function parseCodexRolloutJsonl(input: string): LocalAgentSessionRecord[] {
  const records: LocalAgentSessionRecord[] = [];

  for (const value of parseJsonLines(input)) {
    const type = optionalString(value.type);
    if (type === 'session_meta') {
      const payload = objectValue(value.payload);
      const id = optionalString(payload?.id);
      if (!payload || !id) continue;

      const record: LocalAgentSessionRecord = {
        provider: 'codex',
        id,
        title: 'Codex session',
        description: null,
        model: modelFromValue(payload),
        projectPath: optionalString(payload.cwd),
        lastActivity: optionalString(value.timestamp) ?? optionalString(payload.timestamp),
        resumeCommands: [`codex resume ${id}`]
      };
      upsertAgentSessionRecord(records, record);
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
      const cwd = codexResponseItemWorkdir(value);
      const description = codexResponseItemDescription(value);
      if (!cwd && !description) continue;
      updateLatestCodexRecord(records, cwd, description, null, optionalString(value.timestamp));
    }
  }

  return records;
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

function mergeCodexSessionMetadata(
  indexed: LocalAgentSessionRecord[],
  metadata: LocalAgentSessionRecord[]
) {
  for (const record of metadata) {
    upsertAgentSessionRecord(indexed, record);
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

function parseClaudeJsonl(input: string, projectPath: string): LocalAgentSessionRecord[] {
  const records: LocalAgentSessionRecord[] = [];

  for (const value of parseJsonLines(input)) {
    const id = optionalString(value.sessionId) ?? optionalString(value.session_id);
    if (!id) continue;

    const cwd = optionalString(value.cwd) ?? projectPath;
    upsertAgentSessionRecord(records, {
      provider: 'claude',
      id,
      title: titleFromClaudeMessage(value) ?? 'Claude session',
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

  return records;
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
