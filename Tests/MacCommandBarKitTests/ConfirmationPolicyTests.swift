import XCTest
@testable import MacCommandBarKit

final class ConfirmationPolicyTests: XCTestCase {
    func testDestructiveActionsAlwaysRequireExplicitConfirmation() {
        let policy = ConfirmationPolicy()

        let killProcess = ConfirmableAction(
            actionId: "kill-123",
            kind: .killProcess,
            targetLabel: "dotnet pid 123",
            risk: .high,
            commandPreview: "kill -TERM 123"
        )

        let deleteWorktree = ConfirmableAction(
            actionId: "delete-worktree",
            kind: .deleteWorktree,
            targetLabel: "/tmp/repo/worktree",
            risk: .high,
            commandPreview: "git worktree remove /tmp/repo/worktree"
        )

        XCTAssertTrue(policy.requiresConfirmation(killProcess))
        XCTAssertTrue(policy.requiresConfirmation(deleteWorktree))
    }

    func testDirtyWorktreeDeleteIsBlockedBeforeConfirmation() {
        let policy = ConfirmationPolicy()
        let worktree = WorktreeRecord(
            repo: "mac-command-bar",
            path: "/tmp/repo/feature",
            branch: "feature",
            isDirty: true,
            hasUnmergedCommits: false,
            lastActivity: "2026-06-08T22:00:00Z",
            diskBytes: 1_024,
            deleteEligibility: .blocked("dirty worktree")
        )

        XCTAssertEqual(policy.deletionDecision(for: worktree), .blocked("dirty worktree"))
    }
}

