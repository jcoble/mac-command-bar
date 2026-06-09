export type SourceLanguage = 'csharp' | 'swift' | 'rust';

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
  return {
    ...record,
    content,
    lineCount: content.split(/\r\n|\r|\n/).length
  };
}
