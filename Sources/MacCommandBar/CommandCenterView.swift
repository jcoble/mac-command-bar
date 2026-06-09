import AppKit
import MacCommandBarKit
import SwiftUI

struct CommandCenterView: View {
    @EnvironmentObject private var state: AppState

    var body: some View {
        VStack(spacing: 0) {
            HeaderView()
                .padding(.horizontal, 16)
                .padding(.top, 14)
                .padding(.bottom, 10)

            Divider()

            HStack(spacing: 0) {
                ModuleSidebar()
                    .frame(width: 184)

                Divider()

                ModuleDetailView()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(width: 660, height: 620)
        .background(Color(nsColor: .windowBackgroundColor))
        .sheet(
            item: Binding<ConfirmableAction?>(
                get: { state.pendingAction },
                set: { state.pendingAction = $0 }
            )
        ) { action in
            ConfirmActionSheet(action: action)
                .environmentObject(state)
        }
    }
}

private struct HeaderView: View {
    @EnvironmentObject private var state: AppState

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: "command.square")
                .font(.system(size: 22, weight: .semibold))
                .foregroundStyle(.blue)

            VStack(alignment: .leading, spacing: 2) {
                Text("MacCommandBar")
                    .font(.system(size: 15, weight: .semibold))
                Text(state.statusMessage)
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer(minLength: 12)

            Button {
                Task {
                    await state.refreshSnapshots()
                }
            } label: {
                Image(systemName: "arrow.clockwise")
            }
            .buttonStyle(.borderless)
            .help("Refresh")

            SettingsLink {
                Image(systemName: "gearshape")
            }
            .buttonStyle(.borderless)
            .help("Settings")

            Button {
                NSApplication.shared.terminate(nil)
            } label: {
                Image(systemName: "power")
            }
            .buttonStyle(.borderless)
            .help("Quit")
        }
    }
}

private struct ModuleSidebar: View {
    @EnvironmentObject private var state: AppState

    var body: some View {
        ScrollView {
            LazyVStack(spacing: 4) {
                ForEach(state.modules) { module in
                    Button {
                        state.selectedModuleID = module.id
                    } label: {
                        HStack(spacing: 9) {
                            Image(systemName: module.symbol)
                                .frame(width: 18)
                                .foregroundStyle(module.accent.color)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(module.title)
                                    .font(.system(size: 12, weight: .medium))
                                    .lineLimit(1)
                                Text(module.status)
                                    .font(.system(size: 10))
                                    .foregroundStyle(.secondary)
                                    .lineLimit(1)
                            }
                            Spacer(minLength: 4)
                            Text("\(module.count)")
                                .font(.system(size: 10, weight: .semibold, design: .monospaced))
                                .foregroundStyle(.secondary)
                        }
                        .padding(.horizontal, 10)
                        .padding(.vertical, 8)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .background(
                        RoundedRectangle(cornerRadius: 7)
                            .fill(state.selectedModuleID == module.id ? Color.accentColor.opacity(0.13) : .clear)
                    )
                }
            }
            .padding(10)
        }
    }
}

private struct ModuleDetailView: View {
    @EnvironmentObject private var state: AppState

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            if let module = state.selectedModule {
                HStack {
                    Label(module.title, systemImage: module.symbol)
                        .font(.system(size: 17, weight: .semibold))
                    Spacer()
                    Text(module.status)
                        .font(.system(size: 11, weight: .medium))
                        .foregroundStyle(.secondary)
                }

                detail(for: module.id)
            }

            Spacer(minLength: 0)
        }
        .padding(16)
    }

    @ViewBuilder
    private func detail(for moduleID: String) -> some View {
        switch moduleID {
        case "clipboard":
            ClipboardPanel()
        case "projects":
            ProjectsPanel()
        case "processes":
            ProcessesPanel()
        case "sessions":
            SessionsPanel()
        case "worktrees":
            WorktreesPanel()
        case "artifacts":
            ArtifactPanel()
        case "cleanup":
            ConfirmationPanel(
                title: "Cleanup queue",
                command: "rm -rf selected build artifacts",
                note: "Every cleanup action opens a confirmation sheet before execution."
            )
        case "health":
            HealthPanel()
        case "commands":
            CommandPalettePanel()
        case "focus":
            ConfirmationPanel(
                title: "Indexing controls",
                command: "mdutil and watcher changes are planned but reversible",
                note: "V1 surfaces plans and state before applying focus changes."
            )
        default:
            Text("Module pending")
                .foregroundStyle(.secondary)
        }
    }
}

