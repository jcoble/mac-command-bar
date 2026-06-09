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

    func testOpenProjectURLUsesURLOpenerAndUpdatesStatus() throws {
        let opener = RecordingURLOpener()
        let state = AppState(
            modules: [],
            selectedModuleID: "projects",
            clipboardVault: ClipboardVaultModel(),
            profiles: [],
            statusMessage: "Ready",
            coreClient: nil,
            urlOpener: opener
        )

        state.openProjectURL("http://localhost:5173")

        XCTAssertEqual(opener.openedURLs, [URL(string: "http://localhost:5173")!])
        XCTAssertEqual(state.statusMessage, "Opened localhost")
    }

    func testOpenProjectURLRejectsInvalidURLWithoutCallingOpener() {
        let opener = RecordingURLOpener()
        let state = AppState(
            modules: [],
            selectedModuleID: "projects",
            clipboardVault: ClipboardVaultModel(),
            profiles: [],
            statusMessage: "Ready",
            coreClient: nil,
            urlOpener: opener
        )

        state.openProjectURL("localhost:5173")

        XCTAssertEqual(opener.openedURLs, [])
        XCTAssertEqual(state.statusMessage, "Invalid URL")
    }
}

private final class RecordingCommandLauncher: CommandLaunching {
    private(set) var launches: [CommandLaunch] = []

    func launch(_ command: String, target: ProfileLaunchTarget) throws {
        launches.append(CommandLaunch(command: command, target: target))
    }
}

private final class RecordingURLOpener: URLOpening {
    private(set) var openedURLs: [URL] = []

    func open(_ url: URL) throws {
        openedURLs.append(url)
    }
}
