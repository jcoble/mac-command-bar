import Foundation

public struct ProjectProfileDraft: Equatable, Identifiable, Sendable {
    public var draftID: UUID
    public var profileID: UUID?
    public var name: String
    public var repoPath: String
    public var worktreeRootsText: String
    public var portsText: String
    public var urlsText: String

    public var id: UUID { draftID }

    public init(
        draftID: UUID = UUID(),
        profileID: UUID? = nil,
        name: String = "",
        repoPath: String = "",
        worktreeRootsText: String = "",
        portsText: String = "",
        urlsText: String = ""
    ) {
        self.draftID = draftID
        self.profileID = profileID
        self.name = name
        self.repoPath = repoPath
        self.worktreeRootsText = worktreeRootsText
        self.portsText = portsText
        self.urlsText = urlsText
    }

    public init(profile: ProjectProfile) {
        self.init(
            profileID: profile.id,
            name: profile.name,
            repoPath: profile.repoPath,
            worktreeRootsText: profile.worktreeRoots.joined(separator: "\n"),
            portsText: profile.ports.map(String.init).joined(separator: ", "),
            urlsText: profile.urls.joined(separator: "\n")
        )
    }

    public var trimmedName: String {
        name.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    public var trimmedRepoPath: String {
        repoPath.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    public var worktreeRoots: [String] {
        Self.splitList(worktreeRootsText)
    }

    public var urls: [String] {
        Self.splitList(urlsText)
    }

    public func parsedPorts() -> [Int]? {
        let parts = Self.splitList(portsText)
        guard !parts.isEmpty else {
            return []
        }
        var ports: [Int] = []
        for part in parts {
            guard let port = Int(part), port > 0, port <= 65_535 else {
                return nil
            }
            ports.append(port)
        }
        return ports
    }

    private static func splitList(_ text: String) -> [String] {
        text
            .components(separatedBy: CharacterSet(charactersIn: ",\n"))
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }
}
