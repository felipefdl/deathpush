import SwiftUI

struct ToolbarPillView: View {
	@Environment(TabState.self) private var tabState
	@State private var showBranchPicker = false

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	private var operationDisplayName: String {
		switch repoService?.operationState {
		case .merging: "Merge"
		case .rebasing: "Rebase"
		case .cherryPicking: "Cherry-pick"
		case .reverting: "Revert"
		case .clean, .none: ""
		}
	}

	var body: some View {
		if let repoService {
			HStack(spacing: 8) {
				// Left side: branch + sync
				HStack(spacing: 4) {
					Button(action: { showBranchPicker.toggle() }) {
						HStack(spacing: 3) {
							Image(systemName: "arrow.triangle.branch")
							Text(repoService.currentBranch ?? "HEAD")
								.lineLimit(1)
						}
						.font(.caption)
						.foregroundStyle(.secondary)
					}
					.buttonStyle(.plain)
					.popover(isPresented: $showBranchPicker) {
						BranchPickerView()
							.frame(width: 350, height: 400)
					}

					if repoService.behind > 0 {
						HStack(spacing: 2) {
							Image(systemName: "arrow.down")
								.font(.caption2)
							Text("\(repoService.behind)")
								.font(.caption)
						}
						.foregroundStyle(.orange)
					}

					if repoService.ahead > 0 {
						HStack(spacing: 2) {
							Image(systemName: "arrow.up")
								.font(.caption2)
							Text("\(repoService.ahead)")
								.font(.caption)
						}
						.foregroundStyle(.blue)
					}
				}

				Divider()
					.frame(height: 14)

				// Right side: contextual status
				Group {
					if repoService.isLoading {
						HStack(spacing: 4) {
							ProgressView()
								.controlSize(.mini)
							Text("Loading...")
						}
					} else if repoService.operationState != .clean {
						HStack(spacing: 4) {
							Image(systemName: "exclamationmark.triangle.fill")
							Text(operationDisplayName)
						}
						.foregroundStyle(.orange)
					} else if repoService.totalChanges > 0 {
						HStack(spacing: 4) {
							Image(systemName: "pencil")
							Text("\(repoService.totalChanges) change\(repoService.totalChanges == 1 ? "" : "s")")
						}
					} else {
						HStack(spacing: 4) {
							Image(systemName: "checkmark.circle.fill")
							Text("Clean")
						}
						.foregroundStyle(.green)
					}
				}
				.font(.caption)
			}
			.padding(.horizontal, 8)
			.padding(.vertical, 4)
		}
	}
}
