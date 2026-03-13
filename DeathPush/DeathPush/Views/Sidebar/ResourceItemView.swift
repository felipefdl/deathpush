import SwiftUI

struct ResourceItemView: View {
	let file: FileEntry
	var depth: Int = 0
	let isSelected: Bool
	var showDirectoryPath = true
	let onSelect: () -> Void

	var body: some View {
		Button(action: onSelect) {
			HStack(spacing: 6) {
				FileIconView(fileName: fileName)

				Text(fileName)
					.font(.system(size: 13))
					.lineLimit(1)
					.truncationMode(.middle)
					.layoutPriority(1)

				if showDirectoryPath, let dir = directoryPath, !dir.isEmpty {
					Text(dir)
						.font(.system(size: 11))
						.foregroundStyle(.secondary.opacity(0.7))
						.lineLimit(1)
						.truncationMode(.middle)
				}

				Spacer(minLength: 4)

				FileStatusIcon(status: file.status)
			}
			.padding(.leading, CGFloat(depth) * 16 + 18)
			.contentShape(Rectangle())
		}
		.buttonStyle(.plain)
		.listRowSeparator(.hidden)
		.listRowInsets(EdgeInsets(top: 1, leading: 4, bottom: 1, trailing: 8))
		.listRowBackground(isSelected ? Color.accentColor.opacity(0.15) : Color.clear)
	}

	private var fileName: String {
		URL(fileURLWithPath: file.path).lastPathComponent
	}

	private var directoryPath: String? {
		let components = file.path.split(separator: "/").dropLast()
		return components.isEmpty ? nil : components.joined(separator: "/")
	}
}

struct FileStatusIcon: View {
	let status: FileStatus

	var body: some View {
		Text(statusLetter)
			.font(.system(size: 11, weight: .bold, design: .monospaced))
			.foregroundStyle(statusColor)
			.frame(width: 16, height: 16, alignment: .trailing)
	}

	private var statusLetter: String {
		switch status {
		case .modified, .indexModified: "M"
		case .added, .indexAdded: "A"
		case .deleted, .indexDeleted: "D"
		case .renamed, .indexRenamed: "R"
		case .copied, .indexCopied: "C"
		case .untracked: "U"
		case .ignored: "!"
		case .typeChanged: "T"
		case .intentToAdd, .intentToRename: "I"
		case .bothDeleted, .addedByUs, .deletedByThem,
		     .addedByThem, .deletedByUs, .bothAdded, .bothModified: "X"
		}
	}

	private var statusColor: Color {
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
