import Foundation
import XCTest
@testable import MacCommandBarKit

final class AppStorageRepositoryTests: XCTestCase {
    func testDefaultCipherUsesLocalFileCipherUnlessKeychainIsExplicitlyRequested() {
        XCTAssertTrue(AppStorageRepository.defaultCipher(environment: [:]) is LocalFileContentCipher)
        XCTAssertTrue(
            AppStorageRepository.defaultCipher(environment: ["MCB_USE_KEYCHAIN_CIPHER": "1"])
                is KeychainContentCipher
        )
    }

    func testMissingStateFileLoadsNoSnapshot() throws {
        let directory = try temporaryDirectory()
        let repository = AppStorageRepository(
            stateURL: directory.appendingPathComponent("state.json"),
            cipher: ReversingContentCipher()
        )

        XCTAssertNil(try repository.load())
    }

    func testRepositoryPersistsProfilesAndEncryptedClipboardContent() throws {
        let directory = try temporaryDirectory()
        let stateURL = directory.appendingPathComponent("state.json")
        let repository = AppStorageRepository(
            stateURL: stateURL,
            cipher: ReversingContentCipher()
        )
        let profile = ProjectProfile(
            id: UUID(uuidString: "11111111-1111-1111-1111-111111111111")!,
            name: "MacCommandBar",
            repoPath: "/Users/blackcolours/dev/work/mac-command-bar",
            worktreeRoots: ["/Users/blackcolours/dev/work/worktrees/mac-command-bar"],
            ports: [4242],
            urls: ["http://localhost:4242"],
            commands: [
                ProfileCommand(
                    id: UUID(uuidString: "22222222-2222-2222-2222-222222222222")!,
                    label: "Open repo",
                    command: "cd /Users/blackcolours/dev/work/mac-command-bar",
                    target: .terminal
                )
            ]
        )
        let secretText = "api_key=super-secret-value"
        var vault = ClipboardVaultModel()
        vault.addText(secretText, tags: ["credential"])
        let savedItemID = try XCTUnwrap(vault.items.first?.id)
        vault.togglePinned(savedItemID)

        try repository.save(AppPersistenceSnapshot(profiles: [profile], clipboardVault: vault))

        let rawState = try String(contentsOf: stateURL, encoding: .utf8)
        XCTAssertFalse(rawState.contains(secretText))

        let loaded = try XCTUnwrap(repository.load())
        XCTAssertEqual(loaded.profiles, [profile])
        XCTAssertEqual(loaded.clipboardVault.items.count, 1)

        let loadedItem = try XCTUnwrap(loaded.clipboardVault.items.first)
        XCTAssertEqual(loadedItem.id, savedItemID)
        XCTAssertEqual(loadedItem.preview, ClipboardVaultModel.secretPreview)
        XCTAssertEqual(loadedItem.content, secretText)
        XCTAssertEqual(loadedItem.tags, ["credential"])
        XCTAssertTrue(loadedItem.isPinned)
        XCTAssertTrue(loadedItem.isSecret)
    }

    func testRepositoryLoadsProfilesAndUnavailableClipboardItemWhenDecryptFails() throws {
        let directory = try temporaryDirectory()
        let stateURL = directory.appendingPathComponent("state.json")
        let profileID = UUID(uuidString: "11111111-1111-1111-1111-111111111111")!
        let itemID = UUID(uuidString: "33333333-3333-3333-3333-333333333333")!
        let rawState = """
        {
          "version" : 1,
          "profiles" : [
            {
              "id" : "\(profileID.uuidString)",
              "name" : "MacCommandBar",
              "repoPath" : "/repo",
              "worktreeRoots" : [],
              "ports" : [],
              "urls" : [],
              "commands" : []
            }
          ],
          "clipboardItems" : [
            {
              "id" : "\(itemID.uuidString)",
              "kind" : "text",
              "preview" : "Secret text saved",
              "tags" : ["credential"],
              "isPinned" : true,
              "isSecret" : true,
              "createdAt" : "2026-06-08T00:00:00Z",
              "encryptedContent" : {
                "algorithm" : "test-reverse",
                "payload" : "not decryptable"
              }
            }
          ]
        }
        """
        try FileManager.default.createDirectory(
            at: directory,
            withIntermediateDirectories: true
        )
        try rawState.data(using: .utf8)?.write(to: stateURL)
        let repository = AppStorageRepository(
            stateURL: stateURL,
            cipher: ThrowingContentCipher()
        )

        let loaded = try XCTUnwrap(repository.load())

        XCTAssertEqual(loaded.profiles.map(\.id), [profileID])
        let item = try XCTUnwrap(loaded.clipboardVault.items.first)
        XCTAssertEqual(item.id, itemID)
        XCTAssertEqual(item.preview, "Secret text saved")
        XCTAssertNil(item.content)
        XCTAssertEqual(item.tags, ["credential"])
        XCTAssertTrue(item.isPinned)
        XCTAssertTrue(item.isSecret)
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

private struct ReversingContentCipher: ContentCipher {
    func encrypt(_ plaintext: Data) throws -> EncryptedContent {
        EncryptedContent(
            algorithm: "test-reverse",
            payload: Data(plaintext.reversed()).base64EncodedString()
        )
    }

    func decrypt(_ content: EncryptedContent) throws -> Data {
        let data = Data(base64Encoded: content.payload) ?? Data()
        return Data(data.reversed())
    }
}

private struct ThrowingContentCipher: ContentCipher {
    func encrypt(_ plaintext: Data) throws -> EncryptedContent {
        EncryptedContent(algorithm: "throwing", payload: "")
    }

    func decrypt(_ content: EncryptedContent) throws -> Data {
        throw AppStorageError.invalidEncryptedText
    }
}
