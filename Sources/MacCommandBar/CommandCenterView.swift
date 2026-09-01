import AppKit
import MacCommandBarKit
import SwiftUI

struct CommandCenterView: View {
    @EnvironmentObject private var state: AppState
    @State private var editingProfileDraft: ProjectProfileDraft?

    var body: some View {
        ZStack {
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

                    ModuleDetailView { draft in
                        editingProfileDraft = draft
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                }
            }

            if let draft = editingProfileDraft {
                InWindowModalLayer {
                    ProfileEditorPanel(
                        initialDraft: draft,
                        onCancel: {
                            editingProfileDraft = nil
                        },
                        onSave: { savedDraft in
                            if state.saveProfileDraft(savedDraft) {
                                editingProfileDraft = nil
                            }
                        }
                    )
                    .id(draft.id)
                }
            }

            if let action = state.pendingAction {
                InWindowModalLayer {
                    ConfirmActionPanel(action: action)
                        .environmentObject(state)
                }
            }
        }
        .frame(width: 660, height: 620)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}

private struct InWindowModalLayer<Content: View>: View {
    let content: Content

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    var body: some View {
        ZStack {
            Color.black.opacity(0.42)
                .contentShape(Rectangle())
                .onTapGesture {}

            content
                .background(Color(nsColor: .windowBackgroundColor))
                .clipShape(RoundedRectangle(cornerRadius: 16))
                .overlay(
                    RoundedRectangle(cornerRadius: 16)
                        .stroke(Color(nsColor: .separatorColor), lineWidth: 1)
                )
                .shadow(color: .black.opacity(0.35), radius: 24, x: 0, y: 14)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .transition(.opacity)
        .zIndex(10)
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
                Text("Assembly")
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
    let onEditProfile: (ProjectProfileDraft) -> Void

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
            ProjectsPanel(onEditProfile: onEditProfile)
        case "source":
            SourcePreviewPanel()
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
                note: "Every cleanup action opens a confirmation panel before execution."
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

            Button {
                state.deleteClipboardItem(item.id)
            } label: {
                Image(systemName: "trash")
            }
            .buttonStyle(.borderless)
            .foregroundStyle(.red)
            .help("Delete")
        }
        .padding(.vertical, 9)
    }
}

