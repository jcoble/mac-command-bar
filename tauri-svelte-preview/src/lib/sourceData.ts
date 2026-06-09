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

export type SourceRecentRecord = SourceRecord & {
  projectID: string;
  projectName: string;
  openedAt: number;
};

export type SourceOpenTab = SourceRecentRecord;

export type CloseOpenSourceTabResult = {
  tabs: SourceOpenTab[];
  nextActivePath: string | null;
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
  }
];

const demoContentByPath = new Map<string, string>([
  [sourceRecords[0].path, formatResolverContent],
  [sourceRecords[1].path, formatDetectorContent],
  [sourceRecords[2].path, coreClientContent]
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
