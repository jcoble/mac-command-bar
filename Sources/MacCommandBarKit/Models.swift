import Foundation

public enum JSONValue: Codable, Equatable, Hashable, Sendable {
    case string(String)
    case number(Double)
    case bool(Bool)
    case object([String: JSONValue])
    case array([JSONValue])
    case null

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let value = try? container.decode(Bool.self) {
            self = .bool(value)
        } else if let value = try? container.decode(Double.self) {
            self = .number(value)
        } else if let value = try? container.decode(String.self) {
            self = .string(value)
        } else if let value = try? container.decode([JSONValue].self) {
            self = .array(value)
        } else if let value = try? container.decode([String: JSONValue].self) {
            self = .object(value)
        } else {
            throw DecodingError.dataCorruptedError(
                in: container,
                debugDescription: "Unsupported JSON value"
            )
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .string(let value):
            try container.encode(value)
        case .number(let value):
            try container.encode(value)
        case .bool(let value):
            try container.encode(value)
        case .object(let value):
            try container.encode(value)
        case .array(let value):
            try container.encode(value)
        case .null:
            try container.encodeNil()
        }
    }

    public var stringValue: String? {
        if case .string(let value) = self { value } else { nil }
    }

    public var intValue: Int? {
        if case .number(let value) = self { Int(value) } else { nil }
    }

    public func decode<T: Decodable>(_ type: T.Type = T.self) throws -> T {
        let data = try JSONEncoder().encode(self)
        return try JSONDecoder().decode(T.self, from: data)
    }
}

public enum CoreAction: String, Codable, Sendable {
    case scanWorktrees = "scan.worktrees"
    case scanProcesses = "scan.processes"
    case scanSessions = "scan.sessions"
    case healthSnapshot = "health.snapshot"
    case sourcePreview = "source.preview"
    case planKillProcess = "plan.killProcess"
}

public struct CoreRequest: Codable, Equatable, Sendable {
    public var id: String
    public var action: CoreAction
    public var dryRun: Bool
    public var payload: [String: JSONValue]

    public init(
        id: String = UUID().uuidString,
        action: CoreAction,
        dryRun: Bool,
        payload: [String: JSONValue] = [:]
    ) {
        self.id = id
        self.action = action
        self.dryRun = dryRun
        self.payload = payload
    }
}

public struct CoreResponse: Codable, Equatable, Sendable {
    public var id: String
    public var ok: Bool
    public var summary: String
    public var data: [String: JSONValue]
    public var warnings: [String]
    public var proposedCommand: String?
}

public enum ConfirmableActionKind: String, Codable, Sendable {
    case killProcess = "kill-process"
    case deleteWorktree = "delete-worktree"
    case cleanRepo = "clean-repo"
    case runCommand = "run-command"
}

public enum ActionRisk: String, Codable, Sendable {
    case low
    case medium
    case high
}

public struct ConfirmableAction: Codable, Equatable, Identifiable, Sendable {
    public var actionId: String
    public var kind: ConfirmableActionKind
    public var targetLabel: String
    public var risk: ActionRisk
    public var commandPreview: String
    public var requiresConfirmation: Bool

    public var id: String { actionId }

    public init(
        actionId: String,
        kind: ConfirmableActionKind,
        targetLabel: String,
        risk: ActionRisk,
        commandPreview: String,
        requiresConfirmation: Bool = true
    ) {
        self.actionId = actionId
        self.kind = kind
        self.targetLabel = targetLabel
        self.risk = risk
        self.commandPreview = commandPreview
        self.requiresConfirmation = requiresConfirmation
    }
}

public enum DeleteEligibility: Equatable, Codable, Sendable {
    case allowed
    case requiresConfirmation
    case blocked(String)
    case unknown(String)

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        let value = try container.decode(String.self)
        if value == "allowed" {
            self = .allowed
        } else if value == "requires-confirmation" {
            self = .requiresConfirmation
        } else if let reason = value.stripPrefix("blocked: ") {
            self = .blocked(reason)
        } else {
            self = .unknown(value)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .allowed:
            try container.encode("allowed")
        case .requiresConfirmation:
            try container.encode("requires-confirmation")
        case .blocked(let reason):
            try container.encode("blocked: \(reason)")
        case .unknown(let value):
            try container.encode(value)
        }
    }
}

public struct WorktreeRecord: Codable, Equatable, Identifiable, Sendable {
    public var repo: String
    public var path: String
    public var branch: String
    public var isDirty: Bool
    public var hasUnmergedCommits: Bool
    public var lastActivity: String?
    public var diskBytes: UInt64?
    public var deleteEligibility: DeleteEligibility

    public var id: String { path }

    public init(
        repo: String,
        path: String,
        branch: String,
        isDirty: Bool,
        hasUnmergedCommits: Bool,
        lastActivity: String?,
        diskBytes: UInt64?,
        deleteEligibility: DeleteEligibility
    ) {
        self.repo = repo
        self.path = path
        self.branch = branch
        self.isDirty = isDirty
        self.hasUnmergedCommits = hasUnmergedCommits
        self.lastActivity = lastActivity
        self.diskBytes = diskBytes
        self.deleteEligibility = deleteEligibility
    }
}

public struct ProcessRecord: Codable, Equatable, Identifiable, Sendable {
    public var name: String
    public var pid: Int
    public var user: String?
    public var cwd: String?
    public var listeningPorts: [Int]
    public var args: [String]

