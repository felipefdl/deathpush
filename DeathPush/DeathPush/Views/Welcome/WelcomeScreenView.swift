import SwiftUI

// MARK: - Welcome Filter Field

enum WelcomeFilterField: Hashable {
	case recent, workspace
}

// MARK: - Focused Value for Menu Integration

struct FocusWelcomeFilterKey: FocusedValueKey {
	typealias Value = (WelcomeFilterField) -> Void
}

extension FocusedValues {
	var focusWelcomeFilter: ((WelcomeFilterField) -> Void)? {
		get { self[FocusWelcomeFilterKey.self] }
		set { self[FocusWelcomeFilterKey.self] = newValue }
	}
}

// MARK: - Workspace Tree

final class WorkspaceTreeNode: Identifiable {
	let id = UUID()
	let name: String
	var childMap: [String: WorkspaceTreeNode] = [:]
	var projects: [ProjectInfo] = []

	init(name: String) {
		self.name = name
	}

	var sortedChildren: [WorkspaceTreeNode] {
		childMap.values.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
	}

	var sortedProjects: [ProjectInfo] {
		projects.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
	}
}

func buildWorkspaceTree(projects: [ProjectInfo], rootDirectory: String) -> WorkspaceTreeNode {
	let root = WorkspaceTreeNode(name: "")
	let normalizedRoot = rootDirectory.hasSuffix("/") ? String(rootDirectory.dropLast()) : rootDirectory

	for project in projects {
		guard project.path.hasPrefix(normalizedRoot + "/") else { continue }
		let relative = String(project.path.dropFirst(normalizedRoot.count + 1))
		let parts = relative.split(separator: "/").map(String.init)

		var current = root
		for i in 0..<(parts.count - 1) {
			let part = parts[i]
			if current.childMap[part] == nil {
				current.childMap[part] = WorkspaceTreeNode(name: part)
			}
			current = current.childMap[part]!
		}
		current.projects.append(project)
	}

	return root
}

func buildMultiRootWorkspaceTree(projects: [ProjectInfo], workspaces: [WorkspaceEntry]) -> WorkspaceTreeNode {
	let root = WorkspaceTreeNode(name: "")

	let sorted = workspaces
		.map { ws -> (directory: String, scanDepth: Int) in
			let dir = ws.directory.hasSuffix("/") ? String(ws.directory.dropLast()) : ws.directory
			return (dir, ws.scanDepth)
		}
		.sorted { $0.directory.count > $1.directory.count }

	var projectsByWs: [String: [ProjectInfo]] = [:]
	for ws in sorted {
		projectsByWs[ws.directory] = []
	}

	for project in projects {
		if let match = sorted.first(where: { project.path.hasPrefix($0.directory + "/") }) {
			projectsByWs[match.directory]?.append(project)
		}
	}

	for ws in sorted {
		let dirName = ws.directory.split(separator: "/").last.map(String.init) ?? ws.directory
		let wsProjects = projectsByWs[ws.directory] ?? []

		if ws.scanDepth > 1 {
			let subTree = buildWorkspaceTree(projects: wsProjects, rootDirectory: ws.directory)
			let wsNode = WorkspaceTreeNode(name: dirName)
			wsNode.childMap = subTree.childMap
			wsNode.projects = subTree.projects
			root.childMap[ws.directory] = wsNode
		} else {
			let wsNode = WorkspaceTreeNode(name: dirName)
			wsNode.projects = wsProjects
			root.childMap[ws.directory] = wsNode
		}
	}

	return root
}

// MARK: - Welcome Screen

struct WelcomeScreenView: View {
	@Environment(AppState.self) private var appState
	@Environment(TabState.self) private var tabState

	@State private var showCloneSheet = false
	@State private var cloneURL = ""
	@State private var clonePath = ""
	@State private var recentFilter = ""
	@State private var workspaceFilter = ""
	@State private var showWorkspaceConfig = false
	@State private var recentIndex: Int?
	@State private var workspaceIndex: Int?

	@FocusState private var focusedField: WelcomeFilterField?

