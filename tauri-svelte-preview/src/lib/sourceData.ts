export type SourceLanguage = string;

export type ProjectRoot = {
  id: string;
  name: string;
  path: string;
};

export type SourceRecord = {
  path: string;
  relativePath: string;
  fileName: string;
  language: SourceLanguage;
  byteCount: number;
};

export type SourcePreview = SourceRecord & {
  content: string;
  lineCount: number;
};

export type SourceSearchMatch = SourceRecord & {
  line: number;
  column: number;
  excerpt: string;
};

export type SourceDefinitionTarget = SourceRecord & {
  symbolName: string;
  kind: string;
  line: number;
  column: number;
  detail: string;
};

export type SourceReferenceTarget = SourceRecord & {
  symbolName: string;
  line: number;
  column: number;
  excerpt: string;
};

export type SourceLspStatus = {
  language: SourceLanguage;
  languageID: string;
  available: boolean;
  serverName: string;
  command: string;
  args: string[];
  reason: string | null;
};

export type SourceLspLookupRequest = {
  root: string;
  line: number;
  column: number;
  limit?: number;
};

export type SourceLspHover = {
  contents: string[];
};

export type SourceDiagnosticSeverity = 'error' | 'warning' | 'info' | 'hint';

export type SourceDiagnostic = {
  severity: SourceDiagnosticSeverity;
  message: string;
  line: number;
  column: number;
  source?: string;
};

export type SourceSymbol = {
  name: string;
  kind: string;
  line: number;
  column: number;
  detail: string;
};

export const sourceSemanticTokenLegend = {
  tokenTypes: [
    'namespace',
    'class',
    'interface',
    'type',
    'enum',
    'function',
    'method',
    'variable'
  ],
  tokenModifiers: []
} as const;

export type SourceSemanticTokenType = (typeof sourceSemanticTokenLegend.tokenTypes)[number];

export type SourceSemanticToken = {
  tokenType: SourceSemanticTokenType;
  line: number;
  startColumn: number;
  length: number;
};

export type SourceScanResult = {
  records: SourceRecord[];
  limit: number;
  truncated: boolean;
};

export type SourceRecentRecord = SourceRecord & {
  projectID: string;
  projectName: string;
  openedAt: number;
};

export type SourceOpenTab = SourceRecentRecord;

export type SourceScanCacheEntry = {
  key: string;
  projectID: string;
  projectPath: string;
  limit: number;
  truncated: boolean;
  records: SourceRecord[];
  scannedAt: number;
};

export type SourceScanCache = Record<string, SourceScanCacheEntry>;

export type GitTaskMetadata = {
  taskID?: string | null;
};

export type GitCommitGraphKind = 'head' | 'branch' | 'commit';

export type SourceContextGitStatus = {
  branch?: string | null;
  ahead: number;
  behind: number;
  files: Array<unknown>;
};

export type SourceContextIdentity = {
  projectName: string;
  rootLabel: string;
  rootPath: string;
  gitSummary: string;
  runtime: string;
  summary: string;
};

export type CloseOpenSourceTabResult = {
  tabs: SourceOpenTab[];
  nextActivePath: string | null;
};

export type QuickOpenQuery = {
  searchQuery: string;
  targetLine: number | null;
};

export type SourceTreeNode = {
  id: string;
  name: string;
  relativePath: string;
  file: SourceRecord | null;
  children: SourceTreeNode[];
};

export type SourceTreeRow = {
  node: SourceTreeNode;
  level: number;
};

export type VirtualSourceTreeRows = {
  rows: SourceTreeRow[];
  startIndex: number;
  endIndex: number;
  topSpacerHeight: number;
  bottomSpacerHeight: number;
  totalHeight: number;
};

export const defaultProjectRoots: ProjectRoot[] = [
  {
    id: 'ediplatform',
    name: 'EdiPlatform',
    path: '/Users/blackcolours/dev/work/EdiPlatform'
  },
  {
    id: 'mac-command-bar',
    name: 'MacCommandBar',
    path: '/Users/blackcolours/dev/work/mac-command-bar'
  }
];

export const projectRoots = defaultProjectRoots;

export function normalizeProjectPath(path: string): string {
  return path.trim().replace(/\/+$/, '');
}

export function projectRootNameFromPath(path: string): string {
  const normalizedPath = normalizeProjectPath(path);
  return normalizedPath.split('/').filter(Boolean).at(-1) ?? 'Project';
}

export function createProjectRoot(name: string, path: string): ProjectRoot {
  const normalizedPath = normalizeProjectPath(path);
  return {
    id: `custom-${stableIDFor(normalizedPath)}`,
    name: name.trim() || projectRootNameFromPath(normalizedPath),
    path: normalizedPath
  };
}

export function formatSourceContextRootLabel(path: string): string {
  const normalizedPath = normalizeProjectPath(path);
  const segments = normalizedPath.split('/').filter(Boolean);
  const worktreesIndex = segments.findIndex((segment) => segment === 'worktrees');

  if (worktreesIndex >= 0 && segments[worktreesIndex + 2]) {
    return `worktree:${segments[worktreesIndex + 2]}`;
  }

  if (segments.at(-2) === 'work') {
    return 'main checkout';
  }

  return projectRootNameFromPath(normalizedPath);
}

