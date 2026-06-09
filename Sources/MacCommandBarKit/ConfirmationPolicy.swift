import Foundation

public struct ConfirmationPolicy: Sendable {
    public init() {}

    public func requiresConfirmation(_ action: ConfirmableAction) -> Bool {
        action.requiresConfirmation || action.risk == .high || action.kind != .runCommand
    }

    public func deletionDecision(for worktree: WorktreeRecord) -> DeleteEligibility {
        switch worktree.deleteEligibility {
        case .blocked(let reason):
            return .blocked(reason)
        case .requiresConfirmation, .allowed, .unknown:
            break
        }

        if worktree.isDirty {
            return .blocked("dirty worktree")
        }
        if worktree.hasUnmergedCommits {
            return .blocked("unmerged commits")
        }
        return .requiresConfirmation
    }
}

