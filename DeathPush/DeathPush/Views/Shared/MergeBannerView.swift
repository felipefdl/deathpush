import SwiftUI

struct MergeBannerView: View {
	@Environment(TabState.self) private var tabState

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	private var operationState: RepoOperationState {
		repoService?.operationState ?? .clean
	}

	private var displayName: String {
		switch operationState {
		case .merging: "Merge"
		case .rebasing: "Rebase"
		case .cherryPicking: "Cherry-pick"
		case .reverting: "Revert"
		case .clean: ""
		}
	}

	var body: some View {
		if operationState != .clean {
			GlassEffectContainer {
				HStack(spacing: 12) {
					Image(systemName: "exclamationmark.triangle.fill")
						.foregroundStyle(.orange)

					Text("\(displayName) in progress")
						.font(.callout.bold())

					Spacer()

					if operationState == .rebasing {
						Button("Skip") {
							Task { try? repoService?.skipRebase() }
						}
						.buttonStyle(.glass)
						.controlSize(.small)
					}

					Button("Continue") {
						Task {
							switch operationState {
							case .merging: try? repoService?.continueMerge()
							case .rebasing: try? repoService?.continueRebase()
							case .cherryPicking: try? repoService?.continueCherryPick()
							case .reverting: try? repoService?.continueRevert()
							case .clean: break
							}
						}
					}
					.buttonStyle(.glassProminent)
					.tint(.green)
					.controlSize(.small)

					Button("Abort") {
						Task {
							switch operationState {
							case .merging: try? repoService?.abortMerge()
							case .rebasing: try? repoService?.abortRebase()
							case .cherryPicking: try? repoService?.abortCherryPick()
							case .reverting: try? repoService?.abortRevert()
							case .clean: break
							}
						}
					}
					.buttonStyle(.glass)
					.tint(.red)
					.controlSize(.small)
				}
				.padding(10)
			}
		}
	}
}