	var body: some View {
		VStack(spacing: 0) {
			Spacer(minLength: 20)

			// Logo
			Image("DeathPushLogo")
				.resizable()
				.aspectRatio(contentMode: .fit)
				.frame(width: 80, height: 80)
				.foregroundStyle(
					LinearGradient(
						colors: [
							Color(red: 0.91, green: 0.85, blue: 0.74),
							Color(red: 0.69, green: 0.60, blue: 0.47)
						],
						startPoint: .top,
						endPoint: .bottom
					)
				)

			// Title
			Text("DeathPush")
				.font(.largeTitle.bold())
				.foregroundStyle(
					LinearGradient(
						colors: [
							Color(red: 0.91, green: 0.85, blue: 0.74),
							Color(red: 0.69, green: 0.60, blue: 0.47)
						],
						startPoint: .leading,
						endPoint: .trailing
					)
				)
				.padding(.top, 8)

			// Action buttons
			GlassEffectContainer {
				HStack(spacing: 8) {
					Button(action: openRepositoryPicker) {
						Label("Open Repository", systemImage: "folder")
					}
					.buttonStyle(.glass)

					Button(action: { showCloneSheet = true }) {
						Label("Clone Repository", systemImage: "square.and.arrow.down")
					}
					.buttonStyle(.glass)
				}
			}
			.padding(.top, 24)

			// Two columns: Recent | Workspace
			HStack(alignment: .top, spacing: 24) {
				recentColumn
				workspaceColumn
			}
			.padding(.top, 32)

			Spacer(minLength: 20)

			// Version footer
			versionFooter
				.padding(.bottom, 12)
		}
		.frame(maxWidth: 720)
		.frame(minWidth: 700, minHeight: 500)
		.padding(.horizontal, 32)
		.sheet(isPresented: $showCloneSheet) {
			CloneSheetView(url: $cloneURL, path: $clonePath) {
				showCloneSheet = false
				Task { await tabState.cloneRepository(url: cloneURL, path: clonePath, appState: appState) }
			}
		}
		.sheet(isPresented: $showWorkspaceConfig) {
			WorkspaceConfigSheet(
				workspaces: appState.workspaces,
				onSave: { appState.updateWorkspaces($0) }
			)
		}
		.overlay {
			if tabState.isLoading {
				ProgressView("Opening repository...")
					.padding(24)
					.glassEffect(.regular)
			}
		}
		.onReceive(NotificationCenter.default.publisher(for: .cloneRepository)) { _ in
			showCloneSheet = true
		}
		.onChange(of: recentFilter) { recentIndex = nil }
		.onChange(of: workspaceFilter) { workspaceIndex = nil }
		.focusedSceneValue(\.focusWelcomeFilter) { field in
			focusedField = field
		}
	}

	// MARK: - Recent Column

	private var filteredRecents: [ProjectInfo] {
		guard !recentFilter.isEmpty else { return appState.recentProjects }
		return appState.recentProjects.filter {
			$0.name.localizedCaseInsensitiveContains(recentFilter) ||
			$0.path.localizedCaseInsensitiveContains(recentFilter)
		}
	}

	private var recentColumn: some View {
		VStack(alignment: .leading, spacing: 0) {
			sectionHeader("Recent")
			filterField(
				text: $recentFilter,
				placeholder: "Filter recent (\u{2318}1)",
				field: .recent,
				onSubmit: {
					if let idx = recentIndex, idx < filteredRecents.count {
						openProject(filteredRecents[idx].path)
					} else if filteredRecents.count == 1 {
						openProject(filteredRecents[0].path)
					}
				}
			)
			.padding(.bottom, 4)

			ScrollViewReader { proxy in
				listContainer {
					if appState.recentProjects.isEmpty {
						emptyState("No recent projects")
					} else if filteredRecents.isEmpty {
						emptyState("No matching projects")
					} else {
						ForEach(Array(filteredRecents.enumerated()), id: \.element.path) { index, project in
							RecentProjectRow(
								project: project,
								isSelected: recentIndex == index,
								onSelect: { openProject(project.path) },
								onRemove: { appState.removeRecentProject(path: project.path) }
							)
							.id(project.path)
						}
					}
				}
				.onChange(of: recentIndex) { _, newIndex in
					if let idx = newIndex, idx < filteredRecents.count {
						proxy.scrollTo(filteredRecents[idx].path, anchor: .center)
					}
				}
			}
		}
		.frame(maxWidth: .infinity)
		.onKeyPress(.downArrow) {
			guard focusedField == .recent else { return .ignored }
			let max = filteredRecents.count - 1
			guard max >= 0 else { return .ignored }
			recentIndex = min((recentIndex ?? -1) + 1, max)
			return .handled
		}
		.onKeyPress(.upArrow) {
			guard focusedField == .recent, let current = recentIndex else { return .ignored }
			recentIndex = max(current - 1, 0)
			return .handled
		}
		.onKeyPress(.escape) {
			guard focusedField == .recent else { return .ignored }
			focusedField = nil
			recentIndex = nil
			return .handled
		}
	}

