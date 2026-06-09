import XCTest

final class MenuBarPresentationRegressionTests: XCTestCase {
    func testCommandCenterDoesNotHostSwiftUISheetsInsideMenuBarExtraContent() throws {
        let sourceURL = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("Sources/MacCommandBar/CommandCenterView.swift")

        let source = try String(contentsOf: sourceURL, encoding: .utf8)

        XCTAssertFalse(
            source.contains(".sheet("),
            "CommandCenterView is hosted inside MenuBarExtra; use in-window overlays instead of SwiftUI sheets so focus stays in the menu-bar window."
        )
    }
}