private struct ProjectsPanel: View {
    @EnvironmentObject private var state: AppState
    let onEditProfile: (ProjectProfileDraft) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Project profiles")
                    .font(.system(size: 13, weight: .semibold))
                Spacer()
                Button {
                    onEditProfile(ProjectProfileDraft())
                } label: {
                    Label("Add", systemImage: "plus")
                }
            }

            ForEach(state.profiles) { profile in
                VStack(alignment: .leading, spacing: 6) {
                    HStack {
                        Text(profile.name)
                            .font(.system(size: 13, weight: .semibold))
                        Spacer()
                        Button {
                            onEditProfile(ProjectProfileDraft(profile: profile))
                        } label: {
                            Image(systemName: "pencil")
                        }
                        .buttonStyle(.borderless)
                        .help("Edit")
                        Button {
                            state.openProjectFolder(profile.repoPath)
                        } label: {
                            Image(systemName: "folder")
                        }
                        .buttonStyle(.borderless)
                        .help("Open folder")
                        Text(profile.ports.map(String.init).joined(separator: ", "))
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundStyle(.secondary)
                        Button {
                            state.deleteProfile(profile.id)
                        } label: {
                            Image(systemName: "trash")
                        }
                        .buttonStyle(.borderless)
                        .foregroundStyle(.red)
                        .help("Delete")
                    }
                    Text(profile.repoPath)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                    HStack {
                        ForEach(profile.commands) { command in
                            Button {
                                state.launchProfileCommand(command)
                            } label: {
                                Label(command.label, systemImage: "terminal")
                            }
                        }
                    }
                    if !profile.urls.isEmpty {
                        HStack {
                            ForEach(profile.urls, id: \.self) { url in
                                Button {
                                    state.openProjectURL(url)
                                } label: {
                                    Label(projectURLLabel(url), systemImage: "safari")
                                }
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

private func projectURLLabel(_ rawURL: String) -> String {
    URL(string: rawURL)?.host ?? rawURL
}

private enum SourcePreviewPanelLayout {
    static let treeHeight: CGFloat = 136
    static let previewHeight: CGFloat = 300
}

private struct SourcePreviewPanel: View {
    @EnvironmentObject private var state: AppState
    @State private var selectedProfileID: UUID?
    @State private var browser = SourcePreviewBrowserState()
    @State private var expandedFolderIDs: Set<String> = []

    private var selectedProfile: ProjectProfile? {
        if let selectedProfileID,
           let profile = state.profiles.first(where: { $0.id == selectedProfileID }) {
            return profile
        }
        return state.profiles.first
    }

    private var filteredSourceFiles: [SourceFileRecord] {
        browser.filteredFiles(from: state.sourceFiles)
    }

    private var sourceTree: [SourceFileTreeNode] {
        SourceFileTreeNode.build(from: filteredSourceFiles)
    }

    private var autoExpandFolders: Bool {
        !browser.trimmedFilterText.isEmpty
    }

    private var previewTargetPath: String {
        browser.previewTargetPath(repoPath: selectedProfile?.repoPath, files: state.sourceFiles)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Picker("Project", selection: $selectedProfileID) {
                    ForEach(state.profiles) { profile in
                        Text(profile.name).tag(Optional(profile.id))
                    }
                }
                .labelsHidden()
                .frame(width: 150)
                .disabled(state.profiles.isEmpty)

                Button {
                    scanSelectedProject()
                } label: {
                    Label("Scan", systemImage: "folder.badge.gearshape")
                }
                .disabled(selectedProfile == nil)

                TextField("Search file or paste source path", text: $browser.filterText)
                    .textFieldStyle(.roundedBorder)

                Button {
                    preview(path: previewTargetPath)
                } label: {
                    Label("Preview", systemImage: "doc.text.magnifyingglass")
                }
                .disabled(previewTargetPath.isEmpty)
            }
            .zIndex(3)

            if !state.sourceFiles.isEmpty {
                ScrollView {
                    if sourceTree.isEmpty {
                        VStack(alignment: .leading) {
                            Text("No scanned files match")
                                .font(.system(size: 11))
                                .foregroundStyle(.secondary)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.vertical, 8)
                        }
                    } else {
                        SourceFileTreeBrowser(
                            nodes: sourceTree,
                            activeFilePath: state.sourcePreview?.path,
                            autoExpandFolders: autoExpandFolders,
                            expandedFolderIDs: $expandedFolderIDs
                        ) { file in
                            browser.select(file)
                            preview(path: file.path)
                        }
                    }
                }
                .frame(height: SourcePreviewPanelLayout.treeHeight)
                .background(Color(nsColor: .textBackgroundColor))
                .clipShape(RoundedRectangle(cornerRadius: 7))
                .zIndex(2)
            }

            if let preview = state.sourcePreview {
                HStack(alignment: .firstTextBaseline) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(preview.fileName)
                            .font(.system(size: 13, weight: .semibold))
                        Text(preview.path)
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                    Spacer()
                    Text("\(preview.language) - \(preview.lineCount) lines")
                        .font(.system(size: 10, weight: .medium, design: .monospaced))
                        .foregroundStyle(.secondary)
                }

                SourceCodeTextView(preview: preview)
                    .frame(height: SourcePreviewPanelLayout.previewHeight)
                    .clipShape(RoundedRectangle(cornerRadius: 7))
                    .zIndex(1)
            } else {
                EmptyModuleState(
                    symbol: "doc.text.magnifyingglass",
                    title: "No source file loaded",
                    detail: "Choose a project file to preview it with native text rendering."
                )
            }
        }
        .onAppear {
            selectedProfileID = selectedProfileID ?? state.profiles.first?.id
        }
        .onChange(of: selectedProfileID) { _, _ in
            browser.reset()
        }
    }

    private func scanSelectedProject() {
        guard let repoPath = selectedProfile?.repoPath else {
            return
        }
        browser.selectedFile = nil
        Task {
            await state.refreshSourceFiles(rootPath: repoPath)
        }
    }

    private func preview(path: String) {
        Task {
            await state.previewSourceFile(path: path)
        }
    }
}

private struct SourceFileTreeBrowser: View {
    let nodes: [SourceFileTreeNode]
    let activeFilePath: String?
    let autoExpandFolders: Bool
    @Binding var expandedFolderIDs: Set<String>
    let onSelect: (SourceFileRecord) -> Void

    var body: some View {
        LazyVStack(spacing: 0) {
            ForEach(nodes) { node in
                SourceFileTreeNodeRow(
                    node: node,
                    level: 0,
                    activeFilePath: activeFilePath,
                    autoExpandFolders: autoExpandFolders,
                    expandedFolderIDs: $expandedFolderIDs,
                    onSelect: onSelect
                )
            }
        }
        .padding(.vertical, 4)
    }
}

private struct SourceFileTreeNodeRow: View {
    let node: SourceFileTreeNode
    let level: Int
    let activeFilePath: String?
    let autoExpandFolders: Bool
    @Binding var expandedFolderIDs: Set<String>
    let onSelect: (SourceFileRecord) -> Void

    private var isExpanded: Bool {
        autoExpandFolders || expandedFolderIDs.contains(node.id)
    }

    private var isActiveFile: Bool {
        node.file?.path == activeFilePath
    }

    var body: some View {
        VStack(spacing: 0) {
            Button {
                if node.isFolder {
                    toggleFolder()
                } else if let file = node.file {
                    onSelect(file)
                }
            } label: {
                HStack(spacing: 7) {
                    if node.isFolder {
                        Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                            .font(.system(size: 9, weight: .semibold))
                            .foregroundStyle(.secondary)
                            .frame(width: 11)
                    } else {
                        Color.clear.frame(width: 11, height: 1)
                    }

                    Image(systemName: symbolName)
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(node.isFolder ? Color.secondary : Color.blue)
                        .frame(width: 16)

                    Text(node.name)
                        .font(.system(size: 11, weight: node.isFolder ? .semibold : .medium))
                        .lineLimit(1)

                    Spacer(minLength: 8)

                    if let file = node.file {
                        Text(file.language)
                            .font(.system(size: 9, weight: .medium, design: .monospaced))
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    } else {
                        Text("\(node.children.count)")
                            .font(.system(size: 9, weight: .medium, design: .monospaced))
                            .foregroundStyle(.secondary)
                    }
                }
                .contentShape(Rectangle())
                .frame(height: 24)
                .padding(.leading, CGFloat(level) * 14 + 8)
                .padding(.trailing, 8)
                .background(
                    RoundedRectangle(cornerRadius: 5)
                        .fill(isActiveFile ? Color.accentColor.opacity(0.18) : .clear)
                )
            }
            .buttonStyle(.plain)

            if node.isFolder, isExpanded {
                ForEach(node.children) { child in
                    SourceFileTreeNodeRow(
                        node: child,
                        level: level + 1,
                        activeFilePath: activeFilePath,
                        autoExpandFolders: autoExpandFolders,
                        expandedFolderIDs: $expandedFolderIDs,
                        onSelect: onSelect
                    )
                }
            }
        }
    }

    private var symbolName: String {
        if node.isFolder {
            return isExpanded ? "folder.open" : "folder"
        }
        return node.file?.language == "csharp" ? "curlybraces" : "doc.text"
    }

    private func toggleFolder() {
        if expandedFolderIDs.contains(node.id) {
            expandedFolderIDs.remove(node.id)
        } else {
            expandedFolderIDs.insert(node.id)
        }
    }
}

private struct SourceCodeTextView: NSViewRepresentable {
    let preview: SourcePreview

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSScrollView()
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = true
        scrollView.autohidesScrollers = true
        scrollView.borderType = .noBorder
        scrollView.drawsBackground = true
        scrollView.backgroundColor = .textBackgroundColor

        let textView = NSTextView()
        textView.isEditable = false
        textView.isSelectable = true
        textView.isRichText = false
        textView.drawsBackground = true
        textView.backgroundColor = .textBackgroundColor
        textView.textContainerInset = NSSize(width: 10, height: 10)
        textView.textContainer?.widthTracksTextView = false
        textView.textContainer?.containerSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        textView.minSize = NSSize(width: 0, height: 0)
        textView.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        textView.isHorizontallyResizable = true
        textView.isVerticallyResizable = true
        textView.autoresizingMask = [.width]

        scrollView.documentView = textView
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let textView = scrollView.documentView as? NSTextView else {
            return
        }
        textView.textStorage?.setAttributedString(
            SourceCodeAttributedStringBuilder.attributedString(for: preview)
        )
    }
}