	// MARK: - Workspace Column

	private var filteredWorkspace: [ProjectInfo] {
		guard !workspaceFilter.isEmpty else { return appState.workspaceProjects }
		return appState.workspaceProjects.filter {
			$0.name.localizedCaseInsensitiveContains(workspaceFilter) ||
			$0.path.localizedCaseInsensitiveContains(workspaceFilter)
		}
	}

	private var isTreeView: Bool {
		let ws = appState.workspaces
		return (ws.count > 1 || ws.contains { $0.scanDepth > 1 })
			&& workspaceFilter.isEmpty
			&& workspaceIndex == nil
	}

	private var workspaceColumn: some View {
		VStack(alignment: .leading, spacing: 0) {
			sectionHeader("Workspace")
			filterField(
				text: $workspaceFilter,
				placeholder: "Filter workspace (\u{2318}2)",
				field: .workspace,
				onSubmit: {
					if let idx = workspaceIndex, idx < filteredWorkspace.count {
						openProject(filteredWorkspace[idx].path)
					} else if filteredWorkspace.count == 1 {
						openProject(filteredWorkspace[0].path)
					}
				}
			)
			.padding(.bottom, 4)

			ScrollViewReader { proxy in
				listContainer {
					if appState.workspaces.isEmpty {
						emptyState("No workspace directories configured")
					} else if appState.isScanning {
						ProgressView()
							.frame(maxWidth: .infinity)
							.padding(.top, 24)
					} else if appState.workspaceProjects.isEmpty {
						emptyState("No git repositories found")
					} else if filteredWorkspace.isEmpty {
						emptyState("No matching projects")
					} else if isTreeView {
						WorkspaceFolderView(
							node: buildMultiRootWorkspaceTree(
								projects: filteredWorkspace,
								workspaces: appState.workspaces
							),
							depth: 0,
							onSelectProject: { openProject($0) }
						)
					} else {
						ForEach(Array(filteredWorkspace.enumerated()), id: \.element.path) { index, project in
							WorkspaceProjectRow(
								project: project,
								isSelected: workspaceIndex == index
							) {
								openProject(project.path)
							}
							.id(project.path)
						}
					}
				}
				.onChange(of: workspaceIndex) { _, newIndex in
					if let idx = newIndex, idx < filteredWorkspace.count {
						proxy.scrollTo(filteredWorkspace[idx].path, anchor: .center)
					}
				}
			}

			Button("Configure Workspace...") {
				showWorkspaceConfig = true
			}
			.buttonStyle(.plain)
			.font(.caption)
			.foregroundStyle(.secondary)
			.padding(.top, 8)
		}
		.frame(maxWidth: .infinity)
		.onKeyPress(.downArrow) {
			guard focusedField == .workspace else { return .ignored }
			let max = filteredWorkspace.count - 1
			guard max >= 0 else { return .ignored }
			workspaceIndex = min((workspaceIndex ?? -1) + 1, max)
			return .handled
		}
		.onKeyPress(.upArrow) {
			guard focusedField == .workspace, let current = workspaceIndex else { return .ignored }
			workspaceIndex = max(current - 1, 0)
			return .handled
		}
		.onKeyPress(.escape) {
			guard focusedField == .workspace else { return .ignored }
			focusedField = nil
			workspaceIndex = nil
			return .handled
		}
	}

	// MARK: - Shared Components

	private func sectionHeader(_ title: String) -> some View {
		Text(title.uppercased())
			.font(.caption)
			.fontWeight(.semibold)
			.foregroundStyle(.secondary)
			.padding(.bottom, 8)
	}

