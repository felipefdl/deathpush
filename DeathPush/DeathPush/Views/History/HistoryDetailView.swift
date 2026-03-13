import SwiftUI

struct HistoryDetailView: View {
	let commitId: String
	@Environment(AppState.self) private var appState
	@Environment(TabState.self) private var tabState
	@State private var commitDetail: CommitDetail?
	@State private var selectedFilePath: String?
	@State private var diffContent: DiffContent?
	@State private var diffMode: DiffMode = .sideBySide
	@State private var errorMessage: String?

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	var body: some View {
		VStack(spacing: 0) {
			if let error = errorMessage, commitDetail == nil {
				ContentUnavailableView(
					"Error Loading Commit",
					systemImage: "exclamationmark.triangle",
					description: Text(error)
				)
			} else if let detail = commitDetail {
				commitHeader(detail)
				Divider()
				changedFilesList(detail)
				Divider()
				diffPanel
			} else {
				ProgressView("Loading commit...")
					.frame(maxWidth: .infinity, maxHeight: .infinity)
			}
		}
		.onChange(of: commitId, initial: true) { _, newId in
			loadCommitDetail(newId)
		}
	}

	private func commitHeader(_ detail: CommitDetail) -> some View {
		GlassEffectContainer {
			VStack(alignment: .leading, spacing: 4) {
				Text(detail.commit.message)
					.font(.headline)
					.lineLimit(3)

				HStack(spacing: 8) {
					Text(detail.commit.authorName)
						.font(.caption.bold())

					Text(detail.commit.shortId)
						.font(.caption.monospaced())
						.padding(.horizontal, 6)
						.padding(.vertical, 1)
						.glassEffect(.regular)

					Text(formatDate(detail.commit.authorDate))
						.font(.caption)
						.foregroundStyle(.secondary)

					if detail.commit.parentIds.count > 1 {
						Text("Merge")
							.font(.caption2.bold())
							.foregroundStyle(.purple)
							.padding(.horizontal, 4)
							.padding(.vertical, 1)
							.glassEffect(.regular.tint(.purple))
					}

					Spacer()

					Button {
						NSPasteboard.general.clearContents()
						NSPasteboard.general.setString(detail.commit.id, forType: .string)
					} label: {
						Image(systemName: "doc.on.doc")
					}
					.buttonStyle(.glass)
					.controlSize(.small)
					.help("Copy SHA")

					Button {
						NSPasteboard.general.clearContents()
						NSPasteboard.general.setString(detail.commit.message, forType: .string)
					} label: {
						Image(systemName: "text.document")
					}
					.buttonStyle(.glass)
					.controlSize(.small)
					.help("Copy Commit Message")
				}

				if detail.commit.parentIds.count > 1 {
					HStack(spacing: 4) {
						Text("Parents:")
							.font(.caption)
							.foregroundStyle(.secondary)
						ForEach(detail.commit.parentIds, id: \.self) { parentId in
							Text(String(parentId.prefix(7)))
								.font(.caption.monospaced())
								.foregroundStyle(.tertiary)
						}
					}
				}
			}
			.padding(10)
		}
	}

	private func changedFilesList(_ detail: CommitDetail) -> some View {
		List(detail.files, id: \.path, selection: $selectedFilePath) { file in
			HStack(spacing: 6) {
				FileIconView(fileName: URL(fileURLWithPath: file.path).lastPathComponent)

				Text(fileStatusLetter(file.status))
					.font(.caption.bold().monospaced())
					.foregroundStyle(fileStatusColor(file.status))
					.frame(width: 16)

				Text(file.path)
					.font(.body)
					.lineLimit(1)
					.truncationMode(.middle)
			}
			.tag(file.path)
		}
		.listStyle(.plain)
		.scrollEdgeEffectStyle(.soft, for: .top)
		.frame(maxHeight: 180)
		.onChange(of: selectedFilePath) { _, newPath in
			if let path = newPath {
				loadFileDiff(path)
			} else if let firstFile = detail.files.first {
				selectedFilePath = firstFile.path
			}
		}
	}

	@ViewBuilder
	private var diffPanel: some View {
		if let diff = diffContent {
			VStack(spacing: 0) {
				diffPanelHeader
				Divider()
				if diff.fileType == "image" {
					ImageDiffView(diff: diff)
				} else {
					MonacoDiffView(diff: diff, diffMode: diffMode, themeJSON: appState.themeService.themeDataJSON)
						.background(Color(nsColor: appState.themeService.color(forKey: "editor.background") ?? .black))
				}
			}
		} else {
			ProgressView("Loading diff...")
				.frame(maxWidth: .infinity, maxHeight: .infinity)
		}
	}

	private var diffPanelHeader: some View {
		GlassEffectContainer(spacing: 8) {
			HStack {
				if let path = selectedFilePath {
					HStack(spacing: 4) {
						Image(systemName: "doc.text")
							.foregroundStyle(.secondary)
						Text(path)
							.font(.callout)
							.lineLimit(1)
							.truncationMode(.middle)
					}
				}

				Spacer()

				Button(action: {
					diffMode = diffMode == .sideBySide ? .inline : .sideBySide
				}) {
					Image(systemName: diffMode == .sideBySide ? "rectangle.split.2x1" : "rectangle")
				}
				.buttonStyle(.glass)
				.controlSize(.small)
				.help(diffMode == .sideBySide ? "Switch to Inline" : "Switch to Side-by-Side")
			}
			.padding(.horizontal, 12)
			.padding(.vertical, 6)
		}
	}

	// MARK: - Data Loading

	private func loadCommitDetail(_ id: String) {
		guard let service = repoService else { return }
		errorMessage = nil

		do {
			let detail = try getCommitDetail(sessionId: service.sessionId, commitId: id)
			commitDetail = detail
			if let firstFile = detail.files.first {
				selectedFilePath = firstFile.path
				loadFileDiff(firstFile.path)
			} else {
				selectedFilePath = nil
				diffContent = nil
			}
		} catch {
			errorMessage = error.localizedDescription
			commitDetail = nil
			diffContent = nil
		}
	}

	private func loadFileDiff(_ path: String) {
		guard let service = repoService else { return }

		do {
			let commitDiff = try getCommitFileDiff(sessionId: service.sessionId, commitId: commitId, path: path)
			diffContent = DiffContent(
				path: commitDiff.path,
				original: commitDiff.original,
				modified: commitDiff.modified,
				originalLanguage: commitDiff.language,
				fileType: commitDiff.fileType
			)
		} catch {
			diffContent = nil
		}
	}

	// MARK: - Helpers

	private func formatDate(_ dateStr: String) -> String {
		let formatter = ISO8601DateFormatter()
		formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
		guard let date = formatter.date(from: dateStr) ?? ISO8601DateFormatter().date(from: dateStr) else {
			return dateStr
		}
		let relative = RelativeDateTimeFormatter()
		relative.unitsStyle = .abbreviated
		return relative.localizedString(for: date, relativeTo: Date())
	}

	private func fileStatusLetter(_ status: String) -> String {
		switch status {
		case "added": "A"
		case "deleted": "D"
		case "modified": "M"
		case "renamed": "R"
		case "copied": "C"
		default: "?"
		}
	}

	private func fileStatusColor(_ status: String) -> Color {
		switch status {
		case "added": .green
		case "deleted": .red
		case "modified": .orange
		case "renamed": .blue
		case "copied": .cyan
		default: .secondary
		}
	}
}
