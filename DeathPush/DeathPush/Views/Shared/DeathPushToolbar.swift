import SwiftUI

struct DeathPushToolbarButtons: View {
	@Environment(TabState.self) private var tabState

	private var repoService: RepositoryService? {
		tabState.repoService
	}

	var body: some View {
		Button(action: {
			Task { try? repoService?.fetchRemote() }
		}) {
			Image(systemName: "arrow.clockwise")
		}
		.help("Fetch")

		Button(action: {
			Task { try? repoService?.pullRemote() }
		}) {
			Image(systemName: "arrow.down.to.line")
		}
		.help("Pull")

		Button(action: {
			Task { try? repoService?.pushRemote() }
		}) {
			Image(systemName: "arrow.up.to.line")
		}
		.badge(Int(repoService?.ahead ?? 0))
		.help("Push")
	}
}
