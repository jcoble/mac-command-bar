import AppKit
import Combine
import Foundation

public protocol ClipboardWriting {
    func writeString(_ value: String)
}

public struct SystemClipboardWriter: ClipboardWriting {
    public init() {}

    public func writeString(_ value: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(value, forType: .string)
    }
}

@MainActor
public final class AppState: ObservableObject {
    @Published public var modules: [DashboardModule]
    @Published public var selectedModuleID: String
    @Published public var clipboardVault: ClipboardVaultModel
    @Published public var profiles: [ProjectProfile]
    @Published public var processes: [ProcessRecord]
    @Published public var worktrees: [WorktreeRecord]
    @Published public var sessions: [AgentSessionRecord]
    @Published public var pendingAction: ConfirmableAction?
    @Published public var statusMessage: String
    @Published public var searchText: String
    @Published public var lastCoreWarning: String?

    private var coreClient: (any CoreSending)?
    private var persistence: (any AppPersisting)?
    private var clipboardWriter: any ClipboardWriting

    public init(
        modules: [DashboardModule],
        selectedModuleID: String,
        clipboardVault: ClipboardVaultModel,
        profiles: [ProjectProfile],
        statusMessage: String,
        coreClient: (any CoreSending)?,
        persistence: (any AppPersisting)? = nil,
        clipboardWriter: any ClipboardWriting = SystemClipboardWriter()
    ) {
        self.modules = modules
        self.selectedModuleID = selectedModuleID
        self.clipboardVault = clipboardVault
        self.profiles = profiles
        self.processes = []
        self.worktrees = []
        self.sessions = []
        self.pendingAction = nil
        self.statusMessage = statusMessage
        self.searchText = ""
        self.coreClient = coreClient
        self.persistence = persistence
        self.clipboardWriter = clipboardWriter
    }

    public static func bootstrap() -> AppState {
        let client = try? CoreClient.discover()
        return bootstrap(coreClient: client, persistence: AppStorageRepository())
    }

    public static func bootstrap(
        coreClient: (any CoreSending)?,
        persistence: (any AppPersisting)?
    ) -> AppState {
        var profiles = ProfileStore.defaultProfiles()
        var clipboardVault = ClipboardVaultModel()
        var statusMessage = coreClient == nil ? "Core helper not built yet" : "Ready"

        if let persistence {
            do {
                if let snapshot = try persistence.load() {
                    profiles = snapshot.profiles
                    clipboardVault = snapshot.clipboardVault
                } else {
                    try persistence.save(
                        AppPersistenceSnapshot(
                            profiles: profiles,
                            clipboardVault: clipboardVault
                        )
                    )
                }
            } catch {
                statusMessage = "Storage unavailable: \(error.localizedDescription)"
            }
        }

        return AppState(
            modules: defaultModules(
                profileCount: profiles.count,
                clipboardCount: clipboardVault.items.count
            ),
            selectedModuleID: "projects",
            clipboardVault: clipboardVault,
            profiles: profiles,
            statusMessage: statusMessage,
            coreClient: coreClient,
            persistence: persistence
        )
    }

    public var selectedModule: DashboardModule? {
        modules.first { $0.id == selectedModuleID }
    }

    public func captureClipboardText() {
        guard let text = NSPasteboard.general.string(forType: .string) else {
            statusMessage = "Clipboard has no text"
            return
        }

        clipboardVault.addText(text)
        updateModule("clipboard", count: clipboardVault.items.count, status: "Captured")
        statusMessage = "Clipboard text saved"
        persistState()
    }

    public func togglePinnedClipboardItem(_ id: UUID) {
        clipboardVault.togglePinned(id)
        persistState()
    }

    public func restoreClipboardItem(_ id: UUID) {
        guard let item = clipboardVault.items.first(where: { $0.id == id }) else {
            statusMessage = "Clipboard item missing"
            return
        }
        guard let content = item.content else {
            statusMessage = "Clipboard content unavailable"
            return
        }

        clipboardWriter.writeString(content)
        statusMessage = item.isSecret ? "Secret copied to clipboard" : "Copied to clipboard"
    }

    public func refreshSnapshots() async {
        guard coreClient != nil else {
            lastCoreWarning = "Build mcb-core or set MCB_CORE_PATH"
            statusMessage = "Core helper unavailable"
            return
        }

        statusMessage = "Refreshing"
        await refreshProcesses()
        await refreshSessions()
        if let firstProfile = profiles.first {
            await refreshWorktrees(repoPath: firstProfile.repoPath)
        }
        statusMessage = "Refreshed"
    }

    public func refreshProcesses() async {
        statusMessage = "Scanning processes"
        await refresh(.scanProcesses, moduleID: "processes")
        statusMessage = "Refreshed processes"
    }

    public func refreshSessions() async {
        statusMessage = "Scanning agent sessions"
        await refresh(.scanSessions, moduleID: "sessions")
        statusMessage = "Refreshed agent sessions"
    }

    public func refreshWorktrees(repoPath: String? = nil) async {
        guard let repoPath = repoPath ?? profiles.first?.repoPath else {
            statusMessage = "No project profile for worktree scan"
            return
        }

        statusMessage = "Scanning worktrees"
        await refresh(
            .scanWorktrees,
            moduleID: "worktrees",
            payload: ["repoPath": .string(repoPath)]
        )
        statusMessage = "Refreshed worktrees"
    }

