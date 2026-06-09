import XCTest
@testable import MacCommandBarKit

final class ClipboardVaultTests: XCTestCase {
    func testClipboardVaultSearchesPinsAndSecretFlags() {
        let plain = ClipboardItem(
            id: UUID(uuidString: "11111111-1111-1111-1111-111111111111")!,
            kind: .text,
            preview: "codex resume 019d",
            tags: ["agent"],
            isPinned: false,
            isSecret: false,
            createdAt: Date(timeIntervalSince1970: 1)
        )
        let secret = ClipboardItem(
            id: UUID(uuidString: "22222222-2222-2222-2222-222222222222")!,
            kind: .text,
            preview: "sk-live-1234567890",
            tags: [],
            isPinned: false,
            isSecret: true,
            createdAt: Date(timeIntervalSince1970: 2)
        )

        var vault = ClipboardVaultModel(items: [plain, secret])

        XCTAssertEqual(vault.search("codex").map(\.id), [plain.id])
        vault.togglePinned(plain.id)

        XCTAssertTrue(vault.items.first(where: { $0.id == plain.id })?.isPinned == true)
        XCTAssertTrue(vault.items.first(where: { $0.id == secret.id })?.isSecret == true)
    }

    func testDistinctSecretsDoNotCollapseToSharedRedactedPreview() {
        var vault = ClipboardVaultModel()

        vault.addText("api_key=first-secret")
        vault.addText("api_key=second-secret")

        XCTAssertEqual(vault.items.count, 2)
        XCTAssertEqual(vault.items.map(\.preview), [
            ClipboardVaultModel.secretPreview,
            ClipboardVaultModel.secretPreview
        ])
    }
}