private enum SourceCodeAttributedStringBuilder {
    static func attributedString(for preview: SourcePreview) -> NSAttributedString {
        let paragraphStyle = NSMutableParagraphStyle()
        paragraphStyle.lineBreakMode = .byClipping
        paragraphStyle.lineSpacing = 1

        let baseFont = NSFont.monospacedSystemFont(ofSize: 11, weight: .regular)
        let highlighted = NSMutableAttributedString(
            string: preview.content,
            attributes: [
                .font: baseFont,
                .foregroundColor: NSColor.textColor,
                .paragraphStyle: paragraphStyle
            ]
        )

        for span in preview.spans {
            guard let range = nsRange(start: span.start, end: span.end, in: preview.content) else {
                continue
            }
            highlighted.addAttributes(attributes(for: span.role), range: range)
        }

        return highlighted
    }

    private static func attributes(for role: SourceSyntaxRole) -> [NSAttributedString.Key: Any] {
        switch role {
        case .keyword:
            return [
                .foregroundColor: NSColor.systemPink,
                .font: NSFont.monospacedSystemFont(ofSize: 11, weight: .semibold)
            ]
        case .type:
            return [.foregroundColor: NSColor.systemTeal]
        case .function, .constructor:
            return [.foregroundColor: NSColor.systemBlue]
        case .parameter:
            return [.foregroundColor: NSColor.systemMint]
        case .module:
            return [.foregroundColor: NSColor.systemPurple]
        case .attribute:
            return [.foregroundColor: NSColor.systemIndigo]
        case .variable:
            return [.foregroundColor: NSColor.labelColor]
        case .property:
            return [.foregroundColor: NSColor.systemCyan]
        case .constant:
            return [.foregroundColor: NSColor.systemOrange]
        case .string:
            return [.foregroundColor: NSColor.systemGreen]
        case .comment:
            return [.foregroundColor: NSColor.secondaryLabelColor]
        case .number:
            return [.foregroundColor: NSColor.systemOrange]
        case .operator:
            return [.foregroundColor: NSColor.systemRed]
        case .punctuation:
            return [.foregroundColor: NSColor.tertiaryLabelColor]
        }
    }

