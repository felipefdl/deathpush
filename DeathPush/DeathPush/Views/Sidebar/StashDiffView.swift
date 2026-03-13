import SwiftUI

struct StashDiffView: View {
	let stashIndex: UInt32
	@Environment(AppState.self) private var appState
	@Environment(TabState.self) private var tabState
	@State private var diffContent: DiffContent?
	@State private var hunkCount: Int = 0
	@State private var errorMessage: String?

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	var body: some View {
		VStack(spacing: 0) {
			stashHeader

			Divider()

			if let error = errorMessage, diffContent == nil {
				ContentUnavailableView(
					"Error Loading Stash",
					systemImage: "exclamationmark.triangle",
					description: Text(error)
				)
			} else if let diff = diffContent {
				MonacoDiffView(
					diff: diff,
					diffMode: .inline,
					contentVersion: 0,
					themeJSON: appState.themeService.themeDataJSON
				)
				.background(Color(nsColor: appState.themeService.color(forKey: "editor.background") ?? .black))
			} else {
				ProgressView("Loading stash...")
					.frame(maxWidth: .infinity, maxHeight: .infinity)
			}
		}
		.onChange(of: stashIndex, initial: true) { _, newIndex in
			loadStash(index: newIndex)
		}
	}

	@ViewBuilder
	private var stashHeader: some View {
		GlassEffectContainer(spacing: 8) {
			HStack {
				HStack(spacing: 4) {
					Image(systemName: "tray")
						.foregroundStyle(.secondary)
					Text("stash@{\(stashIndex)}")
						.font(.callout.monospaced())
						.lineLimit(1)
				}

				Text("\(hunkCount) hunk\(hunkCount == 1 ? "" : "s")")
					.font(.caption2.bold())
					.foregroundStyle(.secondary)
					.padding(.horizontal, 6)
					.padding(.vertical, 2)
					.glassEffect(.regular)

				Spacer()

				Button {
					tabState.selectedStashIndex = nil
				} label: {
					Image(systemName: "xmark")
				}
				.buttonStyle(.glass)
				.controlSize(.small)
				.help("Close Stash Diff")
			}
			.padding(.horizontal, 12)
			.padding(.vertical, 6)
		}
	}

	private func loadStash(index: UInt32) {
		guard let service = repoService else { return }
		errorMessage = nil

		do {
			let result = try service.showStash(index: index)
			hunkCount = result.hunks.count

			var originalLines: [String] = []
			var modifiedLines: [String] = []

			for hunk in result.hunks {
				for line in hunk.lines {
					switch line.lineType {
					case "remove":
						originalLines.append(line.content)
					case "add":
						modifiedLines.append(line.content)
					case "context":
						originalLines.append(line.content)
						modifiedLines.append(line.content)
					default:
						break
					}
				}
			}

			diffContent = DiffContent(
				path: result.path,
				original: originalLines.joined(separator: "\n"),
				modified: modifiedLines.joined(separator: "\n"),
				originalLanguage: nil,
				fileType: "text"
			)
		} catch {
			errorMessage = error.localizedDescription
			diffContent = nil
		}
	}
}
