import XCTest
@testable import MacCommandBarKit

@MainActor
final class AppStateSourcePreviewTests: XCTestCase {
    func testRefreshSourceFilesSendsProjectRootAndDecodesFiles() async throws {
        let spy = SourcePreviewCoreClientSpy(responseData: [
            "count": .number(2),
            "files": .array([
                .object([
                    "path": .string("/repo/src/App.svelte"),
                    "relativePath": .string("src/App.svelte"),
                    "fileName": .string("App.svelte"),
                    "language": .string("svelte"),
                    "byteCount": .number(31)
                ]),
                .object([
                    "path": .string("/repo/src/ExternalLogin.cs"),
                    "relativePath": .string("src/ExternalLogin.cs"),
                    "fileName": .string("ExternalLogin.cs"),
                    "language": .string("csharp"),
                    "byteCount": .number(29)
                ])
            ])
        ])
        let state = AppState.sourcePreviewTestState(coreClient: spy)

        await state.refreshSourceFiles(rootPath: "/repo")

        let requests = await spy.recordedRequests()
        XCTAssertEqual(requests.map(\.action), [.sourceList])
        XCTAssertEqual(requests.first?.payload["rootPath"]?.stringValue, "/repo")
        XCTAssertEqual(requests.first?.payload["limit"]?.intValue, 1000)
        XCTAssertEqual(state.sourceFiles.count, 2)
        XCTAssertEqual(state.sourceFiles.map(\.relativePath), ["src/App.svelte", "src/ExternalLogin.cs"])
        XCTAssertEqual(state.modules.first(where: { $0.id == "source" })?.count, 2)
        XCTAssertEqual(state.modules.first(where: { $0.id == "source" })?.status, "Files")
    }

    func testRefreshSourceFilesSendsQueryWhenProvided() async throws {
        let spy = SourcePreviewCoreClientSpy(responseData: [
            "count": .number(1),
            "files": .array([
                .object([
                    "path": .string("/repo/src/TransactionProcessorWorker.cs"),
                    "relativePath": .string("src/TransactionProcessorWorker.cs"),
                    "fileName": .string("TransactionProcessorWorker.cs"),
                    "language": .string("csharp"),
                    "byteCount": .number(41)
                ])
            ])
        ])
        let state = AppState.sourcePreviewTestState(coreClient: spy)

        await state.refreshSourceFiles(rootPath: "/repo", query: " ProcessorWorker.cs ")

        let requests = await spy.recordedRequests()
        XCTAssertEqual(requests.map(\.action), [.sourceList])
        XCTAssertEqual(requests.first?.payload["rootPath"]?.stringValue, "/repo")
        XCTAssertEqual(requests.first?.payload["query"]?.stringValue, "ProcessorWorker.cs")
        XCTAssertEqual(state.sourceFiles.map(\.relativePath), ["src/TransactionProcessorWorker.cs"])
    }

    func testPreviewSourceFileSendsRequestAndDecodesPreview() async throws {
        let spy = SourcePreviewCoreClientSpy(responseData: [
            "path": .string("/repo/Example.cs"),
            "fileName": .string("Example.cs"),
            "language": .string("csharp"),
            "content": .string("public class Example {}"),
            "lineCount": .number(1),
            "byteCount": .number(23),
            "spans": .array([
                .object([
                    "start": .number(0),
                    "end": .number(6),
                    "role": .string("keyword")
                ]),
                .object([
                    "start": .number(13),
                    "end": .number(20),
                    "role": .string("type")
                ]),
                .object([
                    "start": .number(21),
                    "end": .number(24),
                    "role": .string("function")
                ]),
                .object([
                    "start": .number(25),
                    "end": .number(30),
                    "role": .string("parameter")
                ])
            ])
        ])
        let state = AppState.sourcePreviewTestState(coreClient: spy)

        await state.previewSourceFile(path: "/repo/Example.cs")

        let requests = await spy.recordedRequests()
        XCTAssertEqual(requests.map(\.action), [.sourcePreview])
        XCTAssertEqual(requests.first?.payload["path"]?.stringValue, "/repo/Example.cs")
        XCTAssertEqual(
            state.sourcePreview,
            SourcePreview(
                path: "/repo/Example.cs",
                fileName: "Example.cs",
                language: "csharp",
                content: "public class Example {}",
                spans: [
                    SourceSyntaxSpan(start: 0, end: 6, role: .keyword),
                    SourceSyntaxSpan(start: 13, end: 20, role: .type),
                    SourceSyntaxSpan(start: 21, end: 24, role: .function),
                    SourceSyntaxSpan(start: 25, end: 30, role: .parameter)
                ],
                lineCount: 1,
                byteCount: 23
            )
        )
        XCTAssertEqual(state.modules.first(where: { $0.id == "source" })?.count, 1)
        XCTAssertEqual(state.modules.first(where: { $0.id == "source" })?.status, "csharp")
    }

    func testPreviewSourceFileRejectsEmptyPathWithoutCallingCore() async throws {
        let spy = SourcePreviewCoreClientSpy(responseData: [:])
        let state = AppState.sourcePreviewTestState(coreClient: spy)

        await state.previewSourceFile(path: "   ")

        let requests = await spy.recordedRequests()
        XCTAssertEqual(requests, [])
        XCTAssertNil(state.sourcePreview)
        XCTAssertEqual(state.statusMessage, "Source path required")
    }
}

private actor SourcePreviewCoreClientSpy: CoreSending {
    private(set) var requests: [CoreRequest] = []
    private let responseData: [String: JSONValue]

    init(responseData: [String: JSONValue]) {
        self.responseData = responseData
    }

    func send(_ request: CoreRequest) async throws -> CoreResponse {
        requests.append(request)
        return CoreResponse(
            id: request.id,
            ok: true,
            summary: "ok",
            data: responseData,
            warnings: [],
            proposedCommand: nil
        )
    }

    func recordedRequests() -> [CoreRequest] {
        requests
    }
}

private extension AppState {
    static func sourcePreviewTestState(coreClient: any CoreSending) -> AppState {
        AppState(
            modules: [
                DashboardModule(id: "source", title: "Source Preview", symbol: "doc.text.magnifyingglass", count: 0, status: "Read only", accent: .blue)
            ],
            selectedModuleID: "source",
            clipboardVault: ClipboardVaultModel(),
            profiles: [
                ProjectProfile(name: "Test", repoPath: "/repo")
            ],
            statusMessage: "Ready",
            coreClient: coreClient
        )
    }
}
