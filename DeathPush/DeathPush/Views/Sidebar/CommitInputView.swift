import SwiftUI

struct CommitInputView: View {
	@State private var message = ""
	@State private var amendMode = false
	@State private var showCommitOptions = false
	@Environment(TabState.self) private var tabState

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	private var canCommit: Bool {
		(!message.isEmpty || amendMode) && hasStagedFiles
	}

	private var hasStagedFiles: Bool {
		repoService?.resourceGroups.contains { $0.kind == .index && !$0.files.isEmpty } ?? false
	}

	private var branchName: String {
		repoService?.currentBranch ?? "HEAD"
	}

	private var commitLabel: String {
		amendMode ? "Amend" : "Commit"
	}

	var body: some View {
		VStack(spacing: 8) {
			TextField(
				"Message (\u{2318}+Enter to commit on \"\(branchName)\")",
				text: $message,
				axis: .vertical
			)
			.lineLimit(1...6)
			.textFieldStyle(.plain)
			.font(.body)
			.padding(8)
			.background(.quaternary.opacity(0.3), in: RoundedRectangle(cornerRadius: 6))

			HStack(spacing: 2) {
				Button {
					handleCommit()
				} label: {
					HStack(spacing: 4) {
						Image(systemName: "checkmark")
						Text(commitLabel)
					}
					.frame(maxWidth: .infinity, minHeight: 20)
				}
				.buttonStyle(.glassProminent)
				.keyboardShortcut(.return, modifiers: .command)

				Button {
					showCommitOptions = true
				} label: {
					Image(systemName: "chevron.down")
						.font(.caption2)
						.frame(minHeight: 20)
				}
				.buttonStyle(.glassProminent)
				.popover(isPresented: $showCommitOptions, arrowEdge: .bottom) {
					VStack(alignment: .leading, spacing: 2) {
						Button("Commit") {
							showCommitOptions = false
							handleCommit()
						}
						Button("Commit (Amend)") {
							showCommitOptions = false
							handleAmendCommit()
						}
						Divider()
						Button("Commit & Push") {
							showCommitOptions = false
							handleCommitAndPush()
						}
						Button("Commit & Sync") {
							showCommitOptions = false
							handleCommitAndSync()
						}
					}
					.buttonStyle(.plain)
					.padding(6)
				}
			}
			.opacity(canCommit ? 1.0 : 0.4)
			.allowsHitTesting(canCommit)
		}
		.padding(8)
	}

	// MARK: - Actions

	private func handleCommit() {
		guard canCommit else { return }
		try? repoService?.commitChanges(message: message, amend: amendMode)
		message = ""
		amendMode = false
	}

	private func handleAmendCommit() {
		amendMode = true
		if message.isEmpty {
			message = (try? repoService?.lastCommitMessage()) ?? ""
		}
		guard canCommit else { return }
		try? repoService?.commitChanges(message: message, amend: true)
		message = ""
		amendMode = false
	}

	private func handleCommitAndPush() {
		guard canCommit else { return }
		try? repoService?.commitChanges(message: message, amend: amendMode)
		try? repoService?.pushRemote()
		message = ""
		amendMode = false
	}

	private func handleCommitAndSync() {
		guard canCommit else { return }
		try? repoService?.commitChanges(message: message, amend: amendMode)
		try? repoService?.pullRemote()
		try? repoService?.pushRemote()
		message = ""
		amendMode = false
	}
}
