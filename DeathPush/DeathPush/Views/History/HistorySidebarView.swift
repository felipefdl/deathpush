import SwiftUI

struct HistorySidebarView: View {
	@Binding var selectedCommitId: String?
	@Environment(TabState.self) private var tabState
	@State private var searchText = ""
	@State private var showResetConfirm = false
	@State private var resetCommitId: String?
	@State private var resetCommitMessage: String = ""
	@State private var resetMode: String = "mixed"

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	private var filteredCommits: [CommitEntry] {
		guard let commits = repoService?.commitLog else { return [] }
		guard !searchText.isEmpty else { return commits }
		let query = searchText.lowercased()
		return commits.filter {
			$0.message.lowercased().contains(query)
				|| $0.authorName.lowercased().contains(query)
				|| $0.shortId.lowercased().contains(query)
		}
	}

	var body: some View {
		VStack(spacing: 0) {
			searchField
			Divider()
			commitList
		}
		.onAppear {
			try? repoService?.refreshLog()
		}
	}

	private var searchField: some View {
		HStack(spacing: 6) {
			Image(systemName: "magnifyingglass")
				.foregroundStyle(.tertiary)
				.font(.caption)
			TextField("Filter commits...", text: $searchText)
				.textFieldStyle(.plain)
				.font(.callout)
		}
		.padding(.horizontal, 10)
		.padding(.vertical, 6)
	}

	private var commitList: some View {
		List(selection: $selectedCommitId) {
			ForEach(filteredCommits, id: \.id) { entry in
				CommitSidebarRow(commit: entry)
					.tag(entry.id)
					.contextMenu {
						Button("Copy SHA") {
							NSPasteboard.general.clearContents()
							NSPasteboard.general.setString(entry.id, forType: .string)
						}
						Button("Copy Commit Message") {
							NSPasteboard.general.clearContents()
							NSPasteboard.general.setString(entry.message, forType: .string)
						}

						Divider()

						Button("Cherry Pick") {
							try? repoService?.cherryPickCommit(commitId: entry.id)
						}
						Button("Reset to Commit...") {
							resetCommitId = entry.id
							resetCommitMessage = entry.message.components(separatedBy: "\n").first ?? entry.message
							resetMode = "mixed"
							showResetConfirm = true
						}
					}
			}

			if searchText.isEmpty {
				loadMoreButton
			}
		}
		.listStyle(.plain)
		.scrollEdgeEffectStyle(.soft, for: .top)
		.sheet(isPresented: $showResetConfirm) {
			resetConfirmSheet
		}
	}

	private var resetConfirmSheet: some View {
		VStack(spacing: 16) {
			Text("Reset to Commit")
				.font(.headline)

			Text(resetCommitMessage)
				.font(.callout)
				.foregroundStyle(.secondary)
				.lineLimit(2)
				.frame(maxWidth: .infinity, alignment: .leading)

			Picker("Mode", selection: $resetMode) {
				Text("Soft").tag("soft")
				Text("Mixed").tag("mixed")
				Text("Hard").tag("hard")
			}
			.pickerStyle(.segmented)

			Group {
				switch resetMode {
				case "soft":
					Text("Keeps all changes staged in the index.")
				case "hard":
					Text("Discards all changes permanently. This cannot be undone.")
						.foregroundStyle(.red)
				default:
					Text("Keeps all changes as unstaged modifications.")
				}
			}
			.font(.caption)
			.frame(maxWidth: .infinity, alignment: .leading)

			HStack {
				Button("Cancel") {
					showResetConfirm = false
				}
				.buttonStyle(.glass)

				Spacer()

				Button("Reset", role: .destructive) {
					if let commitId = resetCommitId {
						try? repoService?.resetToCommitId(commitId, mode: resetMode)
					}
					showResetConfirm = false
				}
				.buttonStyle(.glassProminent)
				.tint(.red)
			}
		}
		.padding()
		.frame(width: 380)
	}

	private var loadMoreButton: some View {
		Button {
			try? repoService?.loadMoreCommits()
		} label: {
			Text("Load More")
				.font(.caption)
				.foregroundStyle(.secondary)
				.frame(maxWidth: .infinity, alignment: .center)
				.padding(.vertical, 4)
		}
		.buttonStyle(.plain)
	}
}

struct CommitSidebarRow: View {
	let commit: CommitEntry

	var body: some View {
		HStack(spacing: 8) {
			avatar
			commitInfo
		}
		.padding(.vertical, 2)
	}

	private var avatar: some View {
		CachedAvatarView(url: commit.avatarUrl, fallback: initialsFallback)
			.frame(width: 24, height: 24)
			.clipShape(Circle())
	}

	private var initialsFallback: some View {
		Circle()
			.fill(avatarColor)
			.overlay {
				Text(initials)
					.font(.caption2.bold())
					.foregroundStyle(.white)
			}
	}

	private var commitInfo: some View {
		VStack(alignment: .leading, spacing: 2) {
			HStack(spacing: 4) {
				Text(commit.message.components(separatedBy: "\n").first ?? commit.message)
					.font(.body)
					.lineLimit(1)

				if commit.parentIds.count > 1 {
					Text("Merge")
						.font(.caption2.bold())
						.foregroundStyle(.purple)
						.padding(.horizontal, 4)
						.padding(.vertical, 1)
						.background(.purple.opacity(0.15), in: RoundedRectangle(cornerRadius: 3))
				}
			}

			HStack(spacing: 6) {
				Text(commit.authorName)
					.font(.caption)
					.foregroundStyle(.secondary)

				Text(commit.shortId)
					.font(.caption.monospaced())
					.foregroundStyle(.tertiary)

				Spacer()

				Text(formatDate(commit.authorDate))
					.font(.caption)
					.foregroundStyle(.tertiary)
			}
		}
	}

	private var initials: String {
		let parts = commit.authorName.split(separator: " ")
		if parts.count >= 2 {
			return String(parts[0].prefix(1) + parts[1].prefix(1)).uppercased()
		}
		return String(commit.authorName.prefix(2)).uppercased()
	}

	private var avatarColor: Color {
		let hash = commit.authorEmail.hashValue
		let colors: [Color] = [.blue, .green, .orange, .purple, .pink, .teal, .indigo, .mint]
		return colors[abs(hash) % colors.count]
	}

	private func formatDate(_ dateStr: String) -> String {
		DateFormatters.relativeString(from: dateStr)
	}
}
