import SwiftUI

struct SCMView: View {
	@Environment(TabState.self) private var tabState
	@Binding var selectedFilePaths: Set<String>
	@State private var showDiscardAllConfirm = false
	@State private var showStashDialog = false
	@State private var stashMessage = ""
	@State private var filterText = ""
	@State private var showUndoCommitConfirm = false
	@State private var showMergePicker = false
	@State private var showRebasePicker = false
	@State private var showCreateTagSheet = false
	@State private var newTagName = ""
	@State private var newTagMessage = ""
	@State private var tagToDelete: TagEntry?
	@State private var tagToDeleteRemote: TagEntry?

	@AppStorage("scm.viewMode") private var viewMode = "list"
	@AppStorage("scm.collapsed.index") private var indexCollapsed = false
	@AppStorage("scm.collapsed.workingTree") private var workingTreeCollapsed = false
	@AppStorage("scm.collapsed.untracked") private var untrackedCollapsed = false
	@AppStorage("scm.collapsed.merge") private var mergeCollapsed = false
	@AppStorage("scm.collapsed.stashes") private var stashesCollapsed = false
	@AppStorage("scm.collapsed.tags") private var tagsCollapsed = false
	@AppStorage("scm.collapsed.nestedRepos") private var nestedReposCollapsed = true
	@AppStorage("scm.collapsed.worktrees") private var worktreesCollapsed = true

	private func isCollapsed(for kind: ResourceGroupKind) -> Binding<Bool> {
		switch kind {
		case .index: $indexCollapsed
		case .workingTree: $workingTreeCollapsed
		case .untracked: $untrackedCollapsed
		case .merge: $mergeCollapsed
		}
	}

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	private var groups: [ResourceGroup] {
		repoService?.resourceGroups ?? []
	}

	private var filteredGroups: [ResourceGroup] {
		guard !filterText.isEmpty else { return groups }
		let query = filterText.lowercased()
		return groups.compactMap { group in
			let filtered = group.files.filter {
				$0.path.lowercased().contains(query)
			}
			guard !filtered.isEmpty else { return nil }
			return ResourceGroup(kind: group.kind, label: group.label, files: filtered)
		}
	}