    public var id: Int { pid }

    public init(
        name: String,
        pid: Int,
        user: String?,
        cwd: String?,
        listeningPorts: [Int],
        args: [String]
    ) {
        self.name = name
        self.pid = pid
        self.user = user
        self.cwd = cwd
        self.listeningPorts = listeningPorts
        self.args = args
    }
}

public enum AgentProvider: String, Codable, Sendable {
    case codex
    case claude
}

public struct AgentSessionRecord: Codable, Equatable, Identifiable, Sendable {
    public var provider: AgentProvider
    public var id: String
    public var title: String
    public var projectPath: String?
    public var lastActivity: String?
    public var resumeCommands: [String]

    public init(
        provider: AgentProvider,
        id: String,
        title: String,
        projectPath: String?,
        lastActivity: String?,
        resumeCommands: [String]
    ) {
        self.provider = provider
        self.id = id
        self.title = title
        self.projectPath = projectPath
        self.lastActivity = lastActivity
        self.resumeCommands = resumeCommands
    }
}

public struct HealthSnapshot: Codable, Equatable, Sendable {
    public var hostname: String?
    public var cwd: String?
    public var home: String?

    public init(hostname: String?, cwd: String?, home: String?) {
        self.hostname = hostname
        self.cwd = cwd
        self.home = home
    }

    public var populatedFieldCount: Int {
        [hostname, cwd, home].filter { value in
            guard let value else {
                return false
            }
            return !value.isEmpty
        }.count
    }
}

public struct SourcePreview: Codable, Equatable, Sendable {
    public var path: String
    public var fileName: String
    public var language: String
    public var content: String
    public var spans: [SourceSyntaxSpan]
    public var lineCount: Int
    public var byteCount: Int

    public init(
        path: String,
        fileName: String,
        language: String,
        content: String,
        spans: [SourceSyntaxSpan],
        lineCount: Int,
        byteCount: Int
    ) {
        self.path = path
        self.fileName = fileName
        self.language = language
        self.content = content
        self.spans = spans
        self.lineCount = lineCount
        self.byteCount = byteCount
    }
}

public struct SourceSyntaxSpan: Codable, Equatable, Sendable {
    public var start: Int
    public var end: Int
    public var role: SourceSyntaxRole

    public init(start: Int, end: Int, role: SourceSyntaxRole) {
        self.start = start
        self.end = end
        self.role = role
    }
}

public enum SourceSyntaxRole: String, Codable, Sendable {
    case keyword
    case type
    case string
    case comment
    case number
}

public enum ClipboardItemKind: String, Codable, Sendable {
    case text
    case file
    case image
    case url
}

public struct ClipboardItem: Codable, Equatable, Identifiable, Sendable {
    public var id: UUID
    public var kind: ClipboardItemKind
    public var preview: String
    public var content: String?
    public var tags: [String]
    public var isPinned: Bool
    public var isSecret: Bool
    public var createdAt: Date

    public init(
        id: UUID = UUID(),
        kind: ClipboardItemKind,
        preview: String,
        content: String? = nil,
        tags: [String],
        isPinned: Bool,
        isSecret: Bool,
        createdAt: Date = Date()
    ) {
        self.id = id
        self.kind = kind
        self.preview = preview
        self.content = content
        self.tags = tags
        self.isPinned = isPinned
        self.isSecret = isSecret
        self.createdAt = createdAt
    }
}

public enum ProfileLaunchTarget: String, Codable, Sendable {
    case terminal
    case ide
    case tmux
}

public struct ProfileCommand: Codable, Equatable, Identifiable, Sendable {
    public var id: UUID
    public var label: String
    public var command: String
    public var target: ProfileLaunchTarget

    public init(
        id: UUID = UUID(),
        label: String,
        command: String,
        target: ProfileLaunchTarget
    ) {
        self.id = id
        self.label = label
        self.command = command
        self.target = target
    }
}

public struct ProjectProfile: Codable, Equatable, Identifiable, Sendable {
    public var id: UUID
    public var name: String
    public var repoPath: String
    public var worktreeRoots: [String]
    public var ports: [Int]
    public var urls: [String]
    public var commands: [ProfileCommand]

    public init(
        id: UUID = UUID(),
        name: String,
        repoPath: String,
        worktreeRoots: [String] = [],
        ports: [Int] = [],
        urls: [String] = [],
        commands: [ProfileCommand] = []
    ) {
        self.id = id
        self.name = name
        self.repoPath = repoPath
        self.worktreeRoots = worktreeRoots
        self.ports = ports
        self.urls = urls
        self.commands = commands
    }
}

public struct DashboardModule: Equatable, Identifiable, Sendable {
    public var id: String
    public var title: String
    public var symbol: String
    public var count: Int
    public var status: String
    public var accent: ModuleAccent

    public init(
        id: String,
        title: String,
        symbol: String,
        count: Int,
        status: String,
        accent: ModuleAccent
    ) {
        self.id = id
        self.title = title
        self.symbol = symbol
        self.count = count
        self.status = status
        self.accent = accent
    }
}

public enum ModuleAccent: String, Sendable {
    case blue
    case green
    case orange
    case red
    case slate
}

private extension String {
    func stripPrefix(_ prefix: String) -> String? {
        hasPrefix(prefix) ? String(dropFirst(prefix.count)) : nil
    }
}
