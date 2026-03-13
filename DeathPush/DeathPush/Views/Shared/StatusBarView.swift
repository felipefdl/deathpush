import SwiftUI

struct StatusBarView: View {
	@Environment(TabState.self) private var tabState
	@State private var showBranchPicker = false

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	private var syncLabel: String {
		let ahead = repoService?.ahead ?? 0
		let behind = repoService?.behind ?? 0
		guard ahead > 0 || behind > 0 else { return "" }
		var parts: [String] = []
		if behind > 0 { parts.append("\(behind)\u{2193}") }
		if ahead > 0 { parts.append("\(ahead)\u{2191}") }
		return parts.joined(separator: " ")
	}

	var body: some View {
		VStack(spacing: 0) {
			Divider()

			HStack(spacing: 0) {
				// Branch picker with inline sync label
				Button(action: { showBranchPicker.toggle() }) {
					HStack(spacing: 4) {
						Image(systemName: "arrow.triangle.branch")
						Text(repoService?.currentBranch ?? "No branch")
						if !syncLabel.isEmpty {
							Text(syncLabel)
						}
					}
				}
				.buttonStyle(.plain)
				.popover(isPresented: $showBranchPicker) {
					BranchPickerView()
						.frame(width: 350, height: 400)
				}

				Spacer()

				// Last commit info
				if let commit = repoService?.lastCommitInfo {
					HStack(spacing: 4) {
						Image(systemName: "dot.radiowaves.right")
						Text(commit.message)
							.lineLimit(1)
							.truncationMode(.tail)
						Text(formatRelativeDate(commit.authorDate))
							.foregroundStyle(.secondary)
					}
				}
			}
			.font(.caption)
			.padding(.horizontal, 8)
			.frame(height: 22)
		}
		.background(.bar)
	}
}

private func formatRelativeDate(_ isoDate: String) -> String {
	let formatter = ISO8601DateFormatter()
	formatter.formatOptions = [.withInternetDateTime]
	guard let date = formatter.date(from: isoDate) else { return "" }

	let seconds = Int(Date().timeIntervalSince(date))
	if seconds < 60 { return "just now" }

	let minute = 60
	let hour = 60 * minute
	let day = 24 * hour
	let week = 7 * day
	let month = 30 * day
	let year = 365 * day

	if seconds < hour {
		let m = seconds / minute
		return m == 1 ? "1 minute ago" : "\(m) minutes ago"
	}
	if seconds < day {
		let h = seconds / hour
		return h == 1 ? "1 hour ago" : "\(h) hours ago"
	}
	if seconds < week {
		let d = seconds / day
		return d == 1 ? "1 day ago" : "\(d) days ago"
	}
	if seconds < month {
		let w = seconds / week
		return w == 1 ? "1 week ago" : "\(w) weeks ago"
	}
	if seconds < year {
		let mo = seconds / month
		return mo == 1 ? "1 month ago" : "\(mo) months ago"
	}
	let y = seconds / year
	return y == 1 ? "1 year ago" : "\(y) years ago"
}

struct BranchPickerView: View {
	@Environment(TabState.self) private var tabState
	@State private var searchText = ""

	// Branch management state
	@State private var branchToDelete: BranchEntry?
	@State private var showDeleteConfirm = false
	@State private var branchToDeleteRemote: BranchEntry?
	@State private var showDeleteRemoteConfirm = false
	@State private var branchToRename: BranchEntry?
	@State private var showRenameSheet = false
	@State private var renameText = ""

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	private var filteredBranches: [BranchEntry] {
		let all = repoService?.branches ?? []
		if searchText.isEmpty { return all }
		return all.filter { $0.name.localizedCaseInsensitiveContains(searchText) }
	}

