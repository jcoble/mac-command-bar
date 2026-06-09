import XCTest
@testable import MacCommandBarKit

final class ProfileStoreTests: XCTestCase {
    func testProfileStoreParsesGlobalProfilesToml() throws {
        let toml = """
        [[profiles]]
        name = "EdiPlatform"
        repoPath = "/Users/blackcolours/dev/work/EdiPlatform"
        worktreeRoots = ["/Users/blackcolours/dev/work/worktrees/EdiPlatform"]
        ports = [5173, 5001]
        urls = ["http://localhost:5173"]

        [[profiles.commands]]
        label = "Start web"
        command = "pnpm --dir ediplatform-web dev --host 127.0.0.1 --port 5173"
        target = "terminal"
        """

        let profiles = try ProfileStore.parse(toml)

        XCTAssertEqual(profiles.count, 1)
        XCTAssertEqual(profiles[0].name, "EdiPlatform")
        XCTAssertEqual(profiles[0].ports, [5173, 5001])
        XCTAssertEqual(profiles[0].commands.first?.label, "Start web")
        XCTAssertEqual(profiles[0].commands.first?.target, .terminal)
    }
}

