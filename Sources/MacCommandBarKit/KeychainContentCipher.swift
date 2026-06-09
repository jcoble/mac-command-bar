import CryptoKit
import Foundation
import Security

public enum KeychainContentCipherError: Error, Equatable {
    case invalidPayload
    case keychainReadFailed(OSStatus)
    case keychainWriteFailed(OSStatus)
    case unsupportedAlgorithm(String)
}

public struct KeychainContentCipher: ContentCipher {
    private let service: String
    private let account: String

    public init(
        service: String = "dev.blackcolours.MacCommandBar",
        account: String = "clipboard-content-key"
    ) {
        self.service = service
        self.account = account
    }

    public func encrypt(_ plaintext: Data) throws -> EncryptedContent {
        let key = SymmetricKey(data: try loadOrCreateKeyData())
        let sealedBox = try AES.GCM.seal(plaintext, using: key)
        guard let combined = sealedBox.combined else {
            throw KeychainContentCipherError.invalidPayload
        }
        return EncryptedContent(
            algorithm: "AES.GCM.v1",
            payload: combined.base64EncodedString()
        )
    }

    public func decrypt(_ content: EncryptedContent) throws -> Data {
        guard content.algorithm == "AES.GCM.v1" else {
            throw KeychainContentCipherError.unsupportedAlgorithm(content.algorithm)
        }
        guard let combined = Data(base64Encoded: content.payload) else {
            throw KeychainContentCipherError.invalidPayload
        }

        let key = SymmetricKey(data: try loadOrCreateKeyData())
        let sealedBox = try AES.GCM.SealedBox(combined: combined)
        return try AES.GCM.open(sealedBox, using: key)
    }

    private func loadOrCreateKeyData() throws -> Data {
        if let existing = try loadKeyData() {
            return existing
        }

        var bytes = [UInt8](repeating: 0, count: 32)
        let status = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
        guard status == errSecSuccess else {
            throw KeychainContentCipherError.keychainReadFailed(status)
        }

        let data = Data(bytes)
        try saveKeyData(data)
        return data
    }

    private func loadKeyData() throws -> Data? {
        var result: AnyObject?
        let status = SecItemCopyMatching(keyQuery(returnData: true) as CFDictionary, &result)
        if status == errSecItemNotFound {
            return nil
        }
        guard status == errSecSuccess else {
            throw KeychainContentCipherError.keychainReadFailed(status)
        }
        return result as? Data
    }

    private func saveKeyData(_ data: Data) throws {
        var query = keyQuery(returnData: false)
        query[kSecValueData as String] = data
        query[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly

        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainContentCipherError.keychainWriteFailed(status)
        }
    }

    private func keyQuery(returnData: Bool) -> [String: Any] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]
        if returnData {
            query[kSecReturnData as String] = true
            query[kSecMatchLimit as String] = kSecMatchLimitOne
        }
        return query
    }
}
