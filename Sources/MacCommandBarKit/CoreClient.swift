import Foundation

public enum CoreClientError: Error, LocalizedError {
    case helperUnavailable([String])
    case emptyResponse
    case failed(String)

    public var errorDescription: String? {
        switch self {
        case .helperUnavailable(let attempted):
            return "mcb-core helper was not found. Attempted: \(attempted.joined(separator: ", "))"
        case .emptyResponse:
            return "mcb-core returned an empty response"
        case .failed(let message):
            return message
        }
    }
}

public protocol CoreSending: Sendable {
    func send(_ request: CoreRequest) async throws -> CoreResponse
}

public struct CoreClient: CoreSending {
    public var executableURL: URL

    public init(executableURL: URL) {
        self.executableURL = executableURL
    }

    public static func discover(fileManager: FileManager = .default) throws -> CoreClient {
        var candidates: [URL] = []
        if let override = ProcessInfo.processInfo.environment["MCB_CORE_PATH"], !override.isEmpty {
            candidates.append(URL(fileURLWithPath: override))
        }
        if let bundled = Bundle.main.url(forResource: "mcb-core", withExtension: nil) {
            candidates.append(bundled)
        }

        let cwd = URL(fileURLWithPath: fileManager.currentDirectoryPath)
        candidates.append(cwd.appendingPathComponent("core/target/debug/mcb-core"))
        candidates.append(cwd.appendingPathComponent("core/target/release/mcb-core"))
        candidates.append(cwd.deletingLastPathComponent().appendingPathComponent("core/target/debug/mcb-core"))

        for candidate in candidates where fileManager.isExecutableFile(atPath: candidate.path) {
            return CoreClient(executableURL: candidate)
        }

        throw CoreClientError.helperUnavailable(candidates.map(\.path))
    }

    public func send(_ request: CoreRequest) async throws -> CoreResponse {
        try await Task.detached(priority: .utility) {
            try Self.sendSync(request, executableURL: executableURL)
        }.value
    }

    private static func sendSync(_ request: CoreRequest, executableURL: URL) throws -> CoreResponse {
        let input = try JSONEncoder().encode(request)
        let process = Process()
        process.executableURL = executableURL

        let stdin = Pipe()
        let stdout = Pipe()
        let stderr = Pipe()
        process.standardInput = stdin
        process.standardOutput = stdout
        process.standardError = stderr

        try process.run()
        stdin.fileHandleForWriting.write(input)
        try stdin.fileHandleForWriting.close()
        process.waitUntilExit()

        let output = stdout.fileHandleForReading.readDataToEndOfFile()
        if output.isEmpty {
            let errorData = stderr.fileHandleForReading.readDataToEndOfFile()
            let error = String(decoding: errorData, as: UTF8.self)
            throw CoreClientError.failed(error.isEmpty ? "mcb-core exited without output" : error)
        }

        let response = try JSONDecoder().decode(CoreResponse.self, from: output)
        if process.terminationStatus != 0, !response.ok {
            return response
        }
        return response
    }
}