export function formatSourceContextRuntime(runtime: string): string {
  return runtime.trim() || 'browser preview';
}

export function formatSourceContextGitSummary(
  status: SourceContextGitStatus | null,
  loadingGit: boolean,
  gitError: string
): string {
  if (loadingGit) return 'git loading';
  if (!status) return gitError ? 'git unavailable' : 'git clean';

  const branch = status.branch ?? 'detached';
  const syncParts = [
    status.ahead > 0 ? `ahead ${status.ahead}` : '',
    status.behind > 0 ? `behind ${status.behind}` : ''
  ].filter(Boolean);
  const changeLabel =
    status.files.length === 0
      ? 'clean'
      : `${status.files.length} change${status.files.length === 1 ? '' : 's'}`;

  return [branch, ...syncParts, changeLabel].join(' · ');
}

export function formatSourceContextIdentity(
  project: ProjectRoot,
  gitSummary: string,
  runtime: string
): SourceContextIdentity {
  const projectName = project.name.trim() || projectRootNameFromPath(project.path);
  const rootPath = normalizeProjectPath(project.path);
  const rootLabel = formatSourceContextRootLabel(rootPath);
  const normalizedGitSummary = gitSummary.trim() || 'git clean';
  const normalizedRuntime = formatSourceContextRuntime(runtime);

  return {
    projectName,
    rootLabel,
    rootPath,
    gitSummary: normalizedGitSummary,
    runtime: normalizedRuntime,
    summary: [projectName, rootLabel, normalizedGitSummary, normalizedRuntime].join(' · ')
  };
}

export function mergeProjectRoots(defaultRoots: ProjectRoot[], customRoots: ProjectRoot[]): ProjectRoot[] {
  const seenPaths = new Set<string>();
  const mergedRoots: ProjectRoot[] = [];

  for (const root of [...defaultRoots, ...customRoots]) {
    const normalizedPath = normalizeProjectPath(root.path);
    if (!root.id || !root.name.trim() || !normalizedPath || seenPaths.has(normalizedPath)) {
      continue;
    }

    seenPaths.add(normalizedPath);
    mergedRoots.push({
      ...root,
      name: root.name.trim(),
      path: normalizedPath
    });
  }

  return mergedRoots;
}

function stableIDFor(value: string): string {
  let hash = 5381;
  for (const char of value) {
    hash = (hash * 33) ^ char.charCodeAt(0);
  }
  return (hash >>> 0).toString(36);
}

const formatResolverContent = String.raw`using EdiPlatform.Core.Entities;
using EdiPlatform.Core.Models.RetailerRuntime;
using Microsoft.Extensions.Logging;

namespace EdiPlatform.Core.Services;

/// <summary>
/// Resolves which EDI format to use for a file, considering trading partner
/// profile settings and runtime package metadata.
/// </summary>
public class FormatResolver
{
    private readonly FormatDetector _detector;
    private readonly ILogger<FormatResolver>? _logger;

    public FormatResolver(FormatDetector detector, ILogger<FormatResolver>? logger = null)
    {
        _detector = detector;
        _logger = logger;
    }

    public async Task<ResolvedFormat> ResolveAsync(
        Stream source,
        TradingPartnerProfile profile,
        CancellationToken cancellationToken = default)
    {
        var detection = await _detector.DetectAsync(source, cancellationToken);
        if (detection.DocumentType is null)
        {
            _logger?.LogWarning("Could not detect document type for {Partner}", profile.Name);
            return ResolvedFormat.Unknown(profile.Id);
        }

        var package = profile.RuntimePackages
            .FirstOrDefault(item => item.DocumentType == detection.DocumentType);

        return package is null
            ? ResolvedFormat.Unsupported(profile.Id, detection.DocumentType)
            : ResolvedFormat.Runtime(profile.Id, package.PackageId, detection.DocumentType);
    }
}`;

const formatDetectorContent = String.raw`using System.Buffers.Text;
using System.Text;

namespace EdiPlatform.Core.Services;

public sealed class FormatDetector
{
    private static readonly byte[] IsaMarker = "ISA"u8.ToArray();
    private static readonly byte[] StMarker = "ST"u8.ToArray();

    public async Task<FormatDetection> DetectAsync(
        Stream source,
        CancellationToken cancellationToken = default)
    {
        var buffer = new byte[Math.Min(source.Length, 16_384)];
        var read = await source.ReadAsync(buffer.AsMemory(0, buffer.Length), cancellationToken);
        var slice = buffer.AsSpan(0, read);

        if (slice.StartsWith(IsaMarker))
        {
            return DetectX12(slice);
        }

        return FormatDetection.Unknown();
    }

    private static FormatDetection DetectX12(ReadOnlySpan<byte> source)
    {
        var text = Encoding.ASCII.GetString(source);
        var segments = text.Split('~', StringSplitOptions.RemoveEmptyEntries);
        foreach (var segment in segments)
        {
            if (!segment.StartsWith("ST*", StringComparison.Ordinal))
            {
                continue;
            }

            var elements = segment.Split('*');
            return elements.Length > 1
                ? FormatDetection.X12(elements[1])
                : FormatDetection.Unknown();
        }

        return FormatDetection.Unknown();
    }
}`;

