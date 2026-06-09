import Foundation

public struct ClipboardVaultModel: Equatable, Sendable {
    public static let secretPreview = "Secret text saved"

    public private(set) var items: [ClipboardItem]

    public init(items: [ClipboardItem] = []) {
        self.items = items.sortedForDisplay()
    }

    public func search(_ query: String) -> [ClipboardItem] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return items.sortedForDisplay()
        }

        return items
            .filter { item in
                item.preview.localizedCaseInsensitiveContains(trimmed)
                    || item.content?.localizedCaseInsensitiveContains(trimmed) == true
                    || item.tags.contains { $0.localizedCaseInsensitiveContains(trimmed) }
            }
            .sortedForDisplay()
    }

    public mutating func togglePinned(_ id: UUID) {
        guard let index = items.firstIndex(where: { $0.id == id }) else {
            return
        }
        items[index].isPinned.toggle()
        items = items.sortedForDisplay()
    }

    @discardableResult
    public mutating func delete(_ id: UUID) -> Bool {
        let beforeCount = items.count
        items.removeAll { $0.id == id }
        return items.count != beforeCount
    }

    public mutating func addText(_ text: String, tags: [String] = []) {
        let normalized = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else {
            return
        }

        let isSecret = SecretDetector.containsSecret(normalized)
        let preview = isSecret ? Self.secretPreview : String(normalized.prefix(180))
        let item = ClipboardItem(
            kind: normalized.looksLikeURL ? .url : .text,
            preview: preview,
            content: normalized,
            tags: tags,
            isPinned: false,
            isSecret: isSecret
        )
        items.insert(item, at: 0)
        items = Array(items.uniquedByContentOrPreview().prefix(200)).sortedForDisplay()
    }
}

public enum SecretDetector {
    public static func containsSecret(_ text: String) -> Bool {
        let lowered = text.lowercased()
        return lowered.contains("password=")
            || lowered.contains("api_key")
            || lowered.contains("secret")
            || lowered.contains("token=")
            || text.contains("sk-")
            || text.contains("ghp_")
            || text.contains("xoxb-")
    }
}

private extension String {
    var looksLikeURL: Bool {
        lowercased().hasPrefix("http://") || lowercased().hasPrefix("https://")
    }
}

private extension Array where Element == ClipboardItem {
    func sortedForDisplay() -> [ClipboardItem] {
        sorted {
            if $0.isPinned != $1.isPinned {
                return $0.isPinned && !$1.isPinned
            }
            return $0.createdAt > $1.createdAt
        }
    }

    func uniquedByContentOrPreview() -> [ClipboardItem] {
        var seen = Set<String>()
        var result: [ClipboardItem] = []
        for item in self {
            let key = item.content ?? item.preview
            guard !seen.contains(key) else {
                continue
            }
            seen.insert(key)
            result.append(item)
        }
        return result
    }
}
