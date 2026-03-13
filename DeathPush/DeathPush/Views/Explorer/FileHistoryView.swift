import SwiftUI

struct FileHistoryView: View {
	let path: String
	@Environment(TabState.self) private var tabState
	@State private var commits: [CommitEntry] = []
	@State private var errorMessage: String?
	@State private var isLoadingMore = false

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	var body: some View {
		VStack(spacing: 0) {
			header
			Divider()
			content
		}
		.onAppear { loadHistory() }
		.onChange(of: path) { _, _ in loadHistory() }
	}

	private var header: some View {
		GlassEffectContainer {
			HStack {
				HStack(spacing: 4) {
					Image(systemName: "clock.arrow.circlepath")
						.foregroundStyle(.secondary)
					Text("History: \(URL(fileURLWithPath: path).lastPathComponent)")
						.font(.callout.bold())
						.lineLimit(1)
						.truncationMode(.middle)
				}

				Spacer()

				Button {
					tabState.explorerShowFileHistory = false
				} label: {
					Image(systemName: "xmark")
				}
				.buttonStyle(.glass)
				.controlSize(.small)
				.help("Close History")
			}
			.padding(.horizontal, 12)
			.padding(.vertical, 6)
		}
	}

	@ViewBuilder
	private var content: some View {
		if let error = errorMessage {
			ContentUnavailableView(
				"History Unavailable",
				systemImage: "exclamationmark.triangle",
				description: Text(error)
			)
		} else if commits.isEmpty {
			ProgressView("Loading history...")
				.frame(maxWidth: .infinity, maxHeight: .infinity)
		} else {
			commitList
		}
	}

	private var commitList: some View {
		List {
			ForEach(commits, id: \.id) { entry in
				FileHistoryRow(commit: entry)
					.contentShape(Rectangle())
					.onTapGesture {
						navigateToCommit(entry.id)
					}
			}

			loadMoreButton
		}
		.listStyle(.plain)
		.scrollEdgeEffectStyle(.soft, for: .top)
	}

	private var loadMoreButton: some View {
		Button {
			loadMore()
		} label: {
			if isLoadingMore {
				ProgressView()
					.controlSize(.small)
					.frame(maxWidth: .infinity, alignment: .center)
					.padding(.vertical, 4)
			} else {
				Text("Load More")
					.font(.caption)
					.foregroundStyle(.secondary)
					.frame(maxWidth: .infinity, alignment: .center)
					.padding(.vertical, 4)
			}
		}
		.buttonStyle(.plain)
		.disabled(isLoadingMore)
	}

	// MARK: - Data Loading

	private func loadHistory() {
		errorMessage = nil
		commits = []

		guard let service = repoService else {
			errorMessage = "No repository open"
			return
		}

		do {
			commits = try service.fileLog(path: path)
		} catch {
			errorMessage = error.localizedDescription
		}
	}

	private func loadMore() {
		guard let service = repoService, !isLoadingMore else { return }
		isLoadingMore = true

		do {
			let more = try service.fileLog(path: path, skip: UInt32(commits.count))
			commits.append(contentsOf: more)
		} catch {
			// Silently fail on load more
		}

		isLoadingMore = false
	}

	// MARK: - Navigation

	private func navigateToCommit(_ commitId: String) {
		tabState.selectedCommitId = commitId
		tabState.sidebarSelection = .history
	}
}

struct FileHistoryRow: View {
	let commit: CommitEntry

	var body: some View {
		VStack(alignment: .leading, spacing: 4) {
			Text(commit.message.components(separatedBy: "\n").first ?? commit.message)
				.font(.body)
				.lineLimit(2)

			HStack(spacing: 6) {
				Text(commit.authorName)
					.font(.caption.bold())

				Text(commit.shortId)
					.font(.caption.monospaced())
					.foregroundStyle(.tertiary)

				Spacer()

				Text(formatRelativeDate(commit.authorDate))
					.font(.caption)
					.foregroundStyle(.secondary)
			}
		}
		.padding(.vertical, 2)
	}

	private func formatRelativeDate(_ dateStr: String) -> String {
		let formatter = ISO8601DateFormatter()
		formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
		guard let date = formatter.date(from: dateStr) ?? ISO8601DateFormatter().date(from: dateStr) else {
			return dateStr
		}
		let relative = RelativeDateTimeFormatter()
		relative.unitsStyle = .abbreviated
		return relative.localizedString(for: date, relativeTo: Date())
	}
}