const coreClientContent = String.raw`public struct CoreClient: CoreSending {
    public var executableURL: URL

    public init(executableURL: URL) {
        self.executableURL = executableURL
    }

    public func send(_ request: CoreRequest) async throws -> CoreResponse {
        try await Task.detached(priority: .utility) {
            try Self.sendSync(request, executableURL: executableURL)
        }.value
    }
}`;

const sourceIntelligenceContent = String.raw`export type SourceDiagnostic = {
  severity: 'error' | 'warning' | 'info' | 'hint';
  message: string;
  line: number;
  column: number;
};

export class SourceIntelligenceIndex {
  private diagnostics = new Map<string, SourceDiagnostic[]>();

  record(path: string, diagnostics: SourceDiagnostic[]) {
    this.diagnostics.set(path, diagnostics);
  }

  problemsFor(path: string) {
    return this.diagnostics.get(path) ?? [];
  }
}

export function formatDiagnosticCount(diagnostics: SourceDiagnostic[]) {
  return diagnostics.length === 0 ? 'No problems' : diagnostics.length + ' problems';
}`;

export const sourceRecords: SourceRecord[] = [
  {
    path: '/Users/blackcolours/dev/work/EdiPlatform/EdiPlatform.Core/Services/FormatResolver.cs',
    relativePath: 'EdiPlatform.Core/Services/FormatResolver.cs',
    fileName: 'FormatResolver.cs',
    language: 'csharp',
    byteCount: 8037
  },
  {
    path: '/Users/blackcolours/dev/work/EdiPlatform/EdiPlatform.Core/Services/FormatDetector.cs',
    relativePath: 'EdiPlatform.Core/Services/FormatDetector.cs',
    fileName: 'FormatDetector.cs',
    language: 'csharp',
    byteCount: 12047
  },
  {
    path: '/Users/blackcolours/dev/work/mac-command-bar/Sources/MacCommandBarKit/CoreClient.swift',
    relativePath: 'Sources/MacCommandBarKit/CoreClient.swift',
    fileName: 'CoreClient.swift',
    language: 'swift',
    byteCount: 2900
  },
  {
    path: '/Users/blackcolours/dev/work/mac-command-bar/tauri-svelte-preview/src/lib/sourceIntelligence.ts',
    relativePath: 'tauri-svelte-preview/src/lib/sourceIntelligence.ts',
    fileName: 'sourceIntelligence.ts',
    language: 'typescript',
    byteCount: 868
  }
];

const demoContentByPath = new Map<string, string>([
  [sourceRecords[0].path, formatResolverContent],
  [sourceRecords[1].path, formatDetectorContent],
  [sourceRecords[2].path, coreClientContent],
  [sourceRecords[3].path, sourceIntelligenceContent]
]);

export function demoPreviewFor(record: SourceRecord): SourcePreview {
  const content = demoContentByPath.get(record.path) ?? '';
  return previewFromContent(record, content);
}

export function demoRecordsForProject(project: ProjectRoot): SourceRecord[] {
  const matches = sourceRecords.filter((record) => record.path.startsWith(`${project.path}/`));
  return matches.length > 0 ? matches : sourceRecords;
}

export function previewFromContent(record: SourceRecord, content: string): SourcePreview {
  return {
    ...record,
    content,
    lineCount: content.length === 0 ? 0 : content.split(/\r\n|\r|\n/).length
  };
}

export function formatSourceRecordCount(
  filteredCount: number,
  totalCount: number,
  truncated: boolean
): string {
  const totalLabel = `${formatCount(totalCount)}${truncated ? '+' : ''}`;
  return filteredCount === totalCount ? totalLabel : `${formatCount(filteredCount)} / ${totalLabel}`;
}

export function formatSourceScanSummary(
  filteredCount: number,
  totalCount: number,
  truncated: boolean,
  query: string
): string {
  const safeFilteredCount = Math.max(0, Math.floor(filteredCount));
  const safeTotalCount = Math.max(0, Math.floor(totalCount));
  const normalizedQuery = query.trim();
  const totalScope = truncated
    ? `first ${formatCount(safeTotalCount)} files`
    : `${formatCount(safeTotalCount)} ${safeTotalCount === 1 ? 'file' : 'files'}`;

  if (normalizedQuery.length === 0) {
    return truncated
      ? `${formatCount(safeTotalCount)} indexed files; scan limit reached`
      : `${formatCount(safeTotalCount)} indexed ${safeTotalCount === 1 ? 'file' : 'files'}`;
  }

  return `${formatCount(safeFilteredCount)} ${
    safeFilteredCount === 1 ? 'match' : 'matches'
  } for "${normalizedQuery}" across ${totalScope}`;
}