private struct ClipboardPanel: View {
    @EnvironmentObject private var state: AppState
    @State private var query = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 8) {
                TextField("Search saved clipboard", text: $query)
                    .textFieldStyle(.roundedBorder)
                Button {
                    state.captureClipboardText()
                } label: {
                    Label("Capture", systemImage: "plus")
                }
            }

            ScrollView {
                LazyVStack(spacing: 0) {
                    ForEach(state.clipboardVault.search(query)) { item in
                        ClipboardRow(item: item)
                        Divider()
                    }
                    if state.clipboardVault.items.isEmpty {
                        EmptyModuleState(
                            symbol: "doc.on.clipboard",
                            title: "No saved clipboard items",
                            detail: "Capture the current text clipboard to seed the vault."
                        )
                    }
                }
            }
        }
    }
}

private struct ClipboardRow: View {
    @EnvironmentObject private var state: AppState
    let item: ClipboardItem

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: item.isSecret ? "lock.fill" : "text.alignleft")
                .foregroundStyle(item.isSecret ? .orange : .secondary)
                .frame(width: 18)
            VStack(alignment: .leading, spacing: 4) {
                Text(item.preview)
                    .font(.system(size: 12))
                    .lineLimit(2)
                HStack(spacing: 6) {
                    Text(item.kind.rawValue.uppercased())
                    if item.isSecret {
                        Text("SECRET")
                    }
                    if item.isPinned {
                        Text("PINNED")
                    }
                }
                .font(.system(size: 9, weight: .semibold, design: .monospaced))
                .foregroundStyle(.secondary)
            }
            Spacer()
            Button {
                state.restoreClipboardItem(item.id)
            } label: {
                Image(systemName: "doc.on.clipboard")
            }
            .buttonStyle(.borderless)
            .disabled(item.content == nil)
            .help("Copy to clipboard")

            Button {
                state.togglePinnedClipboardItem(item.id)
            } label: {
                Image(systemName: item.isPinned ? "pin.fill" : "pin")
            }
            .buttonStyle(.borderless)
            .help("Pin")
        }
        .padding(.vertical, 9)
    }
}

private struct ProjectsPanel: View {
    @EnvironmentObject private var state: AppState

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            ForEach(state.profiles) { profile in
                VStack(alignment: .leading, spacing: 6) {
                    HStack {
                        Text(profile.name)
                            .font(.system(size: 13, weight: .semibold))
                        Spacer()
                        Text(profile.ports.map(String.init).joined(separator: ", "))
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundStyle(.secondary)
                    }
                    Text(profile.repoPath)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                    HStack {
                        ForEach(profile.commands) { command in
                            Button {
                                TerminalLauncher.open(command.command)
                            } label: {
                                Label(command.label, systemImage: "terminal")
                            }
                        }
                    }
                }
                .padding(.vertical, 8)
                Divider()
            }
        }
    }
}

private struct ProcessesPanel: View {
    @EnvironmentObject private var state: AppState

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Listening processes")
                    .font(.system(size: 13, weight: .semibold))
                Spacer()
                Button {
                    Task {
                        await state.refreshProcesses()
                    }
                } label: {
                    Label("Scan", systemImage: "arrow.clockwise")
                }
            }

            if state.processes.isEmpty {
                EmptyModuleState(
                    symbol: "cpu",
                    title: "No listener data loaded",
                    detail: "Refresh scans local TCP listeners through mcb-core."
                )
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(state.processes) { process in
                            ProcessRow(process: process)
                            Divider()
                        }
                    }
                }
            }
        }
    }
}

private struct SessionsPanel: View {
    @EnvironmentObject private var state: AppState

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Recent agent sessions")
                    .font(.system(size: 13, weight: .semibold))
                Spacer()
                Button {
                    Task {
                        await state.refreshSessions()
                    }
                } label: {
                    Label("Scan", systemImage: "arrow.clockwise")
                }
            }

            if state.sessions.isEmpty {
                EmptyModuleState(
                    symbol: "terminal",
                    title: "No sessions loaded",
                    detail: "Refresh reads Codex and Claude session indexes from your home directory."
                )
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(state.sessions) { session in
                            SessionRow(session: session)
                            Divider()
                        }
                    }
                }
            }
        }
    }
}

