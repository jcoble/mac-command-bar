import XCTest
@testable import MacCommandBarKit

@MainActor
final class AppStateSourcePreviewTests: XCTestCase {
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
                    SourceSyntaxSpan(start: 13, end: 20, role: .type)
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
