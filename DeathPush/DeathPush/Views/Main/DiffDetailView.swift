import SwiftUI

struct DiffDetailView: View {
	let path: String
	@Environment(AppState.self) private var appState
	@Environment(TabState.self) private var tabState
	@State private var diffContent: DiffContent?
	@State private var isStaged = false
	@State private var errorMessage: String?
	@State private var diffMode: DiffMode = .sideBySide
	@State private var diffVersion: Int = 0
	@State private var hunksData: FileDiffWithHunks?
	@State private var selectedHunkIndex: Int?
	@State private var selectedLineStart: Int?
	@State private var selectedLineEnd: Int?

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	var body: some View {
		VStack(spacing: 0) {
			DiffHeaderView(
				path: path,
				isStaged: isStaged,
				diffMode: $diffMode,
				onStage: { stageFile() },
				onUnstage: { unstageFile() },
				onDiscard: { discardFile() }
			)

			Divider()

			if let error = errorMessage, diffContent == nil {
				ContentUnavailableView(
					"Error Loading Diff",
					systemImage: "exclamationmark.triangle",
					description: Text(error)
				)
			} else if let diff = diffContent {
				if diff.fileType == "image" {
					ImageDiffView(diff: diff)
				} else {
					MonacoDiffView(
						diff: diff,
						diffMode: diffMode,
						contentVersion: diffVersion,
						themeJSON: appState.themeService.themeDataJSON,
						hunkMetaJSON: buildHunkMetaJSON(),
						onLineSelection: { hunkIdx, start, end, _ in
							withAnimation(.easeInOut(duration: 0.15)) {
								selectedHunkIndex = hunkIdx
								selectedLineStart = start
								selectedLineEnd = end
							}
						},
						onLineSelectionCleared: {
							withAnimation(.easeInOut(duration: 0.15)) {
								selectedHunkIndex = nil
								selectedLineStart = nil
								selectedLineEnd = nil
							}
						}
					)
					.background(Color(nsColor: appState.themeService.color(forKey: "editor.background") ?? .black))
					.overlay(alignment: .bottom) {
						if let hunkIdx = selectedHunkIndex,
						   let start = selectedLineStart,
						   let end = selectedLineEnd {
							HStack(spacing: 8) {
								Text("\(end - start + 1) line\(end - start == 0 ? "" : "s") selected")
									.font(.caption)
								Button(isStaged ? "Unstage Lines" : "Stage Lines") {
									stageSelectedLines(hunkIndex: hunkIdx, start: start, end: end)
								}
								.buttonStyle(.glassProminent)
								.controlSize(.small)
							}
							.padding(8)
							.glassEffect(.regular)
							.padding(.bottom, 12)
							.transition(.move(edge: .bottom).combined(with: .opacity))
						}
					}
				}
			} else {
				ProgressView("Loading diff...")
					.frame(maxWidth: .infinity, maxHeight: .infinity)
			}
		}
		.onChange(of: path, initial: true) { _, newPath in
			loadDiff(for: newPath)
		}
	}

	private func loadDiff(for filePath: String) {
		guard let service = repoService else { return }
		errorMessage = nil

		let groups = service.resourceGroups
		isStaged = groups.contains { $0.kind == .index && $0.files.contains { $0.path == filePath } }

		do {
			diffContent = try getFileDiff(sessionId: service.sessionId, path: filePath, staged: isStaged)
			hunksData = try? service.getHunks(path: filePath, staged: isStaged)
		} catch {
			errorMessage = error.localizedDescription
			diffContent = nil
			hunksData = nil
		}
	}

	private func buildHunkMetaJSON() -> String? {
		guard let hunks = hunksData else { return nil }
		var meta: [[String: Int]] = []
		for (idx, hunk) in hunks.hunks.enumerated() {
			let modStart = Int(hunk.newStart)
			let modEnd = modStart + Int(hunk.newLines) - 1
			meta.append([
				"hunkIndex": idx,
				"modifiedStartLine": modStart,
				"modifiedEndLine": modEnd,
				"lineCount": hunk.lines.count,
			])
		}
		guard let data = try? JSONSerialization.data(withJSONObject: meta),
		      let str = String(data: data, encoding: .utf8) else { return nil }
		return str
	}