    private static func nsRange(start: Int, end: Int, in content: String) -> NSRange? {
        guard start >= 0, end > start, end <= content.utf8.count else {
            return nil
        }
        let startUTF8 = content.utf8.index(content.utf8.startIndex, offsetBy: start)
        let endUTF8 = content.utf8.index(content.utf8.startIndex, offsetBy: end)
        guard
            let startIndex = String.Index(startUTF8, within: content),
            let endIndex = String.Index(endUTF8, within: content)
        else {
            return nil
        }
        return NSRange(startIndex..<endIndex, in: content)
    }
}

private struct ProfileEditorPanel: View {
    let onCancel: () -> Void
    let onSave: (ProjectProfileDraft) -> Void
    @State private var draft: ProjectProfileDraft

    init(
        initialDraft: ProjectProfileDraft,
        onCancel: @escaping () -> Void,
        onSave: @escaping (ProjectProfileDraft) -> Void
    ) {
        self.onCancel = onCancel
        self.onSave = onSave
        _draft = State(initialValue: initialDraft)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                Label(draft.profileID == nil ? "Add Profile" : "Edit Profile", systemImage: "rectangle.stack.badge.plus")
                    .font(.system(size: 16, weight: .semibold))
                Spacer()
            }

            Form {
                Section("Project") {
                    TextField("Name", text: $draft.name)
                    TextField("Repository path", text: $draft.repoPath)
                }

                Section("Local") {
                    TextField("Ports", text: $draft.portsText)
                    labeledEditor("Worktree roots", text: $draft.worktreeRootsText, height: 66)
                }

                Section("URLs") {
                    labeledEditor("URLs", text: $draft.urlsText, height: 66)
                }
            }

