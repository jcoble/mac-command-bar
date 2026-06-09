import XCTest
@testable import MacCommandBarKit

@MainActor
final class AppStatePersistenceTests: XCTestCase {
    func testBootstrapLoadsPersistedProfilesAndClipboardVault() {
        let profile = ProjectProfile(
            id: UUID(uuidString: "11111111-1111-1111-1111-111111111111")!,
            name: "Stored Project",
            repoPath: "/stored/project"
        )
        let item = ClipboardItem(
            id: UUID(uuidString: "22222222-2222-2222-2222-222222222222")!,
            kind: .text,
            preview: "stored note",
            content: "stored note body",
            tags: ["note"],
            isPinned: true,
            isSecret: false,
            createdAt: Date(timeIntervalSince1970: 100)
        )
        let persistence = MemoryAppPersistence(
            loadedSnapshot: AppPersistenceSnapshot(
                profiles: [profile],
                clipboardVault: ClipboardVaultModel(items: [item])
            )
        )

        let state = AppState.bootstrap(coreClient: nil, persistence: persistence)

        XCTAssertEqual(state.profiles, [profile])
        XCTAssertEqual(state.clipboardVault.items, [item])
        XCTAssertEqual(state.modules.first(where: { $0.id == "projects" })?.count, 1)
        XCTAssertEqual(state.modules.first(where: { $0.id == "clipboard" })?.count, 1)
    }

    func testPinnedClipboardChangesArePersisted() throws {
        let item = ClipboardItem(
            id: UUID(uuidString: "33333333-3333-3333-3333-333333333333")!,
            kind: .text,
            preview: "clip",
            content: "clip body",
            tags: [],
            isPinned: false,
            isSecret: false,
            createdAt: Date(timeIntervalSince1970: 200)
        )
        let persistence = MemoryAppPersistence()
        let state = AppState(
            modules: [
                DashboardModule(id: "clipboard", title: "Clipboard", symbol: "doc.on.clipboard", count: 1, status: "Local", accent: .blue)
            ],
            selectedModuleID: "clipboard",
            clipboardVault: ClipboardVaultModel(items: [item]),
            profiles: [],
            statusMessage: "Ready",
            coreClient: nil,
            persistence: persistence
        )

        state.togglePinnedClipboardItem(item.id)

        let savedSnapshot = try XCTUnwrap(persistence.savedSnapshots.last)
        XCTAssertTrue(savedSnapshot.clipboardVault.items.first?.isPinned == true)
    }
}

private final class MemoryAppPersistence: AppPersisting {
    private let loadedSnapshot: AppPersistenceSnapshot?
    private(set) var savedSnapshots: [AppPersistenceSnapshot]

    init(loadedSnapshot: AppPersistenceSnapshot? = nil) {
        self.loadedSnapshot = loadedSnapshot
        self.savedSnapshots = []
    }

    func load() throws -> AppPersistenceSnapshot? {
        loadedSnapshot
    }

    func save(_ snapshot: AppPersistenceSnapshot) throws {
        savedSnapshots.append(snapshot)
    }
}