	private func stageSelectedLines(hunkIndex: Int, start: Int, end: Int) {
		guard let hunks = hunksData,
		      hunkIndex < hunks.hunks.count else { return }

		let hunk = hunks.hunks[hunkIndex]
		var modLineIdx = 0
		var firstDiffLineIdx: Int?
		var lastDiffLineIdx: Int?

		for (diffIdx, line) in hunk.lines.enumerated() {
			if line.lineType == "context" || line.lineType == "add" {
				if modLineIdx >= start && modLineIdx <= end {
					if firstDiffLineIdx == nil { firstDiffLineIdx = diffIdx }
					lastDiffLineIdx = diffIdx
				}
				modLineIdx += 1
			}
		}

		guard let first = firstDiffLineIdx, let last = lastDiffLineIdx else { return }

		try? repoService?.stageLinesInHunk(
			path: path,
			hunkIndex: UInt32(hunkIndex),
			lineStart: UInt32(first),
			lineEnd: UInt32(last),
			staged: isStaged
		)

		selectedHunkIndex = nil
		selectedLineStart = nil
		selectedLineEnd = nil
		diffVersion += 1
		loadDiff(for: path)
	}

	private func stageFile() {
		try? repoService?.stageFiles([path])
		diffVersion += 1
		loadDiff(for: path)
	}

	private func unstageFile() {
		try? repoService?.unstageFiles([path])
		diffVersion += 1
		loadDiff(for: path)
	}

	private func discardFile() {
		try? repoService?.discardFileChanges([path])
		diffVersion += 1
		loadDiff(for: path)
	}
}

struct DiffHeaderView: View {
  let path: String
  let isStaged: Bool
  @Binding var diffMode: DiffMode
  let onStage: () -> Void
  let onUnstage: () -> Void
  let onDiscard: () -> Void

  @State private var showDiscardConfirm = false

  var body: some View {
    GlassEffectContainer(spacing: 8) {
      HStack {
        HStack(spacing: 4) {
          Image(systemName: "doc.text")
            .foregroundStyle(.secondary)
          Text(path)
            .font(.callout)
            .lineLimit(1)
            .truncationMode(.middle)
        }

        if isStaged {
          Text("Staged")
            .font(.caption2.bold())
            .foregroundStyle(.green)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .glassEffect(.regular)
        }

        Spacer()

        // Diff mode toggle
        Button(action: {
          diffMode = diffMode == .sideBySide ? .inline : .sideBySide
        }) {
          Image(systemName: diffMode == .sideBySide ? "rectangle.split.2x1" : "rectangle")
        }
        .buttonStyle(.glass)
        .controlSize(.small)
        .help(diffMode == .sideBySide ? "Switch to Inline" : "Switch to Side-by-Side")

        if !isStaged {
          Button(action: onStage) {
            Image(systemName: "plus")
          }
          .buttonStyle(.glassProminent)
          .controlSize(.small)
          .help("Stage File")

          Button(action: { showDiscardConfirm = true }) {
            Image(systemName: "trash")
          }
          .buttonStyle(.glass)
          .tint(.red)
          .controlSize(.small)
          .help("Discard Changes")
          .confirmationDialog(
            "Discard changes to \(URL(fileURLWithPath: path).lastPathComponent)?",
            isPresented: $showDiscardConfirm,
            titleVisibility: .visible
          ) {
            Button("Discard", role: .destructive) { onDiscard() }
          }
        } else {
          Button(action: onUnstage) {
            Image(systemName: "minus")
          }
          .buttonStyle(.glass)
          .controlSize(.small)
          .help("Unstage File")
        }
      }
      .padding(.horizontal, 12)
      .padding(.vertical, 6)
    }
  }
}