	var body: some View {
		VStack(spacing: 0) {
			scmToolbar
			CommitInputView()
			fileFilterField

			Divider()

			List(selection: $selectedFilePaths) {
				if filteredGroups.isEmpty && filterText.isEmpty {
					ContentUnavailableView(
						"No Changes",
						systemImage: "checkmark.circle",
						description: Text("Working tree is clean.")
					)
				} else if filteredGroups.isEmpty {
					ContentUnavailableView(
						"No Matches",
						systemImage: "magnifyingglass",
						description: Text("No files match \"\(filterText)\".")
					)
				} else {
					ForEach(filteredGroups, id: \.kind) { group in
						Section {
							if !isCollapsed(for: group.kind).wrappedValue {
								if viewMode == "tree" {
									ResourceTreeView(
										files: group.files,
										groupKind: group.kind,
										selectedFilePaths: $selectedFilePaths,
										contextMenuBuilder: { file in
											AnyView(fileContextMenu(file: file, group: group))
										}
									)
								} else {
									ForEach(group.files, id: \.path) { file in
										ResourceItemView(
											file: file,
											isSelected: selectedFilePaths.contains(file.path),
											onSelect: { selectedFilePaths = [file.path] }
										)
										.tag(file.path)
										.contextMenu {
											fileContextMenu(file: file, group: group)
										}
									}
								}
							}
						} header: {
							resourceGroupHeader(group: group)
						}
					}
				}

				// Stash section
				if let stashes = repoService?.stashes, !stashes.isEmpty {
					Section {
						if !stashesCollapsed {
							ForEach(stashes, id: \.index) { stash in
								HStack {
									Text(stash.message)
										.font(.system(size: 13))
										.lineLimit(1)
									Spacer()
									Text("stash@{\(stash.index)}")
										.font(.system(size: 11, design: .monospaced))
										.foregroundStyle(.tertiary)
								}
								.contentShape(Rectangle())
								.onTapGesture { tabState.selectStash(stash.index) }
								.listRowSeparator(.hidden)
								.listRowInsets(EdgeInsets(top: 1, leading: 8, bottom: 1, trailing: 8))
								.contextMenu {
									Button("View Diff") {
										tabState.selectStash(stash.index)
									}
									Divider()
									Button("Apply") { try? repoService?.applyStash(index: stash.index) }
									Button("Pop") { try? repoService?.popStash(index: stash.index) }
									Divider()
									Button("Drop", role: .destructive) { try? repoService?.dropStash(index: stash.index) }
								}
							}
						}
					} header: {
						stashSectionHeader(count: stashes.count)
					}
				}

				// Tags section
				if let tags = repoService?.tags, !tags.isEmpty {
					Section {
						if !tagsCollapsed {
							ForEach(tags, id: \.name) { tag in
								HStack {
									Image(systemName: "tag")
										.font(.system(size: 11))
										.foregroundStyle(.secondary)
									Text(tag.name)
										.font(.system(size: 13))
										.lineLimit(1)
									Spacer()
									if tag.isAnnotated {
										Text("annotated")
											.font(.system(size: 10))
											.foregroundStyle(.tertiary)
											.glassEffect(.regular)
									}
								}
								.listRowSeparator(.hidden)
								.listRowInsets(EdgeInsets(top: 1, leading: 8, bottom: 1, trailing: 8))
								.contextMenu {
									Button("Push to Remote") {
										try? repoService?.pushTagToRemote(name: tag.name)
									}
									Divider()
									Button("Delete", role: .destructive) {
										tagToDelete = tag
									}
									Button("Delete Remote Tag", role: .destructive) {
										tagToDeleteRemote = tag
									}
								}
							}
						}
					} header: {
						tagsSectionHeader(count: tags.count)
					}
				}

				// Nested repositories section
				if let repos = repoService?.nestedRepos, !repos.isEmpty {
					Section {
						if !nestedReposCollapsed {
							ForEach(repos, id: \.path) { repo in
								NestedRepoRow(repo: repo)
									.listRowSeparator(.hidden)
									.listRowInsets(EdgeInsets(top: 1, leading: 8, bottom: 1, trailing: 8))
									.onTapGesture {
										Task {
											await tabState.openRepository(path: repo.path, appState: _appState)
										}
									}
							}
						}
					} header: {
						nestedReposSectionHeader(count: repos.count)
					}
				}

				// Worktrees section
				if let wts = repoService?.worktrees, wts.count > 1 {
					Section {
						if !worktreesCollapsed {
							ForEach(wts, id: \.path) { wt in
								WorktreeRow(worktree: wt)
									.listRowSeparator(.hidden)
									.listRowInsets(EdgeInsets(top: 1, leading: 8, bottom: 1, trailing: 8))
									.onTapGesture {
										Task {
											await tabState.openRepository(path: wt.path, appState: _appState)
										}
									}
							}
						}
					} header: {
						worktreesSectionHeader(count: wts.count)
					}
				}
			}
			.listStyle(.plain)
			.scrollEdgeEffectStyle(.soft, for: .top)
			.onKeyPress(.space) {
				toggleStageForSelection()
				return .handled
			}
			.onKeyPress(.delete) {
				discardSelection()
				return .handled
			}
			.sheet(isPresented: $showStashDialog) {
				VStack(spacing: 12) {
					Text("Stash Changes")
						.font(.headline)
					TextField("Message (optional)", text: $stashMessage)
						.textFieldStyle(.roundedBorder)
					HStack {
						Button("Cancel") { showStashDialog = false }
							.buttonStyle(.glass)
						Spacer()
						Button("Stash") {
							try? repoService?.saveStash(message: stashMessage.isEmpty ? nil : stashMessage)
							stashMessage = ""
							showStashDialog = false
						}
						.buttonStyle(.glassProminent)
					}
				}
				.padding()
				.frame(width: 350)
			}
			.sheet(isPresented: $showCreateTagSheet) {
				VStack(spacing: 12) {
					Text("Create Tag")
						.font(.headline)
					TextField("Tag name", text: $newTagName)
						.textFieldStyle(.roundedBorder)
					TextField("Message (optional)", text: $newTagMessage)
						.textFieldStyle(.roundedBorder)
					HStack {
						Button("Cancel") {
							newTagName = ""
							newTagMessage = ""
							showCreateTagSheet = false
						}
						.buttonStyle(.glass)
						Spacer()
						Button("Create") {
							let message = newTagMessage.isEmpty ? nil : newTagMessage
							try? repoService?.createNewTag(name: newTagName, message: message)
							newTagName = ""
							newTagMessage = ""
							showCreateTagSheet = false
						}
						.buttonStyle(.glassProminent)
						.disabled(newTagName.trimmingCharacters(in: .whitespaces).isEmpty)
					}
				}
				.padding()
				.frame(width: 350)
			}
			.sheet(isPresented: $showMergePicker) {
				BranchPickerSheet(
					title: "Merge Branch",
					branches: repoService?.branches ?? []
				) { branchName in
					try? repoService?.mergeBranchInto(name: branchName)
				}
			}
			.sheet(isPresented: $showRebasePicker) {
				BranchPickerSheet(
					title: "Rebase onto Branch",
					branches: repoService?.branches ?? []
				) { branchName in
					try? repoService?.rebaseBranchOnto(name: branchName)
				}
			}
			.confirmationDialog(
				"Delete tag \"\(tagToDelete?.name ?? "")\"?",
				isPresented: Binding(
					get: { tagToDelete != nil },
					set: { if !$0 { tagToDelete = nil } }
				),
				titleVisibility: .visible
			) {
				Button("Delete Tag", role: .destructive) {
					if let tag = tagToDelete {
						try? repoService?.removeTag(name: tag.name)
					}
					tagToDelete = nil
				}
			} message: {
				Text("This will delete the local tag. This cannot be undone.")
			}
			.confirmationDialog(
				"Delete remote tag \"\(tagToDeleteRemote?.name ?? "")\"?",
				isPresented: Binding(
					get: { tagToDeleteRemote != nil },
					set: { if !$0 { tagToDeleteRemote = nil } }
				),
				titleVisibility: .visible
			) {
				Button("Delete Remote Tag", role: .destructive) {
					if let tag = tagToDeleteRemote {
						try? repoService?.removeRemoteTag(name: tag.name)
					}
					tagToDeleteRemote = nil
				}
			} message: {
				Text("This will delete the tag from the remote repository. This cannot be undone.")
			}
		}
		.task {
			repoService?.refreshNestedRepos()
			repoService?.refreshWorktrees()
		}
	}

