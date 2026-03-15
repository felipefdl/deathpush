import SwiftUI

enum SidebarItem: String, Hashable, CaseIterable {
	case changes = "Changes"
	case history = "History"
	case explorer = "Explorer"
}

private struct AutoFetchKey: Equatable {
	let enabled: Bool
	let interval: Int
}

struct RepositoryView: View {
	@Environment(AppState.self) private var appState
	@Environment(TabState.self) private var tabState
	@State private var showRecentsSheet = false
	@State private var showWorkspaceSheet = false
	@State private var showGitOutput = false
	@AppStorage("git.autoFetch") private var autoFetch = true
	@AppStorage("git.autoFetchInterval") private var autoFetchInterval = 300

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	var body: some View {
		@Bindable var tab = tabState

		VStack(spacing: 0) {
			NavigationSplitView(columnVisibility: .constant(.all)) {
				SidebarView()
					.toolbar(removing: .sidebarToggle)
					.toolbar {
						ToolbarItem(placement: .automatic) {
							Button("Recents") { showRecentsSheet = true }
						}
						ToolbarItem(placement: .automatic) {
							Button("Workspace") { showWorkspaceSheet = true }
						}
					}
			} detail: {
				VStack(spacing: 0) {
					ZStack {
						GeometryReader { geo in
							let totalHeight = geo.size.height
							let termHeight = max(100, min(totalHeight - 100, totalHeight * tab.terminalFraction))

							VStack(spacing: 0) {
								detailContent
									.frame(maxWidth: .infinity, maxHeight: .infinity)

								if tab.showTerminal {
									Divider()

									Rectangle()
										.fill(Color.clear)
										.frame(height: 6)
										.contentShape(Rectangle())
										.onHover { hovering in
											if hovering {
												NSCursor.resizeUpDown.push()
											} else {
												NSCursor.pop()
											}
										}
										.gesture(
											DragGesture(minimumDistance: 1, coordinateSpace: .named("detailSplit"))
												.onChanged { value in
													let newFraction = 1.0 - (value.location.y / totalHeight)
													tab.terminalFraction = max(0.1, min(0.8, newFraction))
												}
										)
								}

								TerminalPanelView()
									.frame(maxWidth: .infinity)
									.frame(height: termHeight)
									.frame(height: tab.showTerminal ? termHeight : 0, alignment: .top)
									.clipped()
									.allowsHitTesting(tab.showTerminal)
							}
							.coordinateSpace(name: "detailSplit")
						}
					}

					StatusBarView()
				}
			}
			.toolbar {
				ToolbarItem(placement: .principal) {
					ToolbarPillView()
				}

				ToolbarItem(placement: .primaryAction) {
					Button(action: { showGitOutput.toggle() }) {
						Image(systemName: "text.line.last.and.arrowtriangle.forward")
					}
					.help("Git Output")
					.popover(isPresented: $showGitOutput) {
						GitOutputPopoverView()
							.frame(width: 400, height: 350)
					}
				}

				ToolbarItem(placement: .primaryAction) {
					Button(action: { tab.showTerminal.toggle() }) {
						Image(systemName: "apple.terminal.fill")
					}
					.help("Toggle Terminal (Cmd+J)")
					.keyboardShortcut("j", modifiers: .command)
				}
			}
			.safeAreaInset(edge: .top) {
				if repoService?.operationState != .clean {
					MergeBannerView()
				}
			}
		}
		.sheet(isPresented: $showRecentsSheet) {
			RecentsPopoverView()
				.frame(width: 320, height: 400)
		}
		.sheet(isPresented: $showWorkspaceSheet) {
			WorkspacePopoverView()
				.frame(width: 320, height: 400)
		}
		.sheet(isPresented: $tab.showQuickOpen) {
			QuickOpenView { path, line in
				tab.showQuickOpen = false
				if line != nil {
					tab.sidebarSelection = .explorer
					tab.explorerSelectedPath = path
					tab.goToLine = line
				} else {
					tab.selectedFilePaths = [path]
				}
			}
		}
		.sheet(isPresented: $tab.showThemePicker) {
			ThemePickerView()
		}
		.focusedSceneValue(\.showQuickOpen, $tab.showQuickOpen)
		.focusedSceneValue(\.showThemePicker, $tab.showThemePicker)
		.focusedSceneValue(\.showRecents, $showRecentsSheet)
		.focusedSceneValue(\.showWorkspace, $showWorkspaceSheet)
		.focusedSceneValue(\.sidebarSelection, $tab.sidebarSelection)
		.navigationTitle(repoService?.repoName ?? "DeathPush")
		.onAppear {
			try? repoService?.refreshBranches()
			try? repoService?.refreshStashes()
			try? repoService?.refreshTags()
			try? repoService?.refreshLog()
			repoService?.refreshLastCommitInfo()
		}
		.task(id: AutoFetchKey(enabled: autoFetch, interval: autoFetchInterval)) {
			guard autoFetch, autoFetchInterval > 0 else { return }
			while !Task.isCancelled {
				try? await Task.sleep(for: .seconds(autoFetchInterval))
				guard !Task.isCancelled else { break }
				try? repoService?.fetchRemote()
			}
		}
	}

	@ViewBuilder
	private var detailContent: some View {
		ZStack {
			Group {
				if let path = tabState.primarySelectedFilePath {
					DiffDetailView(path: path)
				} else {
					EmptyStateView(
						title: "Select a file",
						subtitle: "Choose a changed file from the sidebar to view its diff.",
						systemImage: "doc.text"
					)
				}
			}
			.opacity(tabState.sidebarSelection == .changes ? 1 : 0)
			.allowsHitTesting(tabState.sidebarSelection == .changes)

			Group {
				if let commitId = tabState.selectedCommitId {
					HistoryDetailView(commitId: commitId)
				} else {
					EmptyStateView(
						title: "Select a commit",
						subtitle: "Choose a commit from the sidebar to view its details.",
						systemImage: "clock.arrow.circlepath"
					)
				}
			}
			.opacity(tabState.sidebarSelection == .history ? 1 : 0)
			.allowsHitTesting(tabState.sidebarSelection == .history)

			Group {
				if let path = tabState.explorerSelectedPath {
					ExplorerDetailView(path: path, goToLine: tabState.goToLine)
				} else {
					EmptyStateView(
						title: "File Explorer",
						subtitle: "Select a file from the explorer to view its contents.",
						systemImage: "folder"
					)
				}
			}
			.opacity(tabState.sidebarSelection == .explorer ? 1 : 0)
			.allowsHitTesting(tabState.sidebarSelection == .explorer)
		}
	}
}
