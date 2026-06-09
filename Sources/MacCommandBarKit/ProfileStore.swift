import Foundation

public enum ProfileStoreError: Error, Equatable {
    case malformedLine(String)
    case missingProfileName
}

public enum ProfileStore {
    public static func parse(_ toml: String) throws -> [ProjectProfile] {
        var profiles: [ProjectProfile] = []
        var currentProfile: ProjectProfile?
        var currentCommand: ProfileCommand?

        func flushCommand() {
            guard let command = currentCommand else { return }
            currentProfile?.commands.append(command)
            currentCommand = nil
        }

        func flushProfile() {
            flushCommand()
            guard let profile = currentProfile else { return }
            profiles.append(profile)
            currentProfile = nil
        }

        for rawLine in toml.components(separatedBy: .newlines) {
            let line = rawLine
                .trimmingCharacters(in: .whitespacesAndNewlines)
            if line.isEmpty || line.hasPrefix("#") {
                continue
            }

            if line == "[[profiles]]" {
                flushProfile()
                currentProfile = ProjectProfile(name: "", repoPath: "")
                continue
            }

            if line == "[[profiles.commands]]" {
                flushCommand()
                currentCommand = ProfileCommand(label: "", command: "", target: .terminal)
                continue
            }

            let parts = line.split(separator: "=", maxSplits: 1).map {
                String($0).trimmingCharacters(in: .whitespacesAndNewlines)
            }
            guard parts.count == 2 else {
                throw ProfileStoreError.malformedLine(line)
            }

            if currentCommand != nil {
                switch parts[0] {
                case "label":
                    currentCommand?.label = try parseString(parts[1])
                case "command":
                    currentCommand?.command = try parseString(parts[1])
                case "target":
                    currentCommand?.target = ProfileLaunchTarget(rawValue: try parseString(parts[1])) ?? .terminal
                default:
                    break
                }
            } else if currentProfile != nil {
                switch parts[0] {
                case "name":
                    currentProfile?.name = try parseString(parts[1])
                case "repoPath":
                    currentProfile?.repoPath = try parseString(parts[1])
                case "worktreeRoots":
                    currentProfile?.worktreeRoots = try parseStringArray(parts[1])
                case "ports":
                    currentProfile?.ports = try parseIntArray(parts[1])
                case "urls":
                    currentProfile?.urls = try parseStringArray(parts[1])
                default:
                    break
                }
            }
        }

        flushProfile()

        if profiles.contains(where: { $0.name.isEmpty }) {
            throw ProfileStoreError.missingProfileName
        }

        return profiles
    }

    public static func defaultProfiles(fileManager: FileManager = .default) -> [ProjectProfile] {
        let candidates: [(String, String, [String], [Int], [String])] = [
            (
                "EdiPlatform",
                "/Users/blackcolours/dev/work/EdiPlatform",
                ["/Users/blackcolours/dev/work/worktrees/EdiPlatform"],
                [5173, 5001],
                ["http://localhost:5173"]
            ),
            (
                "Rental Command",
                "/Users/blackcolours/dev/work/rental-management",
                ["/Users/blackcolours/dev/work/worktrees/rental-management"],
                [3000, 8080],
                ["http://localhost:3000"]
            )
        ]

        return candidates
            .filter { fileManager.fileExists(atPath: $0.1) }
            .map { name, repoPath, roots, ports, urls in
                ProjectProfile(
                    name: name,
                    repoPath: repoPath,
                    worktreeRoots: roots,
                    ports: ports,
                    urls: urls,
                    commands: [
                        ProfileCommand(
                            label: "Open repo",
                            command: "cd \(shellQuote(repoPath))",
                            target: .terminal
                        )
                    ]
                )
            }
    }
}

private func parseString(_ value: String) throws -> String {
    let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
    guard trimmed.hasPrefix("\""), trimmed.hasSuffix("\"") else {
        throw ProfileStoreError.malformedLine(value)
    }
    return String(trimmed.dropFirst().dropLast())
}

private func parseStringArray(_ value: String) throws -> [String] {
    let inner = try parseArrayInner(value)
    guard !inner.isEmpty else { return [] }
    return try inner.split(separator: ",").map { try parseString(String($0)) }
}

private func parseIntArray(_ value: String) throws -> [Int] {
    let inner = try parseArrayInner(value)
    guard !inner.isEmpty else { return [] }
    return try inner.split(separator: ",").map {
        let trimmed = String($0).trimmingCharacters(in: .whitespacesAndNewlines)
        guard let value = Int(trimmed) else {
            throw ProfileStoreError.malformedLine(trimmed)
        }
        return value
    }
}

private func parseArrayInner(_ value: String) throws -> String {
    let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
    guard trimmed.hasPrefix("["), trimmed.hasSuffix("]") else {
        throw ProfileStoreError.malformedLine(value)
    }
    return String(trimmed.dropFirst().dropLast())
}

private func shellQuote(_ value: String) -> String {
    "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'"
}

