import SwiftUI

struct WorktreeRow: View {
	let worktree: WorktreeInfo

	var body: some View {
		HStack(spacing: 6) {
			Image(systemName: "arrow.triangle.branch")
				.foregroundStyle(.secondary)
				.frame(width: 16)

			Text(worktree.name)
				.font(.body)
				.lineLimit(1)

			if worktree.isMain {
				Text("main")
					.font(.caption)
					.glassEffect(.regular)
			}

			Spacer()

			if let branch = worktree.branch {
				Text(branch)
					.font(.caption.monospaced())
					.foregroundStyle(.tertiary)
					.lineLimit(1)
					.frame(maxWidth: 120, alignment: .trailing)
			}
		}
		.contentShape(Rectangle())
	}
}
