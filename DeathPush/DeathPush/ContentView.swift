import SwiftUI

struct WindowContentView: View {
	@Environment(AppState.self) private var appState
	@State private var tabState = TabState()

	var body: some View {
		Group {
			if tabState.repoService != nil {
				RepositoryView()
			} else {
				WelcomeScreenView()
			}
		}
		.environment(tabState)
		.background(WindowTabbingConfigurator())
		.alert("Error", isPresented: hasError) {
			Button("OK") { tabState.errorMessage = nil }
		} message: {
			Text(tabState.errorMessage ?? "")
		}
		.navigationTitle(tabState.repoService != nil ? tabState.repoName : "DeathPush")
		.focusedSceneValue(\.activeTabState, tabState)
		.onDisappear {
			tabState.cleanup()
		}
		.onReceive(NotificationCenter.default.publisher(for: .openRepositoryDeepLink)) { notification in
			guard let path = notification.userInfo?["path"] as? String else { return }
			Task { await tabState.openRepository(path: path, appState: appState) }
		}
	}

	private var hasError: Binding<Bool> {
		Binding(
			get: { tabState.errorMessage != nil },
			set: { if !$0 { tabState.errorMessage = nil } }
		)
	}
}

private struct WindowTabbingConfigurator: NSViewRepresentable {
	func makeNSView(context: Context) -> NSView {
		let view = NSView()
		DispatchQueue.main.async {
			view.window?.tabbingMode = .preferred
		}
		return view
	}

	func updateNSView(_ nsView: NSView, context: Context) {}
}
