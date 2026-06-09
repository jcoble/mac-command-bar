import XCTest
@testable import MacCommandBarKit

final class SourceFileTreeTests: XCTestCase {
    func testBuildGroupsFilesIntoSortedDirectoryTree() {
        let files = [
            SourceFileRecord(
                path: "/repo/src/App.svelte",
                relativePath: "src/App.svelte",
                fileName: "App.svelte",
                language: "svelte",
                byteCount: 10
            ),
            SourceFileRecord(
                path: "/repo/src/Workers/TransactionProcessorWorker.cs",
                relativePath: "src/Workers/TransactionProcessorWorker.cs",
                fileName: "TransactionProcessorWorker.cs",
                language: "csharp",
                byteCount: 20
            ),
            SourceFileRecord(
                path: "/repo/Package.swift",
                relativePath: "Package.swift",
                fileName: "Package.swift",
                language: "swift",
                byteCount: 30
            )
        ]

        let tree = SourceFileTreeNode.build(from: files)

        XCTAssertEqual(tree.map(\.name), ["src", "Package.swift"])
        XCTAssertTrue(tree[0].isFolder)
        XCTAssertEqual(tree[0].children.map(\.name), ["Workers", "App.svelte"])
        XCTAssertTrue(tree[0].children[0].isFolder)
        XCTAssertEqual(tree[0].children[0].children.map(\.name), ["TransactionProcessorWorker.cs"])
        XCTAssertEqual(
            tree[0].children[0].children[0].file?.path,
            "/repo/src/Workers/TransactionProcessorWorker.cs"
        )
    }

    func testSelectingFileDoesNotOverwriteFilterOrHideSiblingFiles() {
        let files = [
            SourceFileRecord(
                path: "/repo/src/App.svelte",
                relativePath: "src/App.svelte",
                fileName: "App.svelte",
                language: "svelte",
                byteCount: 10
            ),
            SourceFileRecord(
                path: "/repo/src/Workers/TransactionProcessorWorker.cs",
                relativePath: "src/Workers/TransactionProcessorWorker.cs",
                fileName: "TransactionProcessorWorker.cs",
                language: "csharp",
                byteCount: 20
            )
        ]
        var browser = SourcePreviewBrowserState(filterText: "")

        browser.select(files[0])

        XCTAssertEqual(browser.filterText, "")
        XCTAssertEqual(browser.filteredFiles(from: files).map(\.relativePath), [
            "src/App.svelte",
            "src/Workers/TransactionProcessorWorker.cs"
        ])
        XCTAssertEqual(
            browser.previewTargetPath(repoPath: "/repo", files: files),
            "/repo/src/App.svelte"
        )

        browser.select(files[1])

        XCTAssertEqual(browser.filterText, "")
        XCTAssertEqual(browser.filteredFiles(from: files).map(\.relativePath), [
            "src/App.svelte",
            "src/Workers/TransactionProcessorWorker.cs"
        ])
        XCTAssertEqual(
            browser.previewTargetPath(repoPath: "/repo", files: files),
            "/repo/src/Workers/TransactionProcessorWorker.cs"
        )
    }
}
