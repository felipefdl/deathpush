import SwiftUI

struct ExplorerFileRow: View {
	let entry: ExplorerEntry
	let depth: Int
	let isSelected: Bool
	let gitStatus: FileStatus?
	let onSelect: () -> Void

	var body: some View {
		Button(action: onSelect) {
			HStack(spacing: 6) {
				FileIconView(fileName: entry.name)

				Text(entry.name)
					.font(.system(size: 13))
					.lineLimit(1)
					.truncationMode(.middle)
					.foregroundStyle(gitStatusColor)

				if entry.isSymlink {
					Image(systemName: "arrow.turn.right.up")
						.font(.caption2)
						.foregroundStyle(.tertiary)
				}

				Spacer()

				if let status = gitStatus {
					FileStatusIcon(status: status)
				}
			}
			.padding(.leading, CGFloat(depth) * 16 + 18)
			.contentShape(Rectangle())
		}
		.buttonStyle(.plain)
		.tag(entry.path)
		.listRowSeparator(.hidden)
		.listRowInsets(EdgeInsets(top: 1, leading: 4, bottom: 1, trailing: 8))
		.listRowBackground(isSelected ? Color.accentColor.opacity(0.15) : Color.clear)
	}

	private var gitStatusColor: Color {
		guard let status = gitStatus else { return .primary }
		return FileStatusIcon.colorForStatus(status)
	}
}

extension FileStatusIcon {
	static func colorForStatus(_ status: FileStatus) -> Color {
		switch status {
		case .modified, .indexModified, .typeChanged: .orange
		case .added, .indexAdded, .untracked, .intentToAdd, .intentToRename: .green
		case .deleted, .indexDeleted: .red
		case .renamed, .indexRenamed: .blue
		case .copied, .indexCopied: .cyan
		case .ignored: .secondary
		case .bothDeleted, .addedByUs, .deletedByThem,
		     .addedByThem, .deletedByUs, .bothAdded, .bothModified: .purple
		}
	}
}
