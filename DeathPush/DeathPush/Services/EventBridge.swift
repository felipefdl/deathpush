import Foundation

final class EventBridge: EventListener, @unchecked Sendable {
	private let gitOutputService: GitOutputService
	private let debounceInterval: Duration = .milliseconds(200)
	private var pendingTasks: [String: Task<Void, Never>] = [:]
	private let lock = NSLock()

	init(gitOutputService: GitOutputService) {
		self.gitOutputService = gitOutputService
	}

	nonisolated func onRepositoryChanged(sessionId: String) {
		// Debounce rapid filesystem events per session
		lock.lock()
		pendingTasks[sessionId]?.cancel()
		let task = Task { @MainActor [weak self] in
			try? await Task.sleep(for: self?.debounceInterval ?? .milliseconds(200))
			guard !Task.isCancelled else { return }
			guard let tab = TabRegistry.shared.tabForSession(sessionId),
						let service = tab.repoService,
						service.sessionId == sessionId else { return }
			service.invalidateFileContentCache()
			try? service.refreshStatus()
			service.refreshExplorerExpandedPaths()
		}
		pendingTasks[sessionId] = task
		lock.unlock()
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
