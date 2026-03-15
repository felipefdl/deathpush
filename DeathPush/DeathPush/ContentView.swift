import SwiftUI

struct WindowContentView: View {
	@Environment(AppState.self) private var appState
	@State private var tabState = TabState()
	@State private var toastMessage: String?

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
		.background(WindowCloseGuard(shouldWarn: {
			!tabState.terminalService.sessions.isEmpty
		}))
		.overlay(alignment: .top) {
			if let message = toastMessage {
				ErrorToastView(message: message) {
					toastMessage = nil
				}
				.transition(.move(edge: .top).combined(with: .opacity))
				.padding(.top, 8)
			}
		}
		.onChange(of: tabState.errorMessage) { _, newValue in
			if let error = newValue {
				toastMessage = error
				tabState.errorMessage = nil
			}
		}
		.onChange(of: tabState.repoService?.operationError) { _, newValue in
			if let error = newValue {
				toastMessage = error
				tabState.repoService?.operationError = nil
			}
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

private struct WindowCloseGuard: NSViewRepresentable {
	let shouldWarn: () -> Bool

	func makeNSView(context: Context) -> NSView {
		let view = NSView()
		DispatchQueue.main.async {
			guard let window = view.window else { return }
			context.coordinator.originalDelegate = window.delegate
			window.delegate = context.coordinator
		}
		return view
	}

	func updateNSView(_ nsView: NSView, context: Context) {
		context.coordinator.shouldWarn = shouldWarn
	}

	func makeCoordinator() -> Coordinator {
		Coordinator()
	}

	class Coordinator: NSObject, NSWindowDelegate {
		weak var originalDelegate: (any NSWindowDelegate)?
		var shouldWarn: (() -> Bool)?

		func windowShouldClose(_ sender: NSWindow) -> Bool {
			guard shouldWarn?() == true else { return true }

			let alert = NSAlert()
			alert.messageText = "Close window?"
			alert.informativeText = "There are active terminal sessions. Closing will terminate them."
			alert.alertStyle = .warning
			alert.addButton(withTitle: "Close")
			alert.addButton(withTitle: "Cancel")

			let response = alert.runModal()
			return response == .alertFirstButtonReturn
		}

		func windowWillClose(_ notification: Notification) {
			originalDelegate?.windowWillClose?(notification)
		}

		func windowDidBecomeKey(_ notification: Notification) {
			originalDelegate?.windowDidBecomeKey?(notification)
		}

		func windowDidResignKey(_ notification: Notification) {
			originalDelegate?.windowDidResignKey?(notification)
		}
	}
}