	@Environment(AppState.self) private var _appState

	// MARK: - SCM Toolbar

	@ViewBuilder
	private var scmToolbar: some View {
		HStack(spacing: 0) {
			Button {
				viewMode = viewMode == "list" ? "tree" : "list"
			} label: {
				Image(systemName: viewMode == "list" ? "list.bullet" : "list.bullet.indent")
					.frame(maxWidth: .infinity)
			}
			.buttonStyle(.borderless)
			.controlSize(.small)
			.help(viewMode == "list" ? "View as Tree" : "View as List")

			Button {
				try? repoService?.stageAllFiles()
			} label: {
				Image(systemName: "plus")
					.frame(maxWidth: .infinity)
			}
			.buttonStyle(.borderless)
			.controlSize(.small)
			.help("Stage All")

			Button {
				try? repoService?.refreshStatus()
			} label: {
				Image(systemName: "arrow.clockwise")
					.frame(maxWidth: .infinity)
			}
			.buttonStyle(.borderless)
			.controlSize(.small)
			.help("Refresh")

			syncOrFetchButton

			overflowMenu
		}
		.padding(.horizontal, 8)
		.padding(.vertical, 4)
	}

	@ViewBuilder
	private var syncOrFetchButton: some View {
		let ahead = repoService?.ahead ?? 0
		let behind = repoService?.behind ?? 0

		if ahead > 0 || behind > 0 {
			Button {
				guard let service = repoService else { return }
				Task {
					await service.performOperation("Syncing...") {
						try service.pullRemote()
						try service.pushRemote()
					}
				}
			} label: {
				Image(systemName: "arrow.triangle.2.circlepath")
					.frame(maxWidth: .infinity)
			}
			.buttonStyle(.borderless)
			.controlSize(.small)
			.help("Sync: \(behind)\u{2193} \(ahead)\u{2191}")
			.disabled(repoService?.isLoading == true)
		} else {
			Button {
				guard let service = repoService else { return }
				Task {
					await service.performOperation("Fetching...") {
						try service.fetchRemote()
					}
				}
			} label: {
				Image(systemName: "icloud.and.arrow.down")
					.frame(maxWidth: .infinity)
			}
			.buttonStyle(.borderless)
			.controlSize(.small)
			.help("Fetch")
			.disabled(repoService?.isLoading == true)
		}
	}

