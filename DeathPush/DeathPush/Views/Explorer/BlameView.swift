import SwiftUI

struct BlameView: View {
	let path: String
	@Environment(TabState.self) private var tabState
	@State private var blame: FileBlame?
	@State private var lines: [String] = []
	@State private var errorMessage: String?
	@State private var hoveredCommitId: String?

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	private static let gutterWidth: CGFloat = 180

	private static let groupColors: [Color] = [
		.blue,
		.purple,
		.orange,
		.teal,
		.pink,
		.green,
		.indigo,
		.mint,
	]

	var body: some View {
		Group {
			if let error = errorMessage {
				ContentUnavailableView(
					"Blame Unavailable",
					systemImage: "exclamationmark.triangle",
					description: Text(error)
				)
			} else if let blame = blame {
				blameContent(blame)
			} else {
				ProgressView("Loading blame...")
					.frame(maxWidth: .infinity, maxHeight: .infinity)
			}
		}
		.onAppear { loadBlame() }
		.onChange(of: path) { _, _ in loadBlame() }
	}

	private func blameContent(_ blame: FileBlame) -> some View {
		ScrollView([.vertical, .horizontal]) {
			LazyVStack(alignment: .leading, spacing: 0) {
				ForEach(Array(blame.lineGroups.enumerated()), id: \.offset) { groupIndex, group in
					let colorIndex = groupIndex % Self.groupColors.count
					let baseColor = Self.groupColors[colorIndex]
					let isHovered = hoveredCommitId == group.commitId

					ForEach(Int(group.startLine)...Int(group.endLine), id: \.self) { lineNumber in
						let isFirstLine = lineNumber == Int(group.startLine)
						let lineIndex = lineNumber - 1
						let lineText = lineIndex < lines.count ? lines[lineIndex] : ""

						HStack(alignment: .top, spacing: 0) {
							gutterCell(group: group, isFirstLine: isFirstLine)
								.frame(width: Self.gutterWidth, alignment: .leading)
								.padding(.leading, 8)
								.padding(.trailing, 4)

							Divider()

							lineNumberCell(lineNumber)
								.frame(width: 44, alignment: .trailing)
								.padding(.trailing, 8)

							Text(lineText)
								.font(.system(size: 13, design: .monospaced))
								.lineLimit(1)
								.textSelection(.enabled)

							Spacer(minLength: 0)
						}
						.frame(height: 20)
						.background(baseColor.opacity(isHovered ? 0.15 : 0.06))
						.onHover { hovering in
							hoveredCommitId = hovering ? group.commitId : nil
						}
						.onTapGesture {
							navigateToCommit(group.commitId)
						}
					}
				}
			}
		}
	}

	@ViewBuilder
	private func gutterCell(group: BlameLineGroup, isFirstLine: Bool) -> some View {
		if isFirstLine {
			HStack(spacing: 4) {
				Text(group.authorName)
					.font(.system(size: 11, weight: .semibold))
					.lineLimit(1)
					.truncationMode(.tail)
					.frame(maxWidth: 80, alignment: .leading)

				Text(group.shortId)
					.font(.system(size: 10, design: .monospaced))
					.foregroundStyle(.tertiary)

				Spacer(minLength: 0)

				Text(formatRelativeDate(group.authorDate))
					.font(.system(size: 10))
					.foregroundStyle(.secondary)
					.lineLimit(1)
			}
			.help("\(group.summary)\n\n\(group.authorName) <\(group.authorEmail)>\n\(group.shortId) \(formatRelativeDate(group.authorDate))")
		} else {
			Color.clear
		}
	}

	private func lineNumberCell(_ number: Int) -> some View {
		Text("\(number)")
			.font(.system(size: 11, design: .monospaced))
			.foregroundStyle(.tertiary)
	}

	// MARK: - Data Loading

	private func loadBlame() {
		errorMessage = nil
		blame = nil
		lines = []

		guard let service = repoService else {
			errorMessage = "No repository open"
			return
		}

		do {
			let fileContent = try service.readExplorerFile(path: path)
			guard fileContent.fileType == "text" else {
				errorMessage = "Blame is only available for text files."
				return
			}
			lines = fileContent.content.components(separatedBy: "\n")
			blame = try service.blameFile(path: path)
		} catch {
			errorMessage = error.localizedDescription
		}
	}

	// MARK: - Navigation

	private func navigateToCommit(_ commitId: String) {
		tabState.selectedCommitId = commitId
		tabState.sidebarSelection = .history
	}

	// MARK: - Formatting

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
