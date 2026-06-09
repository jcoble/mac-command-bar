import Foundation

public struct AppPersistenceSnapshot: Equatable, Sendable {
    public var profiles: [ProjectProfile]
    public var clipboardVault: ClipboardVaultModel

    public init(profiles: [ProjectProfile], clipboardVault: ClipboardVaultModel) {
        self.profiles = profiles
        self.clipboardVault = clipboardVault
    }
}

public protocol AppPersisting {
    func load() throws -> AppPersistenceSnapshot?
    func save(_ snapshot: AppPersistenceSnapshot) throws
}

public struct EncryptedContent: Codable, Equatable, Sendable {
    public var algorithm: String
    public var payload: String

    public init(algorithm: String, payload: String) {
        self.algorithm = algorithm
        self.payload = payload
    }
}

public protocol ContentCipher {
    func encrypt(_ plaintext: Data) throws -> EncryptedContent
    func decrypt(_ content: EncryptedContent) throws -> Data
}

public enum AppStorageError: Error, Equatable {
    case invalidEncryptedText
}

public final class AppStorageRepository: AppPersisting {
    public let stateURL: URL
    private let cipher: any ContentCipher
    private let fileManager: FileManager

    public init(
        stateURL: URL = AppStorageRepository.defaultStateURL(),
        cipher: any ContentCipher = KeychainContentCipher(),
        fileManager: FileManager = .default
    ) {
        self.stateURL = stateURL
        self.cipher = cipher
        self.fileManager = fileManager
    }

    public static func defaultStateURL(fileManager: FileManager = .default) -> URL {
        let applicationSupport = fileManager.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        ).first ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")

        return applicationSupport
            .appendingPathComponent("MacCommandBar", isDirectory: true)
            .appendingPathComponent("state.json")
    }

    public func load() throws -> AppPersistenceSnapshot? {
        guard fileManager.fileExists(atPath: stateURL.path) else {
            return nil
        }

        let data = try Data(contentsOf: stateURL)
        let persisted = try decoder.decode(PersistedAppState.self, from: data)
        let items = try persisted.clipboardItems.map { try $0.clipboardItem(cipher: cipher) }
        return AppPersistenceSnapshot(
            profiles: persisted.profiles,
            clipboardVault: ClipboardVaultModel(items: items)
        )
    }

    public func save(_ snapshot: AppPersistenceSnapshot) throws {
        try fileManager.createDirectory(
            at: stateURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )

        let persisted = try PersistedAppState(
            profiles: snapshot.profiles,
            clipboardItems: snapshot.clipboardVault.items.map {
                try PersistedClipboardItem(item: $0, cipher: cipher)
            }
        )
        let data = try encoder.encode(persisted)
        try data.write(to: stateURL, options: [.atomic])
    }

    private var encoder: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        return encoder
    }

    private var decoder: JSONDecoder {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return decoder
    }
}

private struct PersistedAppState: Codable {
    var version: Int
    var profiles: [ProjectProfile]
    var clipboardItems: [PersistedClipboardItem]

    init(version: Int = 1, profiles: [ProjectProfile], clipboardItems: [PersistedClipboardItem]) {
        self.version = version
        self.profiles = profiles
        self.clipboardItems = clipboardItems
    }
}

private struct PersistedClipboardItem: Codable {
    var id: UUID
    var kind: ClipboardItemKind
    var preview: String
    var tags: [String]
    var isPinned: Bool
    var isSecret: Bool
    var createdAt: Date
    var encryptedContent: EncryptedContent?

    init(item: ClipboardItem, cipher: any ContentCipher) throws {
        self.id = item.id
        self.kind = item.kind
        self.preview = item.preview
        self.tags = item.tags
        self.isPinned = item.isPinned
        self.isSecret = item.isSecret
        self.createdAt = item.createdAt
        if let content = item.content {
            self.encryptedContent = try cipher.encrypt(Data(content.utf8))
        } else {
            self.encryptedContent = nil
        }
    }

    func clipboardItem(cipher: any ContentCipher) throws -> ClipboardItem {
        let content: String?
        if let encryptedContent {
            let data = try cipher.decrypt(encryptedContent)
            guard let text = String(data: data, encoding: .utf8) else {
                throw AppStorageError.invalidEncryptedText
            }
            content = text
        } else {
            content = nil
        }

        return ClipboardItem(
            id: id,
            kind: kind,
            preview: preview,
            content: content,
            tags: tags,
            isPinned: isPinned,
            isSecret: isSecret,
            createdAt: createdAt
        )
    }
}
