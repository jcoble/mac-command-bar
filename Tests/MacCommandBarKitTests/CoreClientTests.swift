import Foundation
import XCTest
@testable import MacCommandBarKit

final class CoreClientTests: XCTestCase {
    func testSendDrainsLargeStdoutResponseBeforeWaitingForHelperExit() async throws {
        let tempDirectory = FileManager.default.temporaryDirectory
            .appendingPathComponent("MacCommandBar-CoreClientTests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: tempDirectory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tempDirectory) }

        let helperURL = tempDirectory.appendingPathComponent("large-core-helper.sh")
        let script = """
        #!/bin/sh
        printf '{"id":"large","ok":true,"summary":"ok","data":{"content":"'
        /usr/bin/perl -e 'print "x" x 120000'
        printf '","count":1},"warnings":[],"proposedCommand":null}\\n'
        """
        try script.write(to: helperURL, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: helperURL.path)

        let client = CoreClient(executableURL: helperURL)
        let request = CoreRequest(action: .healthSnapshot, dryRun: true)
        let completion = CoreClientCompletion()
        let task = Task {
            do {
                let response = try await client.send(request)
                await completion.finish(.success(response))
            } catch {
                await completion.finish(.failure(error))
            }
        }
        defer {
            task.cancel()
            terminateHelperProcesses(matching: helperURL.path)
        }

        let deadline = Date().addingTimeInterval(2)
        while await completion.result == nil, Date() < deadline {
            try await Task.sleep(nanoseconds: 50_000_000)
        }

        guard let result = await completion.result else {
            terminateHelperProcesses(matching: helperURL.path)
            XCTFail("CoreClient.send timed out while helper stdout was still unread")
            return
        }

        let response = try result.get()
        XCTAssertTrue(response.ok)
        XCTAssertEqual(response.id, "large")
        XCTAssertEqual(response.data["content"]?.stringValue?.count, 120000)
    }
}

private actor CoreClientCompletion {
    private(set) var result: Result<CoreResponse, Error>?

    func finish(_ result: Result<CoreResponse, Error>) {
        self.result = result
    }
}

private func terminateHelperProcesses(matching path: String) {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/pkill")
    process.arguments = ["-f", path]
    try? process.run()
    process.waitUntilExit()
}
