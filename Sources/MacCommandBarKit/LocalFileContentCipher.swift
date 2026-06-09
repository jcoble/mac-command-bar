import CryptoKit
import Foundation
import Security

public enum LocalFileContentCipherError: Error, Equatable {
    case invalidPayload
    case randomKeyFailed(OSStatus)
    case unsupportedAlgorithm(String)
}

public struct LocalFileContentCipher: ContentCipher {
    private static let algorithm = "AES.GCM.local-file.v1"

    private let keyURL: URL
    private let fileManager: FileManager

    public init(
        keyURL: URL = LocalFileContentCipher.defaultKeyURL(),
        fileManager: FileManager = .default
    ) {
        self.keyURL = keyURL
        self.fileManager = fileManager
    }

    public static func defaultKeyURL(fileManager: FileManager = .default) -> URL {
        let applicationSupport = fileManager.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        ).first ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")

        return applicationSupport
            .appendingPathComponent("MacCommandBar", isDirectory: true)
            .appendingPathComponent("content-key.bin")
    }

    public func encrypt(_ plaintext: Data) throws -> EncryptedContent {
        let key = SymmetricKey(data: try loadOrCreateKeyData())
        let sealedBox = try AES.GCM.seal(plaintext, using: key)
        guard let combined = sealedBox.combined else {
            throw LocalFileContentCipherError.invalidPayload
        }
        return EncryptedContent(
            algorithm: Self.algorithm,
            payload: combined.base64EncodedString()
        )
    }

    public func decrypt(_ content: EncryptedContent) throws -> Data {
        guard content.algorithm == Self.algorithm else {
            throw LocalFileContentCipherError.unsupportedAlgorithm(content.algorithm)
        }
        guard let combined = Data(base64Encoded: content.payload) else {
            throw LocalFileContentCipherError.invalidPayload
        }

        let key = SymmetricKey(data: try loadOrCreateKeyData())
        let sealedBox = try AES.GCM.SealedBox(combined: combined)
        return try AES.GCM.open(sealedBox, using: key)
    }

    private func loadOrCreateKeyData() throws -> Data {
        if fileManager.fileExists(atPath: keyURL.path) {
            return try Data(contentsOf: keyURL)
        }

        var bytes = [UInt8](repeating: 0, count: 32)
        let status = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
        guard status == errSecSuccess else {
            throw LocalFileContentCipherError.randomKeyFailed(status)
        }

        let data = Data(bytes)
        try fileManager.createDirectory(
            at: keyURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try data.write(to: keyURL, options: [.atomic])
        try fileManager.setAttributes([.posixPermissions: 0o600], ofItemAtPath: keyURL.path)
        return data
    }
}