	@ViewBuilder
	private var overflowMenu: some View {
		Menu {
			Button(viewMode == "list" ? "View as Tree" : "View as List") {
				viewMode = viewMode == "list" ? "tree" : "list"
			}

			Divider()

			Button("Pull") {
				guard let service = repoService else { return }
				Task { await service.performOperation("Pulling...") { try service.pullRemote() } }
			}
			Button("Pull (Rebase)") {
				guard let service = repoService else { return }
				Task { await service.performOperation("Pulling...") { try service.pullRemote(rebase: true) } }
			}
			Button("Push") {
				guard let service = repoService else { return }
				Task { await service.performOperation("Pushing...") { try service.pushRemote() } }
			}
			Button("Push (Force)") {
				guard let service = repoService else { return }
				Task { await service.performOperation("Pushing...") { try service.pushRemote(force: true) } }
			}
			Button("Fetch") {
				guard let service = repoService else { return }
				Task { await service.performOperation("Fetching...") { try service.fetchRemote() } }
			}
			Button("Sync") {
				guard let service = repoService else { return }
				Task {
					await service.performOperation("Syncing...") {
						try service.pullRemote()
						try service.pushRemote()
					}
				}
			}

			Divider()

			Button("Stage All") { try? repoService?.stageAllFiles() }
			Button("Unstage All") { try? repoService?.unstageAllFiles() }
			Button("Discard All...") { showDiscardAllConfirm = true }

			Divider()

			Button("Stash...") { showStashDialog = true }
			Button("Stash (Include Untracked)") { try? repoService?.saveStashIncludeUntracked() }
			Button("Stash Staged Only") { try? repoService?.saveStashStagedOnly() }
			Button("Stash Pop (Latest)") { try? repoService?.popStash(index: 0) }

			Divider()

			Button("Merge Branch...") {
				try? repoService?.refreshBranches()
				showMergePicker = true
			}
			Button("Rebase Branch...") {
				try? repoService?.refreshBranches()
				showRebasePicker = true
			}

			Divider()

			Button("Undo Last Commit...") { showUndoCommitConfirm = true }
		} label: {
			Image(systemName: "ellipsis")
				.frame(maxWidth: .infinity)
		}
		.buttonStyle(.borderless)
		.controlSize(.small)
		.menuIndicator(.hidden)
		.confirmationDialog(
			"Discard all changes?",
			isPresented: $showDiscardAllConfirm,
			titleVisibility: .visible
		) {
			Button("Discard All", role: .destructive) {
				let paths = groups.filter { $0.kind == .workingTree }.flatMap(\.files).map(\.path)
				try? repoService?.discardFileChanges(paths)
			}
		} message: {
			Text("This will permanently discard all working tree changes. This cannot be undone.")
		}
		.confirmationDialog(
			"Undo last commit?",
			isPresented: $showUndoCommitConfirm,
			titleVisibility: .visible
		) {
			Button("Undo Last Commit", role: .destructive) {
				try? repoService?.undoCommit()
			}
		} message: {
			Text("This will undo the last commit and move changes back to the working tree.")
		}
	}

