import SwiftUI

struct WorkspaceConfigSheet: View {
	@State private var entries: [WorkspaceEntry]
	let onSave: ([WorkspaceEntry]) -> Void

	@Environment(\.dismiss) private var dismiss

	init(workspaces: [WorkspaceEntry], onSave: @escaping ([WorkspaceEntry]) -> Void) {
		self.onSave = onSave
		_entries = State(initialValue: workspaces.isEmpty ? [WorkspaceEntry()] : workspaces)
	}

	var body: some View {
		VStack(alignment: .leading, spacing: 16) {
			Text("Workspace Settings")
				.font(.headline)

			Text("Add directories containing your Git repositories. The scan depth controls how many levels deep to search for projects within each directory.")
				.font(.callout)
				.foregroundStyle(.secondary)

			VStack(spacing: 6) {
				ForEach($entries) { $entry in
					entryRow(entry: $entry)
				}
			}

			Button {
				entries.append(WorkspaceEntry())
			} label: {
				Label("Add Directory", systemImage: "plus")
					.font(.callout)
			}
			.buttonStyle(.plain)
			.foregroundStyle(.blue)

			HStack {
				Button("Cancel") { dismiss() }
					.keyboardShortcut(.cancelAction)

				Spacer()

				Button("OK") { save() }
					.buttonStyle(.glassProminent)
					.tint(.blue)
					.keyboardShortcut(.defaultAction)
			}
		}
		.padding(20)
		.frame(width: 500)
	}

	private func entryRow(entry: Binding<WorkspaceEntry>) -> some View {
		HStack(spacing: 6) {
			TextField("Select a directory...", text: entry.directory)
				.textFieldStyle(.roundedBorder)

			Button {
				browseDirectory(for: entry)
			} label: {
				Image(systemName: "folder")
			}

			// Scan depth control
			HStack(spacing: 0) {
				Button {
					entry.wrappedValue.scanDepth = max(1, entry.wrappedValue.scanDepth - 1)
				} label: {
					Image(systemName: "chevron.left")
						.font(.caption2)
				}
				.disabled(entry.wrappedValue.scanDepth <= 1)
				.buttonStyle(.plain)
				.frame(width: 18, height: 18)

				Text("\(entry.wrappedValue.scanDepth)")
					.font(.caption)
					.monospacedDigit()
					.frame(minWidth: 14, alignment: .center)

				Button {
					entry.wrappedValue.scanDepth = min(5, entry.wrappedValue.scanDepth + 1)
				} label: {
					Image(systemName: "chevron.right")
						.font(.caption2)
				}
				.disabled(entry.wrappedValue.scanDepth >= 5)
				.buttonStyle(.plain)
				.frame(width: 18, height: 18)
			}

			if entries.count > 1 {
				Button {
					entries.removeAll { $0.id == entry.wrappedValue.id }
				} label: {
					Image(systemName: "xmark")
						.font(.caption2)
						.foregroundStyle(.secondary)
				}
				.buttonStyle(.plain)
			}
		}
	}

	private func browseDirectory(for entry: Binding<WorkspaceEntry>) {
		let panel = NSOpenPanel()
		panel.canChooseFiles = false
		panel.canChooseDirectories = true
		panel.allowsMultipleSelection = false
		panel.message = "Select Git Projects Directory"
		panel.prompt = "Select"

		if panel.runModal() == .OK, let url = panel.url {
			entry.wrappedValue.directory = url.path(percentEncoded: false)
		}
	}

	private func save() {
		let filtered = entries.filter { !$0.directory.trimmingCharacters(in: .whitespaces).isEmpty }
		onSave(filtered)
		dismiss()
	}
}