export function selectBackgroundIndexProjects(
  projects: ProjectRoot[],
  activeProjectID: string,
  cache: SourceScanCache,
  now: number,
  maxAgeMs: number,
  limit: number
): ProjectRoot[] {
  return projects.filter(
    (project) =>
      project.id !== activeProjectID &&
      getSourceScanCacheEntry(cache, project, limit, now, maxAgeMs) === null
  );
}

export function formatSourceIndexSummary(
  entry: SourceScanCacheEntry | null,
  indexing: boolean,
  error: string
): string {
  if (indexing) return 'Indexing in background';

  const normalizedError = error.trim();
  if (normalizedError) return `Index failed: ${normalizedError}`;

  if (!entry) return 'Index not ready';

  return `Index ready: ${formatCount(entry.records.length)}${entry.truncated ? '+' : ''} ${
    entry.records.length === 1 && !entry.truncated ? 'file' : 'files'
  }`;
}

export function monacoLanguageForSource(language: SourceLanguage): string {
  switch (language) {
    case 'tsx':
      return 'typescript';
    case 'jsx':
      return 'javascript';
    case 'svelte':
      return 'html';
    case 'toml':
      return 'ini';
    case 'plain':
      return 'plaintext';
    default:
      return language;
  }
}

export function sourceSupportsLanguageIntelligence(language: SourceLanguage): boolean {
  return ['typescript', 'tsx', 'javascript', 'jsx', 'csharp'].includes(language);
}

export function textMatchesSearchTokens(
  filter: string,
  ...values: Array<string | number | boolean | null | undefined>
): boolean {
  const tokens = filter
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);
  if (tokens.length === 0) return true;

  const haystack = values.map((value) => String(value ?? '').toLowerCase()).join(' ');
  return tokens.every((token) => haystack.includes(token));
}

export function gitRefLabels(refs: string): string[] {
  return refs
    .split(',')
    .map((ref) => ref.trim())
    .filter(Boolean);
}

export function gitCommitGraphKind(refs: string, _index: number): GitCommitGraphKind {
  const labels = gitRefLabels(refs);
  if (labels.some((label) => label === 'HEAD' || label.startsWith('HEAD ->'))) return 'head';
  if (labels.length > 0) return 'branch';
  return 'commit';
}

export function uniqueTaskIDsFromGitMetadata(...groups: GitTaskMetadata[][]): string[] {
  const seen = new Set<string>();
  const taskIDs: string[] = [];

  for (const group of groups) {
    for (const item of group) {
      const taskID = normalizeTaskID(item.taskID);
      if (!taskID || seen.has(taskID)) continue;

      seen.add(taskID);
      taskIDs.push(taskID);
    }
  }

  return taskIDs;
}

export function formatSourceDiagnosticSummary(diagnostics: SourceDiagnostic[]): string {
  if (diagnostics.length === 0) return 'No problems';

  const errors = diagnostics.filter((diagnostic) => diagnostic.severity === 'error').length;
  const warnings = diagnostics.filter((diagnostic) => diagnostic.severity === 'warning').length;
  const infos = diagnostics.length - errors - warnings;
  const parts = [
    formatProblemCount(errors, 'error'),
    formatProblemCount(warnings, 'warning'),
    formatProblemCount(infos, 'note')
  ].filter(Boolean);

  return parts.join(', ');
}

export function extractSourceSymbols(preview: SourcePreview, content: string): SourceSymbol[] {
  switch (preview.language) {
    case 'typescript':
    case 'tsx':
    case 'javascript':
    case 'jsx':
      return extractTypeScriptSymbols(content);
    case 'csharp':
      return extractCSharpSymbols(content);
    default:
      return [];
  }
}

export function extractSourceSemanticTokens(
  preview: SourcePreview,
  content: string
): SourceSemanticToken[] {
  const lines = content.split(/\r\n|\r|\n/);

  return extractSourceSymbols(preview, content)
    .map((symbol): SourceSemanticToken | null => {
      const tokenType = semanticTokenTypeForSourceSymbol(symbol.kind);
      if (!tokenType) return null;

      const line = lines[symbol.line - 1] ?? '';
      const startIndex = line.lastIndexOf(symbol.name);
      if (startIndex < 0) return null;

      return {
        tokenType,
        line: symbol.line,
        startColumn: startIndex + 1,
        length: symbol.name.length
      };
    })
    .filter((token): token is SourceSemanticToken => token !== null)
    .sort((left, right) => left.line - right.line || left.startColumn - right.startColumn);
}

function formatProblemCount(count: number, label: string): string {
  if (count === 0) return '';
  return `${count} ${label}${count === 1 ? '' : 's'}`;
}

function semanticTokenTypeForSourceSymbol(kind: string): SourceSemanticTokenType | null {
  switch (kind) {
    case 'namespace':
    case 'class':
    case 'interface':
    case 'type':
    case 'enum':
    case 'function':
    case 'method':
      return kind;
    case 'constructor':
      return 'method';
    case 'property':
    case 'constant':
    case 'variable':
      return 'variable';
    case 'record':
    case 'struct':
      return 'class';
    default:
      return null;
  }
}

