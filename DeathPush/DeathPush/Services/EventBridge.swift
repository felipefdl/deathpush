import Foundation

final class EventBridge: EventListener, @unchecked Sendable {
	private let gitOutputService: GitOutputService

	init(gitOutputService: GitOutputService) {
		self.gitOutputService = gitOutputService
	}

	nonisolated func onRepositoryChanged(sessionId: String) {
		Task { @MainActor in
			guard let tab = TabRegistry.shared.tabForSession(sessionId),
						let service = tab.repoService,
						service.sessionId == sessionId else { return }
			try? service.refreshStatus()
			service.invalidateFileContentCache()
			service.invalidateExplorerCache()
			service.refreshExplorerExpandedPaths()
		}
	}

	nonisolated func onGitCommand(command: String, durationMs: UInt64, timestamp: String) {
		let service = gitOutputService
		Task { @MainActor in
			service.append(command: command, durationMs: durationMs, timestamp: timestamp)
		}
	}

	nonisolated func onWatcherError(sessionId: String, message: String) {
		Task { @MainActor in
			guard let tab = TabRegistry.shared.tabForSession(sessionId) else { return }
			tab.errorMessage = "Watcher error: \(message)"
		}
	}

	nonisolated func onTerminalData(sessionId: UInt64, data: String) {
		// Will be used for terminal integration later
	}

	nonisolated func onTerminalExit(sessionId: UInt64) {
		// Will be used for terminal integration later
	}
}
