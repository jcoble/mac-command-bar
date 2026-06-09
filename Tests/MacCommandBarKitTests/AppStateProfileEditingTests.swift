import XCTest
@testable import MacCommandBarKit

@MainActor
final class AppStateProfileEditingTests: XCTestCase {
    func testSaveNewProfileDraftAddsProfileAndPersistsSnapshot() throws {
        let persistence = MemoryAppPersistence()
        let state = AppState.profileEditingTestState(
            profiles: [],
            persistence: persistence
        )
        let draft = ProjectProfileDraft(
            name: "New Project",
            repoPath: "/Users/blackcolours/dev/work/new-project",
            worktreeRootsText: "/Users/blackcolours/dev/work/worktrees/new-project",
            portsText: "5173, 5001",
            urlsText: "http://localhost:5173"
        )

        let didSave = state.saveProfileDraft(draft)

        XCTAssertTrue(didSave)
        XCTAssertEqual(state.profiles.count, 1)
        let profile = try XCTUnwrap(state.profiles.first)
        XCTAssertEqual(profile.name, "New Project")
        XCTAssertEqual(profile.repoPath, "/Users/blackcolours/dev/work/new-project")
        XCTAssertEqual(profile.worktreeRoots, ["/Users/blackcolours/dev/work/worktrees/new-project"])
        XCTAssertEqual(profile.ports, [5173, 5001])
        XCTAssertEqual(profile.urls, ["http://localhost:5173"])
        XCTAssertEqual(profile.commands.first?.label, "Open repo")
        XCTAssertEqual(state.modules.first(where: { $0.id == "projects" })?.count, 1)
        XCTAssertEqual(state.statusMessage, "Profile saved")
        XCTAssertEqual(try XCTUnwrap(persistence.savedSnapshots.last).profiles, state.profiles)
    }

    func testSaveExistingProfileDraftUpdatesProfileAndPreservesCommands() throws {
        let profileID = UUID(uuidString: "11111111-1111-1111-1111-111111111111")!
        let command = ProfileCommand(
            id: UUID(uuidString: "22222222-2222-2222-2222-222222222222")!,
            label: "Start web",
            command: "pnpm dev",
            target: .terminal
        )
        let existing = ProjectProfile(
            id: profileID,
            name: "Old",
            repoPath: "/old",
            worktreeRoots: ["/old/worktrees"],
            ports: [3000],
            urls: [],
            commands: [command]
        )
        let persistence = MemoryAppPersistence()
        let state = AppState.profileEditingTestState(
            profiles: [existing],
            persistence: persistence
        )
        var draft = ProjectProfileDraft(profile: existing)
        draft.name = "Updated"
        draft.repoPath = "/updated"
        draft.portsText = "5173"
        draft.urlsText = "https://example.test"

        let didSave = state.saveProfileDraft(draft)

        XCTAssertTrue(didSave)
        XCTAssertEqual(state.profiles.count, 1)
        let updated = try XCTUnwrap(state.profiles.first)
        XCTAssertEqual(updated.id, profileID)
        XCTAssertEqual(updated.name, "Updated")
        XCTAssertEqual(updated.repoPath, "/updated")
        XCTAssertEqual(updated.ports, [5173])
        XCTAssertEqual(updated.urls, ["https://example.test"])
        XCTAssertEqual(updated.commands, [command])
        XCTAssertEqual(try XCTUnwrap(persistence.savedSnapshots.last).profiles, state.profiles)
    }

    func testSaveProfileDraftRejectsInvalidPortsWithoutPersisting() {
        let persistence = MemoryAppPersistence()
        let state = AppState.profileEditingTestState(
            profiles: [],
            persistence: persistence
        )
        let draft = ProjectProfileDraft(
            name: "Bad Ports",
            repoPath: "/repo",
            portsText: "5173, nope"
        )

        let didSave = state.saveProfileDraft(draft)

        XCTAssertFalse(didSave)
        XCTAssertTrue(state.profiles.isEmpty)
        XCTAssertTrue(persistence.savedSnapshots.isEmpty)
        XCTAssertEqual(state.statusMessage, "Invalid ports")
    }

    func testDeleteProfileRemovesProfileAndPersistsSnapshot() throws {
        let profile = ProjectProfile(
            id: UUID(uuidString: "33333333-3333-3333-3333-333333333333")!,
            name: "Delete",
            repoPath: "/delete"
        )
        let persistence = MemoryAppPersistence()
        let state = AppState.profileEditingTestState(
            profiles: [profile],
            persistence: persistence
        )

        state.deleteProfile(profile.id)

        XCTAssertTrue(state.profiles.isEmpty)
        XCTAssertEqual(state.modules.first(where: { $0.id == "projects" })?.count, 0)
        XCTAssertEqual(state.statusMessage, "Profile deleted")
        XCTAssertEqual(try XCTUnwrap(persistence.savedSnapshots.last).profiles, [])
    }
}

private extension AppState {
    static func profileEditingTestState(
        profiles: [ProjectProfile],
        persistence: MemoryAppPersistence
    ) -> AppState {
        AppState(
            modules: [
                DashboardModule(id: "projects", title: "Projects", symbol: "rectangle.stack", count: profiles.count, status: "Profiles", accent: .blue)
            ],
            selectedModuleID: "projects",
            clipboardVault: ClipboardVaultModel(),
            profiles: profiles,
            statusMessage: "Ready",
            coreClient: nil,
            persistence: persistence
        )
    }
}

private final class MemoryAppPersistence: AppPersisting {
    private(set) var savedSnapshots: [AppPersistenceSnapshot] = []

    func load() throws -> AppPersistenceSnapshot? {
        nil
    }

    func save(_ snapshot: AppPersistenceSnapshot) throws {
        savedSnapshots.append(snapshot)
    }
}
