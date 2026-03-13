import SwiftUI

struct BranchPickerSheet: View {
	let title: String
	let branches: [BranchEntry]
	let onSelect: (String) -> Void

	@Environment(\.dismiss) private var dismiss
	@State private var filterText = ""

	private var filteredBranches: [BranchEntry] {
		guard !filterText.isEmpty else { return branches }
		let query = filterText.lowercased()
		return branches.filter { $0.name.lowercased().contains(query) }
	}

	var body: some View {
		VStack(spacing: 0) {
			Text(title)
				.font(.headline)
				.padding(.top, 12)
				.padding(.bottom, 8)

			HStack {
				Image(systemName: "magnifyingglass")
					.foregroundStyle(.tertiary)
				TextField("Filter branches...", text: $filterText)
					.textFieldStyle(.plain)
				if !filterText.isEmpty {
					Button { filterText = "" } label: {
						Image(systemName: "xmark.circle.fill")
					}
					.buttonStyle(.plain)
					.foregroundStyle(.tertiary)
				}
			}
			.padding(.horizontal, 12)
			.padding(.vertical, 6)

			Divider()

			ScrollView {
				LazyVStack(spacing: 0) {
					ForEach(filteredBranches, id: \.name) { branch in
						Button {
							onSelect(branch.name)
							dismiss()
						} label: {
							HStack {
								Image(systemName: branch.isRemote ? "globe" : "arrow.triangle.branch")
									.foregroundStyle(.secondary)
									.frame(width: 16)

								Text(branch.name)
									.font(.body)
									.lineLimit(1)

								Spacer()

								if branch.isHead {
									Image(systemName: "checkmark")
										.font(.caption)
										.foregroundStyle(.secondary)
								}
							}
							.padding(.horizontal, 12)
							.padding(.vertical, 6)
							.contentShape(Rectangle())
						}
						.buttonStyle(.plain)
					}
				}
			}
			.frame(minHeight: 200, maxHeight: 400)

			Divider()

			HStack {
				Spacer()
				Button("Cancel") { dismiss() }
					.buttonStyle(.glass)
			}
			.padding(12)
		}
		.frame(width: 350)
	}
}
