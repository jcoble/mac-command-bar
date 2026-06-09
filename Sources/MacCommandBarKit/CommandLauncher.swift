import Foundation

public struct CommandLaunch: Equatable, Sendable {
    public var command: String
    public var target: ProfileLaunchTarget

    public init(command: String, target: ProfileLaunchTarget) {
        self.command = command
        self.target = target
    }
}

public protocol CommandLaunching {
    func launch(_ command: String, target: ProfileLaunchTarget) throws
}

public struct TerminalCommandLauncher: CommandLaunching {
    public init() {}

    public func launch(_ command: String, target: ProfileLaunchTarget) throws {
        let script = """
        tell application "Terminal"
          activate
          do script "\(command.escapingForAppleScript)"
        end tell
        """
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
        process.arguments = ["-e", script]
        try process.run()
    }
}

private extension String {
    var escapingForAppleScript: String {
        replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"")
    }
}