	private func filterField(
		text: Binding<String>,
		placeholder: String,
		field: WelcomeFilterField,
		onSubmit: @escaping () -> Void = {}
	) -> some View {
		HStack(spacing: 6) {
			Image(systemName: "magnifyingglass")
				.font(.caption)
				.foregroundStyle(.tertiary)
			TextField(placeholder, text: text)
				.textFieldStyle(.plain)
				.font(.callout)
				.focused($focusedField, equals: field)
				.onSubmit(onSubmit)
		}
		.padding(.horizontal, 8)
		.padding(.vertical, 6)
		.background(.quinary)
		.clipShape(RoundedRectangle(cornerRadius: 6))
	}

	private func listContainer<Content: View>(@ViewBuilder content: () -> Content) -> some View {
		ScrollView {
			VStack(spacing: 0) {
				content()
			}
		}
		.scrollEdgeEffectStyle(.soft, for: .top)
		.frame(height: 300)
		.background(.quinary.opacity(0.3))
		.clipShape(RoundedRectangle(cornerRadius: 8))
		.overlay(
			RoundedRectangle(cornerRadius: 8)
				.strokeBorder(.quaternary, lineWidth: 0.5)
		)
	}

	private func emptyState(_ message: String) -> some View {
		Text(message)
			.font(.callout)
			.foregroundStyle(.tertiary)
			.frame(maxWidth: .infinity)
			.padding(.top, 24)
	}

	private var versionFooter: some View {
		let version = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0.0.0"
		return Text("Version \(version)")
			.font(.caption2)
			.foregroundStyle(.tertiary)
	}

	// MARK: - Actions

	private func openRepositoryPicker() {
		let panel = NSOpenPanel()
		panel.canChooseFiles = false
		panel.canChooseDirectories = true
		panel.allowsMultipleSelection = false
		panel.message = "Select a Git repository"
		panel.prompt = "Open"

		if panel.runModal() == .OK, let url = panel.url {
			openProject(url.path(percentEncoded: false))
		}
	}

	private func openProject(_ path: String) {
		Task { await tabState.openRepository(path: path, appState: appState) }
	}
}

// MARK: - Row Views

struct RecentProjectRow: View {
	let project: ProjectInfo
	let isSelected: Bool
	let onSelect: () -> Void
	let onRemove: () -> Void

	@State private var isHovered = false

	var body: some View {
		Button(action: onSelect) {
			HStack(spacing: 8) {
				Image(systemName: "folder.fill")
					.foregroundStyle(.blue)
					.frame(width: 16)
				VStack(alignment: .leading, spacing: 2) {
					Text(project.name)
						.font(.callout)
						.lineLimit(1)
					Text(project.path)
						.font(.caption)
						.foregroundStyle(.secondary)
						.lineLimit(1)
						.truncationMode(.middle)
				}
				Spacer(minLength: 24)
			}
			.padding(.horizontal, 12)
			.padding(.vertical, 6)
			.contentShape(Rectangle())
		}
		.buttonStyle(.plain)
		.overlay(alignment: .trailing) {
			Button(action: onRemove) {
				Image(systemName: "xmark")
					.font(.caption2)
					.foregroundStyle(.secondary)
			}
			.buttonStyle(.plain)
			.padding(.trailing, 12)
			.opacity(isHovered ? 1 : 0)
		}
		.onHover { isHovered = $0 }
		.background((isHovered || isSelected) ? Color.primary.opacity(0.06) : .clear)
	}
}

struct WorkspaceProjectRow: View {
	let project: ProjectInfo
	let isSelected: Bool
	let onSelect: () -> Void

	@State private var isHovered = false

	init(project: ProjectInfo, isSelected: Bool = false, onSelect: @escaping () -> Void) {
		self.project = project
		self.isSelected = isSelected
		self.onSelect = onSelect
	}

	var body: some View {
		Button(action: onSelect) {
			HStack(spacing: 8) {
				Image(systemName: "arrow.triangle.branch")
					.foregroundStyle(.secondary)
					.frame(width: 16)
				VStack(alignment: .leading, spacing: 2) {
					Text(project.name)
						.font(.callout)
						.lineLimit(1)
					Text(project.path)
						.font(.caption)
						.foregroundStyle(.secondary)
						.lineLimit(1)
						.truncationMode(.middle)
				}
				Spacer(minLength: 0)
			}
			.padding(.horizontal, 12)
			.padding(.vertical, 6)
			.contentShape(Rectangle())
		}
		.buttonStyle(.plain)
		.onHover { isHovered = $0 }
		.background((isHovered || isSelected) ? Color.primary.opacity(0.06) : .clear)
	}
}