            HStack {
                Spacer()
                Button("Cancel") {
                    onCancel()
                }
                Button("Save") {
                    onSave(draft)
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(20)
        .frame(width: 520)
    }

    private func labeledEditor(_ title: String, text: Binding<String>, height: CGFloat) -> some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(title)
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(.secondary)
            TextEditor(text: text)
                .font(.system(size: 11, design: .monospaced))
                .frame(minHeight: height, maxHeight: height)
                .scrollContentBackground(.hidden)
                .background(Color(nsColor: .textBackgroundColor))
                .clipShape(RoundedRectangle(cornerRadius: 6))
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
    @EnvironmentObject private var state: AppState
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
                    state.launchTerminalCommand(command, label: "Resume")
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
    @EnvironmentObject private var state: AppState

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Local environment")
                    .font(.system(size: 13, weight: .semibold))
                Spacer()
                Button {
                    Task {
                        await state.refreshHealth()
                    }
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
            }

            if let snapshot = state.healthSnapshot {
                HealthValueRow(label: "Hostname", value: snapshot.hostname)
                HealthValueRow(label: "Current directory", value: snapshot.cwd)
                HealthValueRow(label: "Home", value: snapshot.home)
            } else {
                EmptyModuleState(
                    symbol: "gauge.with.dots.needle.67percent",
                    title: "No health snapshot loaded",
                    detail: "Refresh asks mcb-core for the local environment snapshot."
                )
            }
        }
    }
}

private struct HealthValueRow: View {
    let label: String
    let value: String?

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Text(label)
                .font(.system(size: 12, weight: .medium))
                .frame(width: 112, alignment: .leading)
            Text(value ?? "Unavailable")
                .font(.system(size: 11, design: .monospaced))
                .foregroundStyle(value == nil ? .secondary : .primary)
                .lineLimit(2)
            Spacer()
        }
        .padding(.vertical, 6)
        Divider()
    }
}

private struct CommandPalettePanel: View {
    @EnvironmentObject private var state: AppState
    @State private var commandQuery = ""

    var body: some View {
        let items = state.commandPaletteItems(matching: commandQuery)

        VStack(alignment: .leading, spacing: 10) {
            TextField("Search commands", text: $commandQuery)
                .textFieldStyle(.roundedBorder)
            if items.isEmpty {
                EmptyModuleState(
                    symbol: "command",
                    title: "No commands found",
                    detail: "Profile commands and scanned agent sessions appear here."
                )
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(items) { item in
                            CommandPaletteRow(item: item)
                            Divider()
                        }
                    }
                }
            }
        }
    }
}

private struct CommandPaletteRow: View {
    @EnvironmentObject private var state: AppState
    let item: CommandPaletteItem

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: item.source == "Codex" || item.source == "Claude" ? "terminal" : "command")
                .foregroundStyle(.secondary)
                .frame(width: 18)

            VStack(alignment: .leading, spacing: 4) {
                Text(item.title)
                    .font(.system(size: 12, weight: .semibold))
                    .lineLimit(1)
                Text(item.source)
                    .font(.system(size: 9, weight: .semibold, design: .monospaced))
                    .foregroundStyle(.secondary)
                Text(item.command)
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
            }

            Spacer()

            Button {
                state.launchCommandPaletteItem(item)
            } label: {
                Image(systemName: "play")
            }
            .buttonStyle(.borderless)
            .help("Run")
        }
        .padding(.vertical, 9)
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

private struct ConfirmActionPanel: View {
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
