import XCTest
@testable import MacCommandBarKit

final class CoreProtocolTests: XCTestCase {
    func testCoreRequestEncodesDryRunAndPayload() throws {
        let request = CoreRequest(
            id: "req-1",
            action: .scanWorktrees,
            dryRun: true,
            payload: [
                "repoPath": .string("/Users/blackcolours/dev/work/EdiPlatform")
            ]
        )

        let data = try JSONEncoder().encode(request)
        let json = String(decoding: data, as: UTF8.self)

        XCTAssertTrue(json.contains("\"id\":\"req-1\""))
        XCTAssertTrue(json.contains("\"action\":\"scan.worktrees\""))
        XCTAssertTrue(json.contains("\"dryRun\":true"))
        XCTAssertTrue(json.contains("\"repoPath\""))
    }

    func testCoreResponseDecodesWarningsAndData() throws {
        let data = Data("""
        {
          "id": "req-1",
          "ok": true,
          "summary": "found 2 worktrees",
          "data": { "count": 2 },
          "warnings": ["one dirty worktree"]
        }
        """.utf8)

        let response = try JSONDecoder().decode(CoreResponse.self, from: data)

        XCTAssertEqual(response.id, "req-1")
        XCTAssertTrue(response.ok)
        XCTAssertEqual(response.summary, "found 2 worktrees")
        XCTAssertEqual(response.warnings, ["one dirty worktree"])
        XCTAssertEqual(response.data["count"], .number(2))
    }
}

