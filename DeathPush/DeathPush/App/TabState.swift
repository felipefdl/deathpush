import SwiftUI

@Observable
final class TabState: Identifiable {
	let id = UUID()
	var repoService: RepositoryService?
	let terminalService = TerminalService()

	// Sidebar / selection state
	var sidebarSelection: SidebarItem = .changes
	var selectedFilePath: String?
	var explorerSelectedPath: String?
	var selectedCommitId: String?

	// UI toggles
	var showQuickOpen = false
	var showThemePicker = false
	var showTerminal = false
	var terminalFraction: CGFloat = 0.35
	var goToLine: Int?

	// Lifecycle state
	var isLoading = false
	var errorMessage: String?

	var repoName: String {
		repoService?.repoName ?? "New Tab"
	}

	var repoPath: String? {
		repoService?.status?.root
	}

	// MARK: - Repository Lifecycle

	func openRepository(path: String, appState: AppState) async {
		closeRepository()
		isLoading = true
		errorMessage = nil

		do {
			let service = RepositoryService()
			try service.open(path: path)
			try service.refreshStatus()
			repoService = service
			TabRegistry.shared.register(self)
			appState.addRecentProject(path: path, name: URL(fileURLWithPath: path).lastPathComponent)
		} catch {
			errorMessage = error.localizedDescription
		}

		isLoading = false
	}

	func closeRepository() {
		if let service = repoService {
			TabRegistry.shared.unregister(sessionId: service.sessionId)
			service.destroy()
		}
		repoService = nil
	}

	func initRepository(path: String, appState: AppState) async {
		closeRepository()
		isLoading = true
		errorMessage = nil

		do {
			let service = RepositoryService()
			try service.initRepo(path: path)
			try service.refreshStatus()
			repoService = service
			TabRegistry.shared.register(self)
			appState.addRecentProject(path: path, name: URL(fileURLWithPath: path).lastPathComponent)
		} catch {
			errorMessage = error.localizedDescription
		}

		isLoading = false
	}

	func cloneRepository(url: String, path: String, appState: AppState) async {
		closeRepository()
		isLoading = true
		errorMessage = nil

		do {
			let service = RepositoryService()
			try service.clone(url: url, path: path)
			try service.refreshStatus()
			repoService = service
			TabRegistry.shared.register(self)
			appState.addRecentProject(path: path, name: URL(fileURLWithPath: path).lastPathComponent)
		} catch {
			errorMessage = error.localizedDescription
		}

		isLoading = false
	}

	func cleanup() {
		closeRepository()
		terminalService.killAllSessions()
	}
}
