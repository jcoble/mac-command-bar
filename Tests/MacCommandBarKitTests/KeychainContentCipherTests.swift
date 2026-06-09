import Foundation
import Security
import XCTest
@testable import MacCommandBarKit

final class KeychainContentCipherTests: XCTestCase {
    func testKeychainCipherRoundTripsEncryptedContent() throws {
        let service = "dev.blackcolours.MacCommandBar.tests.\(UUID().uuidString)"
        let account = "clipboard-content-key"
        defer {
            SecItemDelete(keyQuery(service: service, account: account) as CFDictionary)
        }

        let cipher = KeychainContentCipher(service: service, account: account)
        let plaintext = Data("token=secret-value".utf8)

        let encrypted = try cipher.encrypt(plaintext)

        XCTAssertEqual(encrypted.algorithm, "AES.GCM.v1")
        XCTAssertNotEqual(encrypted.payload, plaintext.base64EncodedString())
        XCTAssertEqual(try cipher.decrypt(encrypted), plaintext)
    }
}

private func keyQuery(service: String, account: String) -> [String: Any] {
    [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account
    ]
}
