import XCTest
@testable import MacCommandBarKit

@MainActor
final class AppStateRefreshTests: XCTestCase {
    func testRefreshSessionsOnlyCallsSessionScanner() async throws {
        let spy = CoreClientSpy(responseData: [
            "count": .number(1),
            "sessions": .array([
                .object([
                    "provider": .string("codex"),
                    "id": .string("019d"),
                    "title": .string("Fix runtime"),
                    "projectPath": .string("/repo"),
                    "lastActivity": .string("2026-06-08T22:00:00Z"),
                    "resumeCommands": .array([.string("codex resume 019d")])
                ])
            ])
        ])
        let state = AppState.testState(coreClient: spy)

        await state.refreshSessions()

        let actions = await spy.recordedActions()
        XCTAssertEqual(actions, [.scanSessions])
        XCTAssertEqual(state.sessions.count, 1)
        XCTAssertEqual(state.modules.first(where: { $0.id == "commands" })?.count, 1)
        XCTAssertEqual(state.worktrees.count, 0)
        XCTAssertEqual(state.processes.count, 0)
    }

    func testRefreshSnapshotsScansEachModuleWithoutBlockingCallersOnSend() async throws {
        let spy = CoreClientSpy(responseData: ["count": .number(0)])
        let state = AppState.testState(coreClient: spy)

        await state.refreshSnapshots()

        let actions = await spy.recordedActions()
        XCTAssertEqual(actions, [.scanProcesses, .scanSessions, .scanWorktrees])
    }
}

private actor CoreClientSpy: CoreSending {
    private(set) var actions: [CoreAction] = []
    private let responseData: [String: JSONValue]

    init(responseData: [String: JSONValue]) {
        self.responseData = responseData
    }

    func send(_ request: CoreRequest) async throws -> CoreResponse {
        actions.append(request.action)
        return CoreResponse(
            id: request.id,
            ok: true,
            summary: "ok",
            data: responseData,
            warnings: [],
            proposedCommand: nil
        )
    }

    func recordedActions() -> [CoreAction] {
        actions
    }
}

private extension AppState {
    static func testState(coreClient: any CoreSending) -> AppState {
        AppState(
            modules: [
                DashboardModule(id: "processes", title: "Processes", symbol: "cpu", count: 0, status: "Ready", accent: .orange),
                DashboardModule(id: "sessions", title: "Sessions", symbol: "terminal", count: 0, status: "Ready", accent: .green),
                DashboardModule(id: "worktrees", title: "Worktrees", symbol: "point.3.connected.trianglepath.dotted", count: 0, status: "Ready", accent: .slate),
                DashboardModule(id: "commands", title: "Commands", symbol: "command", count: 0, status: "Palette", accent: .slate)
            ],
            selectedModuleID: "sessions",
            clipboardVault: ClipboardVaultModel(),
            profiles: [
                ProjectProfile(name: "Test", repoPath: "/repo")
            ],
            statusMessage: "Ready",
            coreClient: coreClient
        )
    }
}
