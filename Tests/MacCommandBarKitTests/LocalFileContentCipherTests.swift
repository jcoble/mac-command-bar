import Foundation
import XCTest
@testable import MacCommandBarKit

final class LocalFileContentCipherTests: XCTestCase {
    func testLocalFileCipherCreatesReusableKeyWithoutKeychain() throws {
        let directory = try temporaryDirectory()
        let keyURL = directory.appendingPathComponent("clipboard.key")
        let cipher = LocalFileContentCipher(keyURL: keyURL)
        let plaintext = Data("token=secret-value".utf8)

        let encrypted = try cipher.encrypt(plaintext)

        XCTAssertEqual(encrypted.algorithm, "AES.GCM.local-file.v1")
        XCTAssertNotEqual(encrypted.payload, plaintext.base64EncodedString())
        XCTAssertTrue(FileManager.default.fileExists(atPath: keyURL.path))

        let secondCipher = LocalFileContentCipher(keyURL: keyURL)
        XCTAssertEqual(try secondCipher.decrypt(encrypted), plaintext)
    }

    func testLocalFileCipherCreatesPrivateKeyFile() throws {
        let directory = try temporaryDirectory()
        let keyURL = directory.appendingPathComponent("clipboard.key")
        let cipher = LocalFileContentCipher(keyURL: keyURL)

        _ = try cipher.encrypt(Data("secret".utf8))

        let attributes = try FileManager.default.attributesOfItem(atPath: keyURL.path)
        XCTAssertEqual(attributes[.posixPermissions] as? Int, 0o600)
    }

    func testAppStorageUsesLocalFileCipherByDefaultUnlessKeychainIsExplicitlyEnabled() {
        XCTAssertTrue(AppStorageRepository.defaultCipher(environment: [:]) is LocalFileContentCipher)
        XCTAssertTrue(
            AppStorageRepository.defaultCipher(environment: ["MCB_USE_KEYCHAIN_CIPHER": "1"])
                is KeychainContentCipher
        )
    }

    private func temporaryDirectory() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("MacCommandBarTests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        addTeardownBlock {
            try? FileManager.default.removeItem(at: url)
        }
        return url
    }
}
