import SwiftUI

// MARK: - Recents Popover

struct RecentsPopoverView: View {
	@Environment(AppState.self) private var appState
	@Environment(TabState.self) private var tabState
	@State private var filter = ""

	private var filteredRecents: [ProjectInfo] {
		guard !filter.isEmpty else { return appState.recentProjects }
		return appState.recentProjects.filter {
			$0.name.localizedCaseInsensitiveContains(filter) ||
			$0.path.localizedCaseInsensitiveContains(filter)
		}
	}

	var body: some View {
		VStack(spacing: 0) {
			HStack(spacing: 6) {
				Image(systemName: "magnifyingglass")
					.foregroundStyle(.secondary)
				TextField("Filter recent...", text: $filter)
					.textFieldStyle(.plain)
			}
			.padding(10)

			Divider()

			List {
				if appState.recentProjects.isEmpty {
					Text("No recent projects")
						.font(.callout)
						.foregroundStyle(.tertiary)
						.frame(maxWidth: .infinity)
						.padding(.top, 24)
						.listRowSeparator(.hidden)
				} else if filteredRecents.isEmpty {
					Text("No matching projects")
						.font(.callout)
						.foregroundStyle(.tertiary)
						.frame(maxWidth: .infinity)
						.padding(.top, 24)
						.listRowSeparator(.hidden)
				} else {
					ForEach(filteredRecents, id: \.path) { project in
						RecentProjectRow(
							project: project,
							isSelected: false,
							onSelect: { openProject(project.path) },
							onRemove: { appState.removeRecentProject(path: project.path) }
						)
						.listRowInsets(EdgeInsets())
					}
				}
			}
			.listStyle(.plain)
			.scrollEdgeEffectStyle(.soft, for: .top)
		}
	}

	private func openProject(_ path: String) {
		Task { await tabState.openRepository(path: path, appState: appState) }
	}
}

// MARK: - Workspace Popover

struct WorkspacePopoverView: View {
	@Environment(AppState.self) private var appState
	@Environment(TabState.self) private var tabState
	@State private var filter = ""
	@State private var showWorkspaceConfig = false

	private var filteredWorkspace: [ProjectInfo] {
		guard !filter.isEmpty else { return appState.workspaceProjects }
		return appState.workspaceProjects.filter {
			$0.name.localizedCaseInsensitiveContains(filter) ||
			$0.path.localizedCaseInsensitiveContains(filter)
		}
	}

	private var isTreeView: Bool {
		let ws = appState.workspaces
		return (ws.count > 1 || ws.contains { $0.scanDepth > 1 })
			&& filter.isEmpty
	}

	var body: some View {
		VStack(spacing: 0) {
			HStack(spacing: 6) {
				Image(systemName: "magnifyingglass")
					.foregroundStyle(.secondary)
				TextField("Filter workspace...", text: $filter)
					.textFieldStyle(.plain)
			}
			.padding(10)

			Divider()

			List {
				if appState.workspaces.isEmpty {
					Text("No workspace directories configured")
						.font(.callout)
						.foregroundStyle(.tertiary)
						.frame(maxWidth: .infinity)
						.padding(.top, 24)
						.listRowSeparator(.hidden)
				} else if appState.isScanning {
					ProgressView()
						.frame(maxWidth: .infinity)
						.padding(.top, 24)
						.listRowSeparator(.hidden)
				} else if appState.workspaceProjects.isEmpty {
					Text("No git repositories found")
						.font(.callout)
						.foregroundStyle(.tertiary)
						.frame(maxWidth: .infinity)
						.padding(.top, 24)
						.listRowSeparator(.hidden)
				} else if filteredWorkspace.isEmpty {
					Text("No matching projects")
						.font(.callout)
						.foregroundStyle(.tertiary)
						.frame(maxWidth: .infinity)
						.padding(.top, 24)
						.listRowSeparator(.hidden)
				} else if isTreeView {
					WorkspaceFolderView(
						node: buildMultiRootWorkspaceTree(
							projects: filteredWorkspace,
							workspaces: appState.workspaces
						),
						depth: 0,
						onSelectProject: { openProject($0) }
					)
					.listRowInsets(EdgeInsets())
				} else {
					ForEach(filteredWorkspace, id: \.path) { project in
						WorkspaceProjectRow(project: project) {
							openProject(project.path)
						}
						.listRowInsets(EdgeInsets())
					}
				}
			}
			.listStyle(.plain)
			.scrollEdgeEffectStyle(.soft, for: .top)

			Divider()

			Button("Configure Workspace...") {
				showWorkspaceConfig = true
			}
			.buttonStyle(.plain)
			.font(.caption)
			.foregroundStyle(.secondary)
			.padding(8)
			.frame(maxWidth: .infinity, alignment: .leading)
		}
		.sheet(isPresented: $showWorkspaceConfig) {
			WorkspaceConfigSheet(
				workspaces: appState.workspaces,
				onSave: { appState.updateWorkspaces($0) }
			)
		}
	}

	private func openProject(_ path: String) {
		Task { await tabState.openRepository(path: path, appState: appState) }
	}
}