// MARK: - Tree View

struct WorkspaceFolderView: View {
	let node: WorkspaceTreeNode
	let depth: Int
	let onSelectProject: (String) -> Void

	@State private var isExpanded: Bool

	init(node: WorkspaceTreeNode, depth: Int, onSelectProject: @escaping (String) -> Void) {
		self.node = node
		self.depth = depth
		self.onSelectProject = onSelectProject
		_isExpanded = State(initialValue: node.name.isEmpty)
	}

	var body: some View {
		VStack(spacing: 0) {
			if !node.name.isEmpty {
				folderHeader
			}

			if isExpanded {
				ForEach(node.sortedChildren) { child in
					WorkspaceFolderView(
						node: child,
						depth: node.name.isEmpty ? depth : depth + 1,
						onSelectProject: onSelectProject
					)
				}
				ForEach(node.sortedProjects, id: \.path) { project in
					treeProjectRow(project, depth: node.name.isEmpty ? depth : depth + 1)
				}
			}
		}
	}

	@State private var isFolderHovered = false

	private var folderHeader: some View {
		Button {
			withAnimation(.easeInOut(duration: 0.15)) {
				isExpanded.toggle()
			}
		} label: {
			HStack(spacing: 4) {
				Image(systemName: "chevron.right")
					.font(.caption2)
					.rotationEffect(.degrees(isExpanded ? 90 : 0))
				Image(systemName: "folder.fill")
					.foregroundStyle(.secondary)
				Text(node.name)
					.lineLimit(1)
					.truncationMode(.tail)
				Spacer()
			}
			.font(.callout)
			.padding(.leading, CGFloat(12 + depth * 16))
			.padding(.vertical, 6)
			.contentShape(Rectangle())
		}
		.buttonStyle(.plain)
		.onHover { isFolderHovered = $0 }
		.background(isFolderHovered ? Color.primary.opacity(0.06) : .clear)
	}

	private func treeProjectRow(_ project: ProjectInfo, depth: Int) -> some View {
		TreeProjectRowView(project: project, depth: depth, onSelect: { onSelectProject(project.path) })
	}
}

struct TreeProjectRowView: View {
	let project: ProjectInfo
	let depth: Int
	let onSelect: () -> Void

	@State private var isHovered = false

	var body: some View {
		Button(action: onSelect) {
			HStack(spacing: 8) {
				Image(systemName: "arrow.triangle.branch")
					.foregroundStyle(.secondary)
				Text(project.name)
					.lineLimit(1)
					.truncationMode(.tail)
				Spacer()
			}
			.font(.callout)
			.padding(.leading, CGFloat(12 + depth * 16))
			.padding(.vertical, 6)
			.contentShape(Rectangle())
		}
		.buttonStyle(.plain)
		.onHover { isHovered = $0 }
		.background(isHovered ? Color.primary.opacity(0.06) : .clear)
	}
}

// MARK: - Clone Sheet

struct CloneSheetView: View {
	@Binding var url: String
	@Binding var path: String
	let onClone: () -> Void

	@Environment(\.dismiss) private var dismiss

	var body: some View {
		VStack(spacing: 16) {
			Text("Clone Repository")
				.font(.headline)

			TextField("Repository URL", text: $url)
				.textFieldStyle(.roundedBorder)

			HStack {
				TextField("Destination Path", text: $path)
					.textFieldStyle(.roundedBorder)

				Button("Browse...") {
					let panel = NSOpenPanel()
					panel.canChooseFiles = false
					panel.canChooseDirectories = true
					panel.canCreateDirectories = true
					if panel.runModal() == .OK, let selected = panel.url {
						path = selected.path(percentEncoded: false)
					}
				}
			}

			HStack {
				Button("Cancel") { dismiss() }
					.keyboardShortcut(.cancelAction)

				Spacer()

				Button("Clone") { onClone() }
					.buttonStyle(.glassProminent)
					.tint(.blue)
					.disabled(url.isEmpty || path.isEmpty)
					.keyboardShortcut(.defaultAction)
			}
		}
		.padding(20)
		.frame(width: 450)
	}
}
