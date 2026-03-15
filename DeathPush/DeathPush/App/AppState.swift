import SwiftUI

struct WorkspaceEntry: Codable, Equatable, Identifiable {
	var id: UUID
	var directory: String
	var scanDepth: Int

	init(id: UUID = UUID(), directory: String = "", scanDepth: Int = 1) {
		self.id = id
		self.directory = directory
		self.scanDepth = scanDepth
	}
}

@Observable
final class AppState {
	let themeService = ThemeService()
	let iconThemeService = IconThemeService()
	let gitOutputService = GitOutputService()
	var recentProjects: [ProjectInfo] = []

	// Workspace
	var workspaces: [WorkspaceEntry] = []
	var workspaceProjects: [ProjectInfo] = []
	var isScanning = false

	private let recentProjectsKey = "deathpush.recentProjects"
	private let workspacesKey = "deathpush.workspaces"

	init() {
		loadRecentProjects()
		loadWorkspaces()
		scanWorkspaces()
		themeService.startAppearanceObservation()
	}

	// MARK: - Recent Projects

	func removeRecentProject(path: String) {
		recentProjects.removeAll { $0.path == path }
		saveRecentProjects()
	}

	func addRecentProject(path: String, name: String) {
		recentProjects.removeAll { $0.path == path }
		recentProjects.insert(ProjectInfo(path: path, name: name), at: 0)
		if recentProjects.count > 20 {
			recentProjects = Array(recentProjects.prefix(20))
		}
		saveRecentProjects()
	}

	private func loadRecentProjects() {
		guard let data = UserDefaults.standard.data(forKey: recentProjectsKey),
					let decoded = try? JSONDecoder().decode([RecentProject].self, from: data) else {
			return
		}
		recentProjects = decoded.map { ProjectInfo(path: $0.path, name: $0.name) }
	}

	private func saveRecentProjects() {
		let encoded = recentProjects.map { RecentProject(path: $0.path, name: $0.name) }
		if let data = try? JSONEncoder().encode(encoded) {
			UserDefaults.standard.set(data, forKey: recentProjectsKey)
		}
	}

	// MARK: - Workspaces

	func updateWorkspaces(_ entries: [WorkspaceEntry]) {
		workspaces = entries
		saveWorkspaces()
		scanWorkspaces()
	}

	func scanWorkspaces() {
		guard !workspaces.isEmpty else {
			workspaceProjects = []
			return
		}

		isScanning = true
		let entries = workspaces

		Task {
			let merged: [ProjectInfo] = await Task.detached {
				var seen = Set<String>()
				var results: [ProjectInfo] = []
				for ws in entries {
					guard !ws.directory.isEmpty else { continue }
					if let projects = try? scanProjectsDirectory(path: ws.directory, depth: UInt32(ws.scanDepth)) {
						for project in projects where !seen.contains(project.path) {
							seen.insert(project.path)
							results.append(project)
						}
					}
				}
				results.sort { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
				return results
			}.value

			workspaceProjects = merged
			isScanning = false
		}
	}

	private func loadWorkspaces() {
		guard let data = UserDefaults.standard.data(forKey: workspacesKey),
					let decoded = try? JSONDecoder().decode([WorkspaceEntry].self, from: data) else {
			return
		}
		workspaces = decoded
	}

	private func saveWorkspaces() {
		if let data = try? JSONEncoder().encode(workspaces) {
			UserDefaults.standard.set(data, forKey: workspacesKey)
		}
	}
}

private struct RecentProject: Codable {
	let path: String
	let name: String
}
