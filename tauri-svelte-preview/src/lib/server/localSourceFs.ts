import { mkdir, readdir, readFile, rename, rm, stat, writeFile } from 'node:fs/promises';
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

type LocalSourceScanInput = {
  root: string;
  query?: string | null;
  limit?: number | null;
};

export type LocalProjectRootValidationResult = {
  path: string;
  exists: boolean;
  isDirectory: boolean;
  isGitRepository: boolean;
  message: string;
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
      message: 'Project path not found'
    };
  }

  if (!rootStats.isDirectory()) {
    return {
      path: normalizedRoot,
      exists: true,
      isDirectory: false,
      isGitRepository: false,
      message: 'Project path points to a file. Choose the repository folder instead.'
    };
  }

  const gitPath = path.join(normalizedRoot, '.git');
  const isGitRepository = Boolean(await stat(gitPath).catch(() => null));

  return {
    path: normalizedRoot,
    exists: true,
    isDirectory: true,
    isGitRepository,
    message: isGitRepository
      ? 'Project root ready'
      : 'Folder is not a Git repository. Source browsing will work, but Git/worktree panels may be unavailable.'
  };
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
  const collectLimit = limit + 1;
  const query = input.query?.trim().toLowerCase() || null;
  const records: SourceRecord[] = [];
  const stats = createSourceScanStats();
  await collectSourceFiles(root, root, collectLimit, query, records, stats);

  records.sort(compareSourceRecords);
  const truncated = records.length > limit;
  records.splice(limit);

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
      if (shouldSkipDir(entry.name)) {
        scanStats.skippedDirectories += 1;
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

function createSourceScanStats(): SourceScanStats {
  return {
    visitedEntries: 0,
    matchedFiles: 0,
    skippedDirectories: 0,
    unsupportedFiles: 0,
    unreadableEntries: 0
  };
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
    case 'tsx':
      return 'typescript';
    case 'js':
    case 'jsx':
    case 'mjs':
    case 'cjs':
      return 'javascript';
    case 'svelte':
      return 'svelte';
    case 'json':
    case 'jsonc':
      return 'json';
    case 'md':
    case 'mdx':
      return 'markdown';
    case 'yml':
    case 'yaml':
      return 'yaml';
    case 'toml':
      return 'toml';
    case 'xml':
    case 'xsd':
    case 'csproj':
    case 'props':
    case 'targets':
      return 'xml';
    case 'html':
      return 'html';
    case 'css':
    case 'scss':
      return 'css';
    case 'rs':
      return 'rust';
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

function shouldSkipDir(name: string) {
  const normalizedName = name.toLowerCase();
  if (normalizedName.endsWith('_files')) return true;

  return new Set([
    '__pycache__',
    '.cache',
    '.git',
    '.hg',
    '.svn',
    '.agents',
    '.build',
    '.claude',
    '.codex',
    '.dev',
    '.gradle',
    '.history',
    '.idea',
    '.merge-backups',
    '.next',
    '.nuxt',
    '.omx',
    '.parcel-cache',
    '.playwright',
    '.playwright-cli',
    '.pytest_cache',
    '.run',
    '.slots',
    '.svelte-kit',
    '.tmp',
    '.turbo',
    '.vite',
    '.vscode',
    '.zed',
    'bin',
    'build',
    'coverage',
    'deriveddata',
    'dist',
    'node_modules',
    'obj',
    'pods',
    'target',
    'testresults',
    'vendor',
    'worktrees'
  ]).has(normalizedName);
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

  if (language === 'markdown') return 55;
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
  return left.toLowerCase().localeCompare(right.toLowerCase());
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
