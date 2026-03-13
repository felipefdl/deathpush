import SwiftUI

struct NestedRepoRow: View {
	let repo: DiscoveredRepo
	@State private var branch: String?

	var body: some View {
		HStack(spacing: 6) {
			Image(systemName: "folder.badge.gearshape")
				.foregroundStyle(.secondary)
				.frame(width: 16)

			Text(repo.name)
				.font(.body)
				.lineLimit(1)

			Spacer()

			if let branch {
				Text(branch)
					.font(.caption.monospaced())
					.foregroundStyle(.tertiary)
					.lineLimit(1)
					.frame(maxWidth: 120, alignment: .trailing)
			}
		}
		.contentShape(Rectangle())
		.task {
			branch = try? getRepoBranch(path: repo.path)
		}
	}
}