	// MARK: - File Filter

	@ViewBuilder
	private var fileFilterField: some View {
		HStack {
			Image(systemName: "magnifyingglass")
				.foregroundStyle(.tertiary)
			TextField("Filter files...", text: $filterText)
				.textFieldStyle(.plain)
			if !filterText.isEmpty {
				Button { filterText = "" } label: {
					Image(systemName: "xmark.circle.fill")
				}
				.buttonStyle(.plain)
				.foregroundStyle(.tertiary)
			}
		}
		.padding(.horizontal, 8)
		.padding(.vertical, 4)
	}

	// MARK: - Section Headers

	@ViewBuilder
	private func resourceGroupHeader(group: ResourceGroup) -> some View {
		let collapsed = isCollapsed(for: group.kind)

		GlassEffectContainer(spacing: 4) {
			HStack {
				Button {
					withAnimation(.easeInOut(duration: 0.15)) {
						collapsed.wrappedValue.toggle()
					}
				} label: {
					HStack(spacing: 4) {
						Image(systemName: "chevron.right")
							.font(.caption2)
							.rotationEffect(.degrees(collapsed.wrappedValue ? 0 : 90))

						Text(group.label)
							.font(.caption.bold())
							.foregroundStyle(.secondary)

						Text("\(group.files.count)")
							.font(.caption)
							.glassEffect(.regular)
					}
					.contentShape(Rectangle())
				}
				.buttonStyle(.plain)

				Spacer()

				if group.kind == .workingTree || group.kind == .untracked {
					Button(action: {
						let paths = group.files.map(\.path)
						try? repoService?.stageFiles(paths)
					}) {
						Image(systemName: "plus")
					}
					.buttonStyle(.glass)
					.controlSize(.small)
					.help("Stage All")
				}

				if group.kind == .index {
					Button(action: {
						let paths = group.files.map(\.path)
						try? repoService?.unstageFiles(paths)
					}) {
						Image(systemName: "minus")
					}
					.buttonStyle(.glass)
					.controlSize(.small)
					.help("Unstage All")
				}

				if group.kind == .workingTree {
					Button(action: { showDiscardAllConfirm = true }) {
						Image(systemName: "trash")
					}
					.buttonStyle(.glass)
					.tint(.red)
					.controlSize(.small)
					.help("Discard All Changes")
				}
			}
		}
	}

	@ViewBuilder
	private func stashSectionHeader(count: Int) -> some View {
		GlassEffectContainer(spacing: 4) {
			HStack {
				Button {
					withAnimation(.easeInOut(duration: 0.15)) {
						stashesCollapsed.toggle()
					}
				} label: {
					HStack(spacing: 4) {
						Image(systemName: "chevron.right")
							.font(.caption2)
							.rotationEffect(.degrees(stashesCollapsed ? 0 : 90))

						Text("Stashes")
							.font(.caption.bold())
							.foregroundStyle(.secondary)

						Text("\(count)")
							.font(.caption)
							.glassEffect(.regular)
					}
					.contentShape(Rectangle())
				}
				.buttonStyle(.plain)

				Spacer()

				Button(action: { showStashDialog = true }) {
					Image(systemName: "tray.and.arrow.down")
				}
				.buttonStyle(.glass)
				.controlSize(.small)
				.help("Stash Changes")
			}
		}
	}

	@ViewBuilder
	private func tagsSectionHeader(count: Int) -> some View {
		GlassEffectContainer(spacing: 4) {
			HStack {
				Button {
					withAnimation(.easeInOut(duration: 0.15)) {
						tagsCollapsed.toggle()
					}
				} label: {
					HStack(spacing: 4) {
						Image(systemName: "chevron.right")
							.font(.caption2)
							.rotationEffect(.degrees(tagsCollapsed ? 0 : 90))

						Text("Tags")
							.font(.caption.bold())
							.foregroundStyle(.secondary)

						Text("\(count)")
							.font(.caption)
							.glassEffect(.regular)
					}
					.contentShape(Rectangle())
				}
				.buttonStyle(.plain)

				Spacer()

				Button(action: {
					try? repoService?.refreshTags()
					showCreateTagSheet = true
				}) {
					Image(systemName: "plus")
				}
				.buttonStyle(.glass)
				.controlSize(.small)
				.help("Create Tag")
			}
		}
	}