private struct WorktreesPanel: View {
    @EnvironmentObject private var state: AppState

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Worktree safety")
                    .font(.system(size: 13, weight: .semibold))
                Spacer()
                Button {
                    Task {
                        await state.refreshWorktrees()
                    }
                } label: {
                    Label("Scan", systemImage: "arrow.clockwise")
                }
            }

            if state.worktrees.isEmpty {
                EmptyModuleState(
                    symbol: "point.3.connected.trianglepath.dotted",
                    title: "No worktree data loaded",
                    detail: "Refresh scans the first configured project profile."
                )
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(state.worktrees) { worktree in
                            WorktreeRow(worktree: worktree)
                            Divider()
                        }
                    }
                }
            }
        }
    }
}

private struct ProcessRow: View {
    @EnvironmentObject private var state: AppState
    let process: ProcessRecord

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: "cpu")
                .foregroundStyle(.orange)
                .frame(width: 18)

            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(process.name)
                        .font(.system(size: 12, weight: .semibold))
                    Text("pid \(process.pid)")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.secondary)
                }
                Text("ports \(process.listeningPorts.map(String.init).joined(separator: ", "))")
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(.secondary)
                if let cwd = process.cwd {
                    Text(cwd)
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.tertiary)
                        .lineLimit(1)
                }
            }

            Spacer()

            Button {
                state.requestKill(process)
            } label: {
                Image(systemName: "xmark.octagon")
            }
            .buttonStyle(.borderless)
            .help("Plan kill")
        }
        .padding(.vertical, 9)
    }
}

private struct SessionRow: View {
    let session: AgentSessionRecord

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: session.provider == .codex ? "command" : "text.bubble")
                .foregroundStyle(session.provider == .codex ? .blue : .green)
                .frame(width: 18)

            VStack(alignment: .leading, spacing: 4) {
                Text(session.title)
                    .font(.system(size: 12, weight: .semibold))
                    .lineLimit(1)
                HStack(spacing: 7) {
                    Text(session.provider.rawValue.uppercased())
                    if let lastActivity = session.lastActivity {
                        Text(lastActivity)
                    }
                }
                .font(.system(size: 9, design: .monospaced))
                .foregroundStyle(.secondary)
                if let projectPath = session.projectPath {
                    Text(projectPath)
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.tertiary)
                        .lineLimit(1)
                }
            }

            Spacer()

            Button {
                if let command = session.resumeCommands.first {
                    TerminalLauncher.open(command)
                }
            } label: {
                Image(systemName: "play")
            }
            .buttonStyle(.borderless)
            .help("Resume")
        }
        .padding(.vertical, 9)
    }
}

private struct WorktreeRow: View {
    @EnvironmentObject private var state: AppState
    let worktree: WorktreeRecord

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: worktree.isDirty ? "exclamationmark.triangle" : "checkmark.circle")
                .foregroundStyle(worktree.isDirty ? .orange : .green)
                .frame(width: 18)

            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(worktree.branch)
                        .font(.system(size: 12, weight: .semibold))
                    Text(worktree.repo)
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.secondary)
                }
                Text(worktree.path)
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Text(worktree.deleteEligibility.label)
                    .font(.system(size: 9, weight: .semibold, design: .monospaced))
                    .foregroundStyle(worktree.deleteEligibility.isBlocked ? .orange : .secondary)
            }

            Spacer()

            Button {
                state.requestDelete(worktree)
            } label: {
                Image(systemName: "trash")
            }
            .buttonStyle(.borderless)
            .disabled(worktree.deleteEligibility.isBlocked)
            .help("Delete worktree")
        }
        .padding(.vertical, 9)
    }
}

private struct ArtifactPanel: View {
    private let templates = ["Markdown note", "PRD", "Handoff", "Changelog", "Notion task", "Test plan"]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(templates, id: \.self) { template in
                HStack {
                    Image(systemName: "doc.badge.plus")
                        .foregroundStyle(.green)
                    Text(template)
                    Spacer()
                    Text(".md")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.secondary)
                }
                .padding(.vertical, 6)
                Divider()
            }
        }
    }
}

private struct HealthPanel: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            CommandExampleRow(label: "CPU and RAM", command: "ps and system_profiler summary")
            CommandExampleRow(label: "Disk pressure", command: "du summaries by repo")
            CommandExampleRow(label: "Docker state", command: "docker ps when available")
        }
    }
}

