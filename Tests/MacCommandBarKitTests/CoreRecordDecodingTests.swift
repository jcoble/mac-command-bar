import XCTest
@testable import MacCommandBarKit

final class CoreRecordDecodingTests: XCTestCase {
    func testDecodesProcessRecordsFromCoreJSONValue() throws {
        let value: JSONValue = .array([
            .object([
                "name": .string("node"),
                "pid": .number(456),
                "user": .string("blackcolours"),
                "cwd": .string("/repo"),
                "listeningPorts": .array([.number(5173)]),
                "args": .array([.string("node"), .string("server.js")])
            ])
        ])

        let records: [ProcessRecord] = try value.decode()

        XCTAssertEqual(records.count, 1)
        XCTAssertEqual(records[0].name, "node")
        XCTAssertEqual(records[0].pid, 456)
        XCTAssertEqual(records[0].listeningPorts, [5173])
    }

    func testDecodesAgentSessionRecordsFromCoreJSONValue() throws {
        let value: JSONValue = .array([
            .object([
                "provider": .string("codex"),
                "id": .string("019d"),
                "title": .string("Fix runtime"),
                "projectPath": .string("/repo"),
                "lastActivity": .string("2026-06-08T22:00:00Z"),
                "resumeCommands": .array([.string("codex resume 019d")])
            ])
        ])

        let records: [AgentSessionRecord] = try value.decode()

        XCTAssertEqual(records.count, 1)
        XCTAssertEqual(records[0].provider, .codex)
        XCTAssertEqual(records[0].resumeCommands, ["codex resume 019d"])
    }
}

