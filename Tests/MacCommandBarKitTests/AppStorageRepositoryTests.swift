import Foundation
import XCTest
@testable import MacCommandBarKit

final class AppStorageRepositoryTests: XCTestCase {
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