	@ViewBuilder
	private func nestedReposSectionHeader(count: Int) -> some View {
		GlassEffectContainer(spacing: 4) {
			HStack {
				Button {
					withAnimation(.easeInOut(duration: 0.15)) {
						nestedReposCollapsed.toggle()
					}
				} label: {
					HStack(spacing: 4) {
						Image(systemName: "chevron.right")
							.font(.caption2)
							.rotationEffect(.degrees(nestedReposCollapsed ? 0 : 90))

						Text("Nested Repositories")
							.font(.caption.bold())
							.foregroundStyle(.secondary)

						Text("\(count)")
							.font(.caption)
							.glassEffect(.regular)
					}
					.contentShape(Rectangle())
				}
				.buttonStyle(.plain)

				Spacer()
			}
		}
	}

	@ViewBuilder
	private func worktreesSectionHeader(count: Int) -> some View {
		GlassEffectContainer(spacing: 4) {
			HStack {
				Button {
					withAnimation(.easeInOut(duration: 0.15)) {
						worktreesCollapsed.toggle()
					}
				} label: {
					HStack(spacing: 4) {
						Image(systemName: "chevron.right")
							.font(.caption2)
							.rotationEffect(.degrees(worktreesCollapsed ? 0 : 90))

						Text("Worktrees")
							.font(.caption.bold())
							.foregroundStyle(.secondary)

						Text("\(count)")
							.font(.caption)
							.glassEffect(.regular)
					}
					.contentShape(Rectangle())
				}
				.buttonStyle(.plain)

				Spacer()
			}
		}
	}

	// MARK: - Context Menu

	private var selectedPathsArray: [String] {
		Array(selectedFilePaths)
	}

	@ViewBuilder
	private func fileContextMenu(file: FileEntry, group: ResourceGroup) -> some View {
		let paths = selectedFilePaths.count > 1 ? selectedPathsArray : [file.path]
		let label = selectedFilePaths.count > 1 ? " (\(selectedFilePaths.count) files)" : ""

		if group.kind == .workingTree || group.kind == .untracked {
			Button("Stage\(label)") {
				try? repoService?.stageFiles(paths)
			}
		}
		if group.kind == .index {
			Button("Unstage\(label)") {
				try? repoService?.unstageFiles(paths)
			}
		}
		if group.kind == .workingTree {
			Divider()
			Button("Discard Changes\(label)", role: .destructive) {
				try? repoService?.discardFileChanges(paths)
			}
		}
		Divider()
		Button("Reveal in Finder") {
			try? revealInFileManager(
				sessionId: repoService?.sessionId ?? "",
				path: file.path
			)
		}
		Button("Copy Path") {
			NSPasteboard.general.clearContents()
			NSPasteboard.general.setString(file.path, forType: .string)
		}
	}

	// MARK: - Keyboard Actions

	private func toggleStageForSelection() {
		guard !selectedFilePaths.isEmpty else { return }
		let indexPaths = Set(groups.filter { $0.kind == .index }.flatMap(\.files).map(\.path))
		let selectedArray = Array(selectedFilePaths)

		if selectedArray.allSatisfy({ indexPaths.contains($0) }) {
			try? repoService?.unstageFiles(selectedArray)
		} else {
			try? repoService?.stageFiles(selectedArray)
		}
	}

	private func discardSelection() {
		guard !selectedFilePaths.isEmpty else { return }
		let workingPaths = Set(groups.filter { $0.kind == .workingTree }.flatMap(\.files).map(\.path))
		let toDiscard = Array(selectedFilePaths.filter { workingPaths.contains($0) })
		guard !toDiscard.isEmpty else { return }
		try? repoService?.discardFileChanges(toDiscard)
	}
}
