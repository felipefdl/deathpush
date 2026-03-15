import SwiftUI

struct ExplorerFolderRow: View {
	let entry: ExplorerEntry
	let depth: Int
	let repoService: RepositoryService?
	let contextMenus: ExplorerContextMenus
	let isCut: Bool

	private var isExpanded: Bool {
		repoService?.explorerExpandedPaths.contains(entry.path) ?? false
	}

	private func toggle() {
		if isExpanded {
			repoService?.explorerExpandedPaths.remove(entry.path)
		} else {
			repoService?.explorerExpandedPaths.insert(entry.path)
			_ = try? repoService?.listExplorerDirectory(path: entry.path)
		}
	}

	var body: some View {
		Button(action: toggle) {
			HStack(spacing: 4) {
				Image(systemName: "chevron.right")
					.font(.system(size: 10, weight: .semibold))
					.foregroundStyle(.tertiary)
					.rotationEffect(.degrees(isExpanded ? 90 : 0))
					.animation(.easeInOut(duration: 0.15), value: isExpanded)
					.frame(width: 14)

				FileIconView(fileName: entry.name, isDirectory: true, isExpanded: isExpanded)

				Text(entry.name)
					.font(.system(size: 13))
					.lineLimit(1)
					.truncationMode(.middle)

				if entry.isSymlink {
					Image(systemName: "arrow.turn.right.up")
						.font(.caption2)
						.foregroundStyle(.tertiary)
				}

				Spacer()
			}
			.padding(.leading, CGFloat(depth) * 16)
			.contentShape(Rectangle())
		}
		.buttonStyle(.plain)
		.tag(entry.path)
		.listRowSeparator(.hidden)
		.listRowInsets(EdgeInsets(top: 1, leading: 4, bottom: 1, trailing: 8))
		.contextMenu {
			contextMenus.folderMenu(entry: entry)
		}
		.opacity(isCut ? 0.5 : 1.0)
		.draggable(entry.path) {}
		.dropDestination(for: String.self) { droppedPaths, _ in
			for path in droppedPaths {
				try? repoService?.moveExplorerEntries(sources: [path], destinationDir: entry.path, onConflict: "keepBoth")
			}
			return true
		}
	}
}