function extractTypeScriptSymbols(content: string): SourceSymbol[] {
  const symbols: SourceSymbol[] = [];
  const lines = content.split(/\r\n|\r|\n/);

  lines.forEach((line, index) => {
    const trimmed = line.trim();
    const lineNumber = index + 1;
    const column = line.search(/\S/) + 1 || 1;
    const declaration = /^(?:export\s+)?(?:default\s+)?(?:abstract\s+)?(class|interface|type|enum)\s+([A-Za-z_$][\w$]*)/.exec(trimmed);
    if (declaration) {
      symbols.push({
        kind: declaration[1],
        name: declaration[2],
        line: lineNumber,
        column,
        detail: trimmed
      });
      return;
    }

    const functionDeclaration = /^(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_$][\w$]*)\s*\(/.exec(trimmed);
    if (functionDeclaration) {
      symbols.push({
        kind: 'function',
        name: functionDeclaration[1],
        line: lineNumber,
        column,
        detail: trimmed
      });
      return;
    }

    const constantDeclaration = /^(?:export\s+)?const\s+([A-Za-z_$][\w$]*)\s*=/.exec(trimmed);
    if (constantDeclaration) {
      symbols.push({
        kind: 'constant',
        name: constantDeclaration[1],
        line: lineNumber,
        column,
        detail: trimmed
      });
      return;
    }

    const methodDeclaration =
      /^(?:(?:public|private|protected|static|async|readonly|override)\s+)*([A-Za-z_$][\w$]*)\s*\(/.exec(
        trimmed
      );
    if (methodDeclaration && !isControlFlowKeyword(methodDeclaration[1])) {
      symbols.push({
        kind: 'method',
        name: methodDeclaration[1],
        line: lineNumber,
        column,
        detail: trimmed
      });
    }
  });

  return symbols;
}

function isControlFlowKeyword(value: string) {
  return ['for', 'foreach', 'if', 'switch', 'while', 'catch'].includes(value);
}

function extractCSharpSymbols(content: string): SourceSymbol[] {
  const symbols: SourceSymbol[] = [];
  const lines = content.split(/\r\n|\r|\n/);

  lines.forEach((line, index) => {
    const trimmed = line.trim();
    const lineNumber = index + 1;
    const column = line.search(/\S/) + 1 || 1;
    const namespaceDeclaration = /^namespace\s+([A-Za-z_][\w.]*);?/.exec(trimmed);
    if (namespaceDeclaration) {
      symbols.push({
        kind: 'namespace',
        name: namespaceDeclaration[1],
        line: lineNumber,
        column,
        detail: trimmed
      });
      return;
    }

    const typeDeclaration = /^(?:(?:public|private|protected|internal|sealed|abstract|static|partial)\s+)*(class|interface|record|enum)\s+([A-Za-z_][\w]*)/.exec(trimmed);
    if (typeDeclaration) {
      symbols.push({
        kind: typeDeclaration[1],
        name: typeDeclaration[2],
        line: lineNumber,
        column,
        detail: trimmed
      });
      return;
    }

    const fieldDeclaration =
      /^(?:(?:public|private|protected|internal|static|readonly|const|volatile|required|new)\s+)+[\w<>,.?[\]\s]+\s+([A-Za-z_][\w]*)\s*(?:=|;)/.exec(
        trimmed
      );
    if (fieldDeclaration) {
      symbols.push({
        kind: 'variable',
        name: fieldDeclaration[1],
        line: lineNumber,
        column,
        detail: trimmed
      });
      return;
    }

    const constructorDeclaration =
      /^(?:(?:public|private|protected|internal|static|new)\s+)+([A-Z][A-Za-z_0-9]*)\s*\(/.exec(
        trimmed
      );
    if (constructorDeclaration) {
      symbols.push({
        kind: 'constructor',
        name: constructorDeclaration[1],
        line: lineNumber,
        column,
        detail: trimmed
      });
      return;
    }

    const propertyDeclaration =
      /^(?:(?:public|private|protected|internal|static|virtual|override|sealed|required|new)\s+)+[\w<>,.?[\]\s]+\s+([A-Za-z_][\w]*)\s*\{/.exec(
        trimmed
      );
    if (propertyDeclaration) {
      symbols.push({
        kind: 'property',
        name: propertyDeclaration[1],
        line: lineNumber,
        column,
        detail: trimmed
      });
      return;
    }

    const methodDeclaration = /^(?:(?:public|private|protected|internal|static|async|virtual|override|sealed)\s+)+[\w<>,.?[\]\s]+\s+([A-Za-z_][\w]*)\s*\(/.exec(trimmed);
    if (methodDeclaration) {
      symbols.push({
        kind: 'method',
        name: methodDeclaration[1],
        line: lineNumber,
        column,
        detail: trimmed
      });
    }
  });

  return symbols;
}

export function selectPreferredSourceRecord(
  records: SourceRecord[],
  preferredPath: string | null | undefined,
  fallbackPath: string | null | undefined = null
): SourceRecord | null {
  const preferredRecord = preferredPath
    ? records.find((record) => record.path === preferredPath)
    : null;
  if (preferredRecord) return preferredRecord;

  const fallbackRecord = fallbackPath
    ? records.find((record) => record.path === fallbackPath)
    : null;
  return fallbackRecord ?? records[0] ?? null;
}

export function upsertRecentSourceRecord(
  recents: SourceRecentRecord[],
  record: SourceRecord,
  project: ProjectRoot,
  openedAt = Date.now(),
  limit = 10
): SourceRecentRecord[] {
  const cappedLimit = Math.max(0, Math.floor(limit));
  if (cappedLimit === 0) return [];

  return [
    createProjectSourceRecord(record, project, openedAt),
    ...recents.filter((recentRecord) => recentRecord.path !== record.path)
  ].slice(0, cappedLimit);
}

export function getSourceScanCacheEntry(
  cache: SourceScanCache,
  project: ProjectRoot,
  limit: number,
  now = Date.now(),
  maxAgeMs = 5 * 60 * 1000
): SourceScanCacheEntry | null {
  const entry = cache[sourceScanCacheKey(project, limit)];
  if (!entry) return null;
  if (now - entry.scannedAt > maxAgeMs) return null;
  return entry;
}

export function upsertSourceScanCacheEntry(
  cache: SourceScanCache,
  project: ProjectRoot,
  records: SourceRecord[],
  limit: number,
  scannedAt = Date.now(),
  maxEntries = 8,
  truncated = false
): SourceScanCache {
  const cappedMaxEntries = Math.max(0, Math.floor(maxEntries));
  if (cappedMaxEntries === 0) return {};

  const key = sourceScanCacheKey(project, limit);
  const nextEntries = [
    ...Object.values(cache).filter((entry) => entry.key !== key),
    {
      key,
      projectID: project.id,
      projectPath: normalizeProjectPath(project.path),
      limit,
      truncated,
      records,
      scannedAt
    }
  ]
    .sort((left, right) => left.scannedAt - right.scannedAt)
    .slice(-cappedMaxEntries);

  return Object.fromEntries(nextEntries.map((entry) => [entry.key, entry]));
}

function sourceScanCacheKey(project: ProjectRoot, limit: number): string {
  return `${normalizeProjectPath(project.path)}::${Math.max(0, Math.floor(limit))}`;
}

function formatCount(value: number): string {
  return Math.max(0, Math.floor(value)).toLocaleString('en-US');
}

export function upsertOpenSourceTab(
  tabs: SourceOpenTab[],
  record: SourceRecord,
  project: ProjectRoot,
  openedAt = Date.now(),
  limit = 8
): SourceOpenTab[] {
  const cappedLimit = Math.max(0, Math.floor(limit));
  if (cappedLimit === 0) return [];

  const nextTab = createProjectSourceRecord(record, project, openedAt);
  const existingTabIndex = tabs.findIndex((tab) => tab.path === record.path);
  const nextTabs =
    existingTabIndex === -1
      ? [...tabs, nextTab]
      : tabs.map((tab, index) => (index === existingTabIndex ? nextTab : tab));

  return nextTabs.slice(Math.max(0, nextTabs.length - cappedLimit));
}

export function closeOpenSourceTab(
  tabs: SourceOpenTab[],
  closedPath: string,
  activePath: string | null | undefined
): CloseOpenSourceTabResult {
  const closedTabIndex = tabs.findIndex((tab) => tab.path === closedPath);
  if (closedTabIndex === -1) {
    return {
      tabs,
      nextActivePath: activePath ?? tabs[0]?.path ?? null
    };
  }

  const nextTabs = tabs.filter((tab) => tab.path !== closedPath);
  const closedActiveTab = activePath === closedPath;
  if (!closedActiveTab) {
    return {
      tabs: nextTabs,
      nextActivePath: activePath ?? nextTabs[0]?.path ?? null
    };
  }

  return {
    tabs: nextTabs,
    nextActivePath: nextTabs[closedTabIndex]?.path ?? nextTabs[closedTabIndex - 1]?.path ?? null
  };
}

export function filterSourceRecords(records: SourceRecord[], query: string): SourceRecord[] {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) return records;

  return records.filter((record) =>
    `${record.relativePath} ${record.fileName} ${record.language}`.toLowerCase().includes(normalizedQuery)
  );
}

export function rankSourceRecords(
  records: SourceRecord[],
  query: string,
  limit = 20
): SourceRecord[] {
  const cappedLimit = Math.max(0, Math.floor(limit));
  if (cappedLimit === 0) return [];

  const normalizedQuery = parseQuickOpenQuery(query).searchQuery.toLowerCase();
  if (!normalizedQuery) return records.slice(0, cappedLimit);

  return records
    .map((record) => ({
      record,
      score: scoreSourceRecord(record, normalizedQuery)
    }))
    .filter((result): result is { record: SourceRecord; score: number } => result.score !== null)
    .sort(
      (left, right) =>
        left.score - right.score ||
        localizedAscending(left.record.relativePath, right.record.relativePath)
    )
    .slice(0, cappedLimit)
    .map((result) => result.record);
}

export function findSourceSearchMatches(
  previews: SourcePreview[],
  query: string,
  limit = 50
): SourceSearchMatch[] {
  const cappedLimit = Math.max(0, Math.floor(limit));
  if (cappedLimit === 0) return [];

  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) return [];

  const matches: SourceSearchMatch[] = [];
  for (const preview of previews) {
    const lines = preview.content.split(/\r\n|\r|\n/);
    for (const [lineIndex, line] of lines.entries()) {
      const columnIndex = line.toLowerCase().indexOf(normalizedQuery);
      if (columnIndex < 0) continue;

      matches.push({
        path: preview.path,
        relativePath: preview.relativePath,
        fileName: preview.fileName,
        language: preview.language,
        byteCount: preview.byteCount,
        line: lineIndex + 1,
        column: columnIndex + 1,
        excerpt: compactSourceLineExcerpt(line)
      });

      if (matches.length >= cappedLimit) {
        return matches;
      }
    }
  }

  return matches;
}

export function findSourceDefinitionTargets(
  previews: SourcePreview[],
  symbolName: string,
  limit = 20
): SourceDefinitionTarget[] {
  const cappedLimit = Math.max(0, Math.floor(limit));
  if (cappedLimit === 0) return [];

  const normalizedSymbolName = symbolName.trim().toLowerCase();
  if (!normalizedSymbolName) return [];

  const targets: SourceDefinitionTarget[] = [];
  for (const preview of previews) {
    for (const symbol of extractSourceSymbols(preview, preview.content)) {
      if (symbol.name.toLowerCase() !== normalizedSymbolName) continue;

      targets.push({
        path: preview.path,
        relativePath: preview.relativePath,
        fileName: preview.fileName,
        language: preview.language,
        byteCount: preview.byteCount,
        symbolName: symbol.name,
        kind: symbol.kind,
        line: symbol.line,
        column: symbol.detail.indexOf(symbol.name) + 1 || symbol.column,
        detail: symbol.detail
      });

      if (targets.length >= cappedLimit) {
        return targets;
      }
    }
  }

  return targets;
}

export function findSourceReferenceTargets(
  previews: SourcePreview[],
  symbolName: string,
  limit = 50
): SourceReferenceTarget[] {
  const cappedLimit = Math.max(0, Math.floor(limit));
  if (cappedLimit === 0) return [];

  const normalizedSymbolName = symbolName.trim();
  if (!normalizedSymbolName) return [];

  const symbolPattern = new RegExp(`\\b${escapeRegExp(normalizedSymbolName)}\\b`, 'i');
  const targets: SourceReferenceTarget[] = [];
  for (const preview of previews) {
    const lines = preview.content.split(/\r\n|\r|\n/);
    for (const [lineIndex, line] of lines.entries()) {
      const match = symbolPattern.exec(line);
      if (!match) continue;

      targets.push({
        path: preview.path,
        relativePath: preview.relativePath,
        fileName: preview.fileName,
        language: preview.language,
        byteCount: preview.byteCount,
        symbolName: match[0],
        line: lineIndex + 1,
        column: match.index + 1,
        excerpt: compactSourceLineExcerpt(line)
      });

      if (targets.length >= cappedLimit) {
        return targets;
      }
    }
  }

  return targets;
}

export function parseQuickOpenQuery(query: string): QuickOpenQuery {
  const trimmedQuery = query.trim();
  const lineMatch = /^(.*):(\d*)$/.exec(trimmedQuery);
  if (!lineMatch) {
    return {
      searchQuery: trimmedQuery,
      targetLine: null
    };
  }

  if (!lineMatch[2]) {
    return {
      searchQuery: lineMatch[1].trim(),
      targetLine: null
    };
  }

  const parsedLine = Number.parseInt(lineMatch[2], 10);
  if (!Number.isSafeInteger(parsedLine) || parsedLine < 1) {
    return {
      searchQuery: trimmedQuery,
      targetLine: null
    };
  }

  return {
    searchQuery: lineMatch[1].trim(),
    targetLine: parsedLine
  };
}

export function folderIdsForSourceRecord(record: SourceRecord): string[] {
  const folderComponents = record.relativePath.split('/').filter(Boolean).slice(0, -1);
  return folderComponents.map((_, index) => `folder:${folderComponents.slice(0, index + 1).join('/')}`);
}

export function buildSourceTree(records: SourceRecord[]): SourceTreeNode[] {
  const entries = records.map((record) => ({
    components: record.relativePath.split('/').filter(Boolean),
    record
  }));

  return buildTreeNodes(entries, []);
}

export function flattenSourceTree(
  nodes: SourceTreeNode[],
  expandedFolderIds: Set<string>,
  autoExpandFolders: boolean
): SourceTreeRow[] {
  const rows: SourceTreeRow[] = [];

  function visit(nodeList: SourceTreeNode[], level: number) {
    for (const node of nodeList) {
      rows.push({ node, level });
      if (node.file === null && (autoExpandFolders || expandedFolderIds.has(node.id))) {
        visit(node.children, level + 1);
      }
    }
  }

  visit(nodes, 0);
  return rows;
}

export function virtualizeSourceTreeRows(
  rows: SourceTreeRow[],
  scrollTop: number,
  viewportHeight: number,
  rowHeight: number,
  overscanRows: number
): VirtualSourceTreeRows {
  if (rows.length === 0) {
    return {
      rows: [],
      startIndex: 0,
      endIndex: 0,
      topSpacerHeight: 0,
      bottomSpacerHeight: 0,
      totalHeight: 0
    };
  }

  const safeRowHeight = Math.max(1, rowHeight);
  const safeOverscanRows = Math.max(0, Math.floor(overscanRows));
  const visibleRowCount = Math.max(1, Math.ceil(Math.max(0, viewportHeight) / safeRowHeight));
  const maxFirstVisibleIndex = Math.max(0, rows.length - visibleRowCount);
  const firstVisibleIndex = Math.min(
    maxFirstVisibleIndex,
    Math.max(0, Math.floor(Math.max(0, scrollTop) / safeRowHeight))
  );
  const startIndex = Math.max(0, firstVisibleIndex - safeOverscanRows);
  const endIndex = Math.min(rows.length, firstVisibleIndex + visibleRowCount + safeOverscanRows);

  return {
    rows: rows.slice(startIndex, endIndex),
    startIndex,
    endIndex,
    topSpacerHeight: startIndex * safeRowHeight,
    bottomSpacerHeight: Math.max(0, (rows.length - endIndex) * safeRowHeight),
    totalHeight: rows.length * safeRowHeight
  };
}

export function scrollTopForSourceTreeReveal(
  rowIndex: number,
  currentScrollTop: number,
  viewportHeight: number,
  rowHeight: number
): number {
  if (rowIndex < 0) return currentScrollTop;

  const safeRowHeight = Math.max(1, rowHeight);
  const safeViewportHeight = Math.max(safeRowHeight, viewportHeight);
  const safeScrollTop = Math.max(0, currentScrollTop);
  const rowTop = rowIndex * safeRowHeight;
  const rowBottom = rowTop + safeRowHeight;
  const viewportBottom = safeScrollTop + safeViewportHeight;

  if (rowTop < safeScrollTop) return rowTop;
  if (rowBottom > viewportBottom) return rowBottom - safeViewportHeight;
  return safeScrollTop;
}

type SourceTreeEntry = {
  components: string[];
  record: SourceRecord;
};

function buildTreeNodes(entries: SourceTreeEntry[], prefix: string[]): SourceTreeNode[] {
  const folderEntries = new Map<string, SourceTreeEntry[]>();
  const fileEntries: SourceTreeEntry[] = [];

  for (const entry of entries) {
    const firstComponent = entry.components[0];
    if (!firstComponent) continue;

    if (entry.components.length === 1) {
      fileEntries.push(entry);
      continue;
    }

    const existing = folderEntries.get(firstComponent) ?? [];
    existing.push({
      components: entry.components.slice(1),
      record: entry.record
    });
    folderEntries.set(firstComponent, existing);
  }

  const folderNodes = [...folderEntries.keys()]
    .sort(localizedAscending)
    .map((folderName) => {
      const relativePath = [...prefix, folderName].join('/');
      return {
        id: `folder:${relativePath}`,
        name: folderName,
        relativePath,
        file: null,
        children: buildTreeNodes(folderEntries.get(folderName) ?? [], [...prefix, folderName])
      };
    });

  const fileNodes = fileEntries
    .sort((left, right) => localizedAscending(left.record.fileName, right.record.fileName))
    .map((entry) => ({
      id: `file:${entry.record.path}`,
      name: entry.record.fileName,
      relativePath: entry.record.relativePath,
      file: entry.record,
      children: []
    }));

  return [...folderNodes, ...fileNodes];
}

function localizedAscending(left: string, right: string): number {
  return left.localeCompare(right, undefined, { sensitivity: 'base', numeric: true });
}

function scoreSourceRecord(record: SourceRecord, normalizedQuery: string): number | null {
  const fileName = record.fileName.toLowerCase();
  const relativePath = record.relativePath.toLowerCase();
  const language = record.language.toLowerCase();

  if (fileName === normalizedQuery) return 0;
  if (fileName.startsWith(normalizedQuery)) return 1;
  if (fileName.includes(normalizedQuery)) return 2;
  if (relativePath.startsWith(normalizedQuery)) return 3;
  if (relativePath.includes(normalizedQuery)) return 4;
  if (language.includes(normalizedQuery)) return 5;
  return null;
}

function compactSourceLineExcerpt(line: string): string {
  const trimmedLine = line.trim();
  if (trimmedLine.length <= 180) return trimmedLine;
  return `${trimmedLine.slice(0, 177)}...`;
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function normalizeTaskID(taskID: string | null | undefined): string | null {
  const normalized = String(taskID ?? '').trim().toUpperCase();
  return /^TSK-\d+$/.test(normalized) ? normalized : null;
}

function createProjectSourceRecord(
  record: SourceRecord,
  project: ProjectRoot,
  openedAt: number
): SourceRecentRecord {
  return {
    ...record,
    projectID: project.id,
    projectName: project.name,
    openedAt
  };
}