    public func planKill(pid: Int) -> ConfirmableAction {
        ConfirmableAction(
            actionId: "kill-\(pid)",
            kind: .killProcess,
            targetLabel: "pid \(pid)",
            risk: .high,
            commandPreview: "kill -TERM \(pid)"
        )
    }

    public func requestKill(_ process: ProcessRecord) {
        pendingAction = ConfirmableAction(
            actionId: "kill-\(process.pid)",
            kind: .killProcess,
            targetLabel: "\(process.name) pid \(process.pid)",
            risk: .high,
            commandPreview: "kill -TERM \(process.pid)"
        )
    }

    public func requestDelete(_ worktree: WorktreeRecord) {
        switch ConfirmationPolicy().deletionDecision(for: worktree) {
        case .blocked(let reason):
            statusMessage = "Delete blocked: \(reason)"
            pendingAction = nil
        case .allowed, .requiresConfirmation, .unknown:
            pendingAction = ConfirmableAction(
                actionId: "delete-\(worktree.path.hashValue)",
                kind: .deleteWorktree,
                targetLabel: worktree.path,
                risk: .high,
                commandPreview: "git worktree remove \(shellQuote(worktree.path))"
            )
        }
    }

    public func executePendingAction() async {
        guard let action = pendingAction else {
            return
        }

        do {
            let exitCode = try await Task.detached(priority: .utility) {
                let process = Process()
                process.executableURL = URL(fileURLWithPath: "/bin/zsh")
                process.arguments = ["-lc", action.commandPreview]
                try process.run()
                process.waitUntilExit()
                return process.terminationStatus
            }.value
            statusMessage = exitCode == 0 ? "Action completed" : "Action exited \(exitCode)"
        } catch {
            statusMessage = "Action failed: \(error.localizedDescription)"
        }

        pendingAction = nil
        await refreshSnapshots()
    }

    public func cancelPendingAction() {
        pendingAction = nil
    }

    private func refresh(
        _ action: CoreAction,
        moduleID: String,
        payload: [String: JSONValue] = [:]
    ) async {
        do {
            let response = try await coreClient?.send(
                CoreRequest(action: action, dryRun: true, payload: payload)
            )
            guard let response else { return }
            let count = response.data["count"]?.intValue ?? 0
            updateRecords(action: action, response: response)
            updateModule(moduleID, count: count, status: response.ok ? "Live" : "Check")
            lastCoreWarning = response.warnings.first
        } catch {
            updateModule(moduleID, count: 0, status: "Unavailable")
            lastCoreWarning = error.localizedDescription
        }
    }

    private func updateModule(_ id: String, count: Int, status: String) {
        guard let index = modules.firstIndex(where: { $0.id == id }) else {
            return
        }
        modules[index].count = count
        modules[index].status = status
    }

    private func updateRecords(action: CoreAction, response: CoreResponse) {
        do {
            switch action {
            case .scanProcesses:
                processes = try response.data["processes"]?.decode() ?? []
            case .scanSessions:
                sessions = try response.data["sessions"]?.decode() ?? []
            case .scanWorktrees:
                worktrees = try response.data["worktrees"]?.decode() ?? []
            case .healthSnapshot, .planKillProcess:
                break
            }
        } catch {
            lastCoreWarning = "Could not decode \(action.rawValue): \(error.localizedDescription)"
        }
    }

    private func persistState() {
        guard let persistence else {
            return
        }

        do {
            try persistence.save(
                AppPersistenceSnapshot(
                    profiles: profiles,
                    clipboardVault: clipboardVault
                )
            )
        } catch {
            statusMessage = "Storage failed: \(error.localizedDescription)"
        }
    }

    private static func defaultModules(profileCount: Int, clipboardCount: Int = 0) -> [DashboardModule] {
        [
            DashboardModule(id: "projects", title: "Projects", symbol: "rectangle.stack", count: profileCount, status: "Profiles", accent: .blue),
            DashboardModule(id: "processes", title: "Processes", symbol: "cpu", count: 0, status: "Scan ready", accent: .orange),
            DashboardModule(id: "sessions", title: "Agent Sessions", symbol: "terminal", count: 0, status: "Scan ready", accent: .green),
            DashboardModule(id: "worktrees", title: "Worktrees", symbol: "point.3.connected.trianglepath.dotted", count: 0, status: "Scan ready", accent: .slate),
            DashboardModule(id: "clipboard", title: "Clipboard Vault", symbol: "doc.on.clipboard", count: clipboardCount, status: "Local", accent: .blue),
            DashboardModule(id: "artifacts", title: "Artifacts", symbol: "doc.badge.plus", count: 6, status: "Templates", accent: .green),
            DashboardModule(id: "cleanup", title: "Repo Cleanup", symbol: "externaldrive.badge.minus", count: 0, status: "Confirm only", accent: .red),
            DashboardModule(id: "health", title: "Dev Health", symbol: "gauge.with.dots.needle.67percent", count: 0, status: "Local", accent: .orange),
            DashboardModule(id: "commands", title: "Commands", symbol: "command", count: 0, status: "Palette", accent: .slate),
            DashboardModule(id: "focus", title: "Focus", symbol: "sparkle.magnifyingglass", count: 0, status: "Reversible", accent: .green)
        ]
    }
}

private func shellQuote(_ value: String) -> String {
    "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'"
}
