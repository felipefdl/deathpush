import SwiftUI

private final class WeakRef<T: AnyObject> {
	weak var value: T?
	init(_ value: T) { self.value = value }
}

enum ClipboardOperation {
	case copy, cut
}

struct ExplorerClipboard {
	let paths: [String]
	let operation: ClipboardOperation
}

struct RecentFile: Codable, Equatable, Identifiable {
	var id: String { path }
	let path: String
	let lastOpened: Date
}

@Observable
final class TabState: Identifiable {
	let id = UUID()
	var repoService: RepositoryService?
	let terminalService = TerminalService()

	private static var allInstances: [UUID: WeakRef<TabState>] = [:]

	static var hasActiveTerminals: Bool {
		allInstances.values.compactMap(\.value).contains { !$0.terminalService.sessions.isEmpty }
	}

	init() {
		TabState.allInstances[id] = WeakRef(self)
	}

	deinit {
		TabState.allInstances.removeValue(forKey: id)
	}

	// Sidebar / selection state
	var sidebarSelection: SidebarItem = .changes
	var selectedFilePaths: Set<String> = []
	var explorerSelectedPath: String?
	var selectedCommitId: String?
	var selectedStashIndex: UInt32?

	var primarySelectedFilePath: String? {
		selectedFilePaths.first
	}

	// UI toggles
	var showQuickOpen = false
	var showThemePicker = false
	var showTerminal = false
	var terminalFraction: CGFloat = 0.35
	var goToLine: Int?
	var explorerShowBlame = false
	var explorerShowFileHistory = false

	// Inline blame state
	var currentEditorCursorLine: Int?
	var currentFileBlame: FileBlame?

	var currentLineBlame: String? {
		guard let blame = currentFileBlame,
		      let line = currentEditorCursorLine else { return nil }
		let group = blame.lineGroups.first { line >= Int($0.startLine) && line <= Int($0.endLine) }
		guard let g = group else { return nil }
		let date = DateFormatters.relativeString(from: g.authorDate)
		let summary = g.summary.components(separatedBy: "\n").first ?? g.summary
		return "\(g.authorName), \(date) - \(summary)"
	}

	// Explorer clipboard
	var explorerClipboard: ExplorerClipboard?

	// Recent files
	private(set) var recentFiles: [RecentFile] = []

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
			loadRecentFiles()
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
			loadRecentFiles()
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
			loadRecentFiles()
		} catch {
			errorMessage = error.localizedDescription
		}

		isLoading = false
	}

	func selectFile(_ path: String) {
		selectedFilePaths = [path]
		selectedStashIndex = nil
		addRecentFile(path: path)
	}

	func selectStash(_ index: UInt32) {
		selectedStashIndex = index
		selectedFilePaths = []
	}

	// MARK: - Recent Files

	func addRecentFile(path: String) {
		recentFiles.removeAll { $0.path == path }
		recentFiles.insert(RecentFile(path: path, lastOpened: Date()), at: 0)
		if recentFiles.count > 20 { recentFiles = Array(recentFiles.prefix(20)) }
		saveRecentFiles()
	}

	private func loadRecentFiles() {
		guard let root = repoService?.status?.root else { return }
		let key = "recentFiles.\(root)"
		guard let data = UserDefaults.standard.data(forKey: key),
		      let decoded = try? JSONDecoder().decode([RecentFile].self, from: data) else { return }
		recentFiles = decoded
	}

	private func saveRecentFiles() {
		guard let root = repoService?.status?.root else { return }
		let key = "recentFiles.\(root)"
		if let data = try? JSONEncoder().encode(recentFiles) {
			UserDefaults.standard.set(data, forKey: key)
		}
	}

	func cleanup() {
		closeRepository()
		terminalService.killAllSessions()
	}
}
