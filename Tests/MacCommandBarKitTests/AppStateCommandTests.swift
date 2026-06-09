import XCTest
@testable import MacCommandBarKit

@MainActor
final class AppStateCommandTests: XCTestCase {
    func testLaunchProfileCommandUsesLauncherAndUpdatesStatus() throws {
        let launcher = RecordingCommandLauncher()
        let state = AppState(
            modules: [],
            selectedModuleID: "projects",
            clipboardVault: ClipboardVaultModel(),
            profiles: [],
            statusMessage: "Ready",
            coreClient: nil,
            commandLauncher: launcher
        )
        let command = ProfileCommand(
            id: UUID(uuidString: "11111111-1111-1111-1111-111111111111")!,
            label: "Start web",
            command: "pnpm dev",
            target: .terminal
        )

        state.launchProfileCommand(command)

        XCTAssertEqual(launcher.launches, [
            CommandLaunch(command: "pnpm dev", target: .terminal)
        ])
        XCTAssertEqual(state.statusMessage, "Launched Start web")
    }

    func testLaunchTerminalCommandUsesTerminalTarget() throws {
        let launcher = RecordingCommandLauncher()
        let state = AppState(
            modules: [],
            selectedModuleID: "sessions",
            clipboardVault: ClipboardVaultModel(),
            profiles: [],
            statusMessage: "Ready",
            coreClient: nil,
            commandLauncher: launcher
        )

        state.launchTerminalCommand("codex resume 019d", label: "Resume")

        XCTAssertEqual(launcher.launches, [
            CommandLaunch(command: "codex resume 019d", target: .terminal)
        ])
        XCTAssertEqual(state.statusMessage, "Launched Resume")
    }
}

private final class RecordingCommandLauncher: CommandLaunching {
    private(set) var launches: [CommandLaunch] = []

    func launch(_ command: String, target: ProfileLaunchTarget) throws {
        launches.append(CommandLaunch(command: command, target: target))
    }
}