private struct CommandPalettePanel: View {
    @State private var commandQuery = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            TextField("Search commands", text: $commandQuery)
                .textFieldStyle(.roundedBorder)
            CommandExampleRow(label: "Refresh everything", command: "mcb-core health.snapshot")
            CommandExampleRow(label: "Open active repo", command: "cd <repo>")
            CommandExampleRow(label: "Resume latest agent", command: "codex resume --last")
        }
    }
}

private struct ConfirmationPanel: View {
    let title: String
    let command: String
    let note: String

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(title)
                .font(.system(size: 13, weight: .semibold))
            Text(command)
                .font(.system(size: 12, design: .monospaced))
                .padding(9)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color(nsColor: .textBackgroundColor))
                .clipShape(RoundedRectangle(cornerRadius: 7))
            Text(note)
                .font(.system(size: 12))
                .foregroundStyle(.secondary)
            Button {
            } label: {
                Label("Open confirmation", systemImage: "checkmark.shield")
            }
            .disabled(true)
        }
    }
}

private struct ConfirmActionSheet: View {
    @EnvironmentObject private var state: AppState
    let action: ConfirmableAction

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Label("Confirm action", systemImage: "checkmark.shield")
                .font(.system(size: 16, weight: .semibold))

            Text(action.targetLabel)
                .font(.system(size: 12, weight: .medium))

            Text(action.commandPreview)
                .font(.system(size: 12, design: .monospaced))
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color(nsColor: .textBackgroundColor))
                .clipShape(RoundedRectangle(cornerRadius: 7))

            Text("This command will not run unless you confirm it here.")
                .font(.system(size: 12))
                .foregroundStyle(.secondary)

            HStack {
                Spacer()
                Button("Cancel") {
                    state.cancelPendingAction()
                }
                Button("Run") {
                    Task {
                        await state.executePendingAction()
                    }
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(20)
        .frame(width: 460)
    }
}

private struct CommandExampleRow: View {
    let label: String
    let command: String

    var body: some View {
        HStack(spacing: 10) {
            Text(label)
                .font(.system(size: 12, weight: .medium))
            Spacer()
            Text(command)
                .font(.system(size: 10, design: .monospaced))
                .foregroundStyle(.secondary)
                .lineLimit(1)
        }
        .padding(.vertical, 7)
        Divider()
    }
}

private struct EmptyModuleState: View {
    let symbol: String
    let title: String
    let detail: String

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: symbol)
                .font(.system(size: 28))
                .foregroundStyle(.secondary)
            Text(title)
                .font(.system(size: 13, weight: .semibold))
            Text(detail)
                .font(.system(size: 12))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 280)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 44)
    }
}

struct SettingsView: View {
    @EnvironmentObject private var state: AppState
    @State private var launchAtLoginStatus = LaunchAtLoginController.statusDescription

    var body: some View {
        Form {
            Section("General") {
                HStack {
                    Text("Launch at login")
                    Spacer()
                    Text(launchAtLoginStatus)
                        .foregroundStyle(.secondary)
                    Button("Enable") {
                        try? LaunchAtLoginController.setEnabled(true)
                        launchAtLoginStatus = LaunchAtLoginController.statusDescription
                    }
                    Button("Disable") {
                        try? LaunchAtLoginController.setEnabled(false)
                        launchAtLoginStatus = LaunchAtLoginController.statusDescription
                    }
                }
            }

            Section("Profiles") {
                ForEach(state.profiles) { profile in
                    Text(profile.repoPath)
                        .font(.system(.body, design: .monospaced))
                }
            }
        }
        .padding(20)
        .frame(width: 520)
    }
}

private enum TerminalLauncher {
    static func open(_ command: String) {
        let script = """
        tell application "Terminal"
          activate
          do script "\(command.escapingForAppleScript)"
        end tell
        """
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
        process.arguments = ["-e", script]
        try? process.run()
    }
}

private extension String {
    var escapingForAppleScript: String {
        replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"")
    }
}

private extension ModuleAccent {
    var color: Color {
        switch self {
        case .blue:
            return .blue
        case .green:
            return .green
        case .orange:
            return .orange
        case .red:
            return .red
        case .slate:
            return .secondary
        }
    }
}

private extension DeleteEligibility {
    var label: String {
        switch self {
        case .allowed:
            return "ALLOWED"
        case .requiresConfirmation:
            return "CONFIRM REQUIRED"
        case .blocked(let reason):
            return "BLOCKED: \(reason.uppercased())"
        case .unknown(let value):
            return value.uppercased()
        }
    }

    var isBlocked: Bool {
        if case .blocked = self {
            return true
        }
        return false
    }
}
