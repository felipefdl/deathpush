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
	@State private var showRecentsPopover = false
	@State private var showWorkspacePopover = false
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
							Button(action: { showRecentsPopover.toggle() }) {
								Text("Recents")
							}
							.popover(isPresented: $showRecentsPopover) {
								RecentsPopoverView()
									.frame(width: 320, height: 400)
							}
						}

						ToolbarItem(placement: .automatic) {
							Button(action: { showWorkspacePopover.toggle() }) {
								Text("Workspace")
							}
							.popover(isPresented: $showWorkspacePopover) {
								WorkspacePopoverView()
									.frame(width: 320, height: 400)
							}
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
									.frame(height: tab.showTerminal ? termHeight : 0)
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
		.sheet(isPresented: $tab.showQuickOpen) {
			QuickOpenView { path, line in
				tab.showQuickOpen = false
				if line != nil {
					tab.sidebarSelection = .explorer
					tab.explorerSelectedPath = path
					tab.goToLine = line
				} else {
					tab.selectedFilePath = path
				}
			}
		}
		.sheet(isPresented: $tab.showThemePicker) {
			ThemePickerView()
		}
		.focusedSceneValue(\.showQuickOpen, $tab.showQuickOpen)
		.focusedSceneValue(\.showThemePicker, $tab.showThemePicker)
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
		switch tabState.sidebarSelection {
		case .changes:
			if let path = tabState.selectedFilePath {
				DiffDetailView(path: path)
			} else {
				EmptyStateView(
					title: "Select a file",
					subtitle: "Choose a changed file from the sidebar to view its diff.",
					systemImage: "doc.text"
				)
			}
		case .history:
			if let commitId = tabState.selectedCommitId {
				HistoryDetailView(commitId: commitId)
			} else {
				EmptyStateView(
					title: "Select a commit",
					subtitle: "Choose a commit from the sidebar to view its details.",
					systemImage: "clock.arrow.circlepath"
				)
			}
		case .explorer:
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
	}
}