	var body: some View {
		VStack(spacing: 0) {
			HStack {
				Image(systemName: "magnifyingglass")
					.foregroundStyle(.secondary)
				TextField("Filter branches...", text: $searchText)
					.textFieldStyle(.plain)
			}
			.padding(10)

			Divider()

			List {
				Section("Local") {
					ForEach(filteredBranches.filter { !$0.isRemote }, id: \.name) { branch in
						Button(action: {
							Task { try? repoService?.switchBranch(name: branch.name) }
						}) {
							HStack {
								if branch.isHead {
									Image(systemName: "checkmark")
										.foregroundStyle(.green)
								}
								Text(branch.name)
									.font(.body)
								Spacer()
								if let upstream = branch.upstream {
									Text(upstream)
										.font(.caption)
										.foregroundStyle(.tertiary)
								}
							}
						}
						.buttonStyle(.plain)
						.contextMenu {
							if !branch.isHead {
								Button {
									branchToRename = branch
									renameText = branch.name
									showRenameSheet = true
								} label: {
									Label("Rename Branch...", systemImage: "pencil")
								}

								Divider()

								Button(role: .destructive) {
									branchToDelete = branch
									showDeleteConfirm = true
								} label: {
									Label("Delete Branch...", systemImage: "trash")
								}
							}
						}
					}
				}

				Section("Remote") {
					ForEach(filteredBranches.filter { $0.isRemote }, id: \.name) { branch in
						HStack {
							Text(branch.name)
								.font(.body)
						}
						.contextMenu {
							Button(role: .destructive) {
								branchToDeleteRemote = branch
								showDeleteRemoteConfirm = true
							} label: {
								Label("Delete Remote Branch...", systemImage: "trash")
							}
						}
					}
				}
			}
			.listStyle(.plain)
			.scrollEdgeEffectStyle(.soft, for: .top)
		}
		.onAppear {
			try? repoService?.refreshBranches()
		}
		.confirmationDialog(
			"Delete branch \"\(branchToDelete?.name ?? "")\"?",
			isPresented: $showDeleteConfirm,
			titleVisibility: .visible
		) {
			Button("Delete", role: .destructive) {
				guard let branch = branchToDelete else { return }
				try? repoService?.removeBranch(name: branch.name)
				try? repoService?.refreshBranches()
			}
			Button("Force Delete", role: .destructive) {
				guard let branch = branchToDelete else { return }
				try? repoService?.removeBranch(name: branch.name, force: true)
				try? repoService?.refreshBranches()
			}
		} message: {
			Text("This will delete the local branch. Use \"Force Delete\" if the branch has unmerged changes.")
		}
		.confirmationDialog(
			"Delete remote branch \"\(branchToDeleteRemote?.name ?? "")\"?",
			isPresented: $showDeleteRemoteConfirm,
			titleVisibility: .visible
		) {
			Button("Delete Remote Branch", role: .destructive) {
				guard let branch = branchToDeleteRemote else { return }
				let parts = branch.name.split(separator: "/", maxSplits: 1)
				let remote = parts.count > 1 ? String(parts[0]) : "origin"
				let branchName = parts.count > 1 ? String(parts[1]) : branch.name
				try? repoService?.removeRemoteBranch(remote: remote, name: branchName)
				try? repoService?.refreshBranches()
			}
		} message: {
			Text("This will delete the branch from the remote. Other collaborators will no longer see it. This cannot be undone.")
		}
		.sheet(isPresented: $showRenameSheet) {
			BranchRenameSheet(
				oldName: branchToRename?.name ?? "",
				newName: $renameText
			) { newName in
				guard let branch = branchToRename else { return }
				try? repoService?.renameBranchTo(oldName: branch.name, newName: newName)
				try? repoService?.refreshBranches()
				showRenameSheet = false
			}
		}
	}
}

struct BranchRenameSheet: View {
	let oldName: String
	@Binding var newName: String
	let onRename: (String) -> Void

	@Environment(\.dismiss) private var dismiss
	@FocusState private var isTextFieldFocused: Bool

	private var isValid: Bool {
		let trimmed = newName.trimmingCharacters(in: .whitespaces)
		return !trimmed.isEmpty && trimmed != oldName
	}

	var body: some View {
		VStack(spacing: 16) {
			Text("Rename Branch")
				.font(.headline)

			VStack(alignment: .leading, spacing: 6) {
				Text("Current name:")
					.font(.caption)
					.foregroundStyle(.secondary)
				Text(oldName)
					.font(.body.monospaced())

				Text("New name:")
					.font(.caption)
					.foregroundStyle(.secondary)
					.padding(.top, 4)
				TextField("New branch name", text: $newName)
					.textFieldStyle(.roundedBorder)
					.focused($isTextFieldFocused)
					.onSubmit {
						if isValid {
							onRename(newName.trimmingCharacters(in: .whitespaces))
						}
					}
			}

			HStack {
				Spacer()
				Button("Cancel") { dismiss() }
					.buttonStyle(.glass)
				Button("Rename") {
					onRename(newName.trimmingCharacters(in: .whitespaces))
				}
				.buttonStyle(.glassProminent)
				.disabled(!isValid)
			}
		}
		.padding(20)
		.frame(width: 320)
		.onAppear {
			isTextFieldFocused = true
		}
	}
}
