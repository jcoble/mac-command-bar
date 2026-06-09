import XCTest
@testable import MacCommandBarKit

@MainActor
final class AppStateCommandPaletteTests: XCTestCase {
    func testCommandPaletteIncludesProfileCommandsAndSessionResumes() {
        let profile = ProjectProfile(
            name: "EdiPlatform",
            repoPath: "/repo",
            commands: [
                ProfileCommand(label: "Start web", command: "pnpm dev", target: .terminal)
            ]
        )
        let session = AgentSessionRecord(
            provider: .codex,
            id: "019d",
            title: "Fix scanner",
            projectPath: "/repo",
            lastActivity: nil,
            resumeCommands: ["codex resume 019d"]
        )
        let state = AppState.commandPaletteTestState(profiles: [profile], sessions: [session])

        let items = state.commandPaletteItems(matching: "")

        XCTAssertEqual(items.map(\.title), ["Start web", "Resume Fix scanner"])
        XCTAssertEqual(items.map(\.command), ["pnpm dev", "codex resume 019d"])
        XCTAssertEqual(items.map(\.source), ["EdiPlatform", "Codex"])
    }

    func testCommandPaletteSearchFiltersTitleSourceAndCommand() {
        let profile = ProjectProfile(
            name: "Rental Command",
            repoPath: "/rental",
            commands: [
                ProfileCommand(label: "Open repo", command: "cd /rental", target: .terminal),
                ProfileCommand(label: "Start API", command: "dotnet run", target: .terminal)
            ]
        )
        let state = AppState.commandPaletteTestState(profiles: [profile])

        XCTAssertEqual(state.commandPaletteItems(matching: "api").map(\.title), ["Start API"])
        XCTAssertEqual(state.commandPaletteItems(matching: "rental").map(\.title), ["Open repo", "Start API"])
        XCTAssertEqual(state.commandPaletteItems(matching: "dotnet").map(\.title), ["Start API"])
    }

    func testLaunchCommandPaletteItemUsesLauncherAndUpdatesStatus() throws {
        let launcher = RecordingCommandLauncher()
        let state = AppState.commandPaletteTestState(commandLauncher: launcher)
        let item = CommandPaletteItem(
            id: "test",
            title: "Start web",
            source: "EdiPlatform",
            command: "pnpm dev",
            target: .terminal
        )

        state.launchCommandPaletteItem(item)

        XCTAssertEqual(launcher.launches, [
            CommandLaunch(command: "pnpm dev", target: .terminal)
        ])
        XCTAssertEqual(state.statusMessage, "Launched Start web")
    }
}

private extension AppState {
    static func commandPaletteTestState(
        profiles: [ProjectProfile] = [],
        sessions: [AgentSessionRecord] = [],
        commandLauncher: any CommandLaunching = RecordingCommandLauncher()
    ) -> AppState {
        let state = AppState(
            modules: [],
            selectedModuleID: "commands",
            clipboardVault: ClipboardVaultModel(),
            profiles: profiles,
            statusMessage: "Ready",
            coreClient: nil,
            commandLauncher: commandLauncher
        )
        state.sessions = sessions
        return state
    }
}

private final class RecordingCommandLauncher: CommandLaunching {
    private(set) var launches: [CommandLaunch] = []

    func launch(_ command: String, target: ProfileLaunchTarget) throws {
        launches.append(CommandLaunch(command: command, target: target))
    }
}
