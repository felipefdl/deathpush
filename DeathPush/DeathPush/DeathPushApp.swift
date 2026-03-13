import SwiftUI
import UserNotifications

final class DeathPushAppDelegate: NSObject, NSApplicationDelegate, UNUserNotificationCenterDelegate {
	func applicationDidFinishLaunching(_ notification: Notification) {
		NSWindow.allowsAutomaticWindowTabbing = true
		let center = UNUserNotificationCenter.current()
		center.delegate = self
		center.requestAuthorization(options: [.alert, .sound]) { _, _ in }
	}

	func userNotificationCenter(
		_ center: UNUserNotificationCenter,
		didReceive response: UNNotificationResponse,
		withCompletionHandler completionHandler: @escaping () -> Void
	) {
		let userInfo = response.notification.request.content.userInfo
		if let sessionIdString = userInfo["sessionId"] as? String,
		   let sessionId = UUID(uuidString: sessionIdString) {
			NSApp.activate(ignoringOtherApps: true)
			NotificationCenter.default.post(
				name: .focusTerminalSession,
				object: nil,
				userInfo: ["sessionId": sessionId]
			)
		}
		completionHandler()
	}
}

@main
struct DeathPushApp: App {
	@NSApplicationDelegateAdaptor(DeathPushAppDelegate.self) var appDelegate
	@State private var appState = AppState()
	private let updater = AppUpdater()

	nonisolated init() {
		initialize()
	}

	var body: some Scene {
		WindowGroup(id: "main") {
			WindowContentView()
				.frame(minWidth: 700, minHeight: 400)
				.environment(appState)
				.preferredColorScheme(appState.themeService.colorScheme)
				.onAppear {
					let bridge = EventBridge(gitOutputService: appState.gitOutputService)
					registerEventListener(listener: bridge)
				}
				.onOpenURL { url in
					handleDeepLink(url)
				}
		}
		.defaultSize(width: 990, height: 700)
		.commands {
			DeathPushMenuCommands()

			CommandGroup(after: .appInfo) {
				Button("Check for Updates...") {
					updater.checkForUpdates()
				}
			}
		}

		Settings {
			SettingsView()
				.environment(appState)
				.preferredColorScheme(appState.themeService.colorScheme)
		}
	}

	private func handleDeepLink(_ url: URL) {
		guard url.scheme == "deathpush" else { return }

		var repoPath = url.absoluteString
			.replacingOccurrences(of: "deathpush://", with: "")
			.removingPercentEncoding ?? ""

		guard !repoPath.isEmpty else { return }

		if !repoPath.hasPrefix("/") {
			repoPath = "/" + repoPath
		}

		NotificationCenter.default.post(
			name: .openRepositoryDeepLink,
			object: nil,
			userInfo: ["path": repoPath]
		)
	}
}

extension Notification.Name {
	static let openRepositoryDeepLink = Notification.Name("deathpush.openRepositoryDeepLink")
	static let cloneRepository = Notification.Name("deathpush.cloneRepository")
	static let newTerminalSession = Notification.Name("deathpush.newTerminalSession")
	static let findInTerminal = Notification.Name("deathpush.findInTerminal")
	static let focusTerminalSession = Notification.Name("deathpush.focusTerminalSession")
	static let terminalSessionExited = Notification.Name("deathpush.terminalSessionExited")
}

struct DeathPushMenuCommands: Commands {
	@FocusedValue(\.showQuickOpen) private var showQuickOpen
	@FocusedValue(\.showThemePicker) private var showThemePicker
	@FocusedValue(\.sidebarSelection) private var sidebarSelection
	@FocusedValue(\.focusWelcomeFilter) private var focusWelcomeFilter
	@FocusedValue(\.activeTabState) private var tabState

	var body: some Commands {
		CommandGroup(before: .newItem) {
			Button("New Tab") {
				guard let currentWindow = NSApp.keyWindow,
					  let windowController = currentWindow.windowController else { return }
				windowController.newWindowForTab(nil)
				guard let newWindow = NSApp.keyWindow, newWindow != currentWindow else { return }
				currentWindow.addTabbedWindow(newWindow, ordered: .above)
				newWindow.makeKeyAndOrderFront(nil)
			}
			.keyboardShortcut("t", modifiers: .command)

			Divider()
		}

		CommandGroup(after: .newItem) {
			Divider()

			Button("Open Repository...") {}
				.keyboardShortcut("o", modifiers: .command)

			Button("Clone Repository...") {
				NotificationCenter.default.post(name: .cloneRepository, object: nil)
			}
			.keyboardShortcut("c", modifiers: [.command, .shift])

			Divider()

			Button("Install Command Line Tool...") {
				CLIInstaller.install()
			}
		}

		CommandMenu("View") {
			Button("Quick Open") {
				showQuickOpen?.wrappedValue = true
			}
			.keyboardShortcut("p", modifiers: .command)

			Button("Color Theme") {
				showThemePicker?.wrappedValue = true
			}
			.keyboardShortcut("t", modifiers: [.command, .shift])

			Divider()

			Button("Changes") {
				if let sidebarSelection {
					sidebarSelection.wrappedValue = .changes
				} else {
					focusWelcomeFilter?(.recent)
				}
			}
			.keyboardShortcut("1", modifiers: .command)

			Button("History") {
				if let sidebarSelection {
					sidebarSelection.wrappedValue = .history
				} else {
					focusWelcomeFilter?(.workspace)
				}
			}
			.keyboardShortcut("2", modifiers: .command)

			Button("Explorer") {
				sidebarSelection?.wrappedValue = .explorer
			}
			.keyboardShortcut("3", modifiers: .command)
		}

		CommandMenu("Git") {
			Button("Fetch") {
				try? tabState?.repoService?.fetchRemote()
			}
			.keyboardShortcut("f", modifiers: [.command, .shift])

			Button("Pull") {
				try? tabState?.repoService?.pullRemote()
			}
			.keyboardShortcut("p", modifiers: [.command, .shift])

			Button("Push") {
				try? tabState?.repoService?.pushRemote()
			}
			.keyboardShortcut("u", modifiers: [.command, .shift])

			Divider()

			Button("Stage All") {
				try? tabState?.repoService?.stageAllFiles()
			}
			.keyboardShortcut("a", modifiers: [.command, .shift])

			Button("Unstage All") {
				try? tabState?.repoService?.unstageAllFiles()
			}

			Divider()

			Button("Stash") {
				try? tabState?.repoService?.saveStash()
			}

			Button("Stash Pop") {
				try? tabState?.repoService?.popStash(index: 0)
			}

			Divider()

			Button("Undo Last Commit") {
				try? tabState?.repoService?.undoCommit()
			}
		}

		CommandMenu("Terminal") {
			Button("New Terminal") {
				if tabState?.showTerminal == false {
					tabState?.showTerminal = true
				}
				NotificationCenter.default.post(name: .newTerminalSession, object: nil)
			}
			.keyboardShortcut("j", modifiers: [.command, .shift])

			Button("Toggle Terminal") {
				tabState?.showTerminal.toggle()
			}
			.keyboardShortcut("j", modifiers: .command)

			Divider()

			Button("Find in Terminal") {
				guard let state = tabState,
					  state.showTerminal,
					  let sessionId = state.terminalService.focusedSessionId else { return }
				NotificationCenter.default.post(
					name: .findInTerminal,
					object: nil,
					userInfo: ["sessionId": sessionId]
				)
			}
			.keyboardShortcut("f", modifiers: .command)
			.disabled(tabState?.showTerminal != true || tabState?.terminalService.focusedSessionId == nil)

			Divider()

			Button("Kill Terminal") {
				if let id = tabState?.terminalService.focusedSessionId {
					tabState?.terminalService.killSession(id)
				}
			}
			.keyboardShortcut("w", modifiers: [.command, .shift])
			.disabled(tabState?.terminalService.focusedSessionId == nil)
		}
	}
}

// MARK: - Focused Values

struct ShowQuickOpenKey: FocusedValueKey {
	typealias Value = Binding<Bool>
}

struct ShowThemePickerKey: FocusedValueKey {
	typealias Value = Binding<Bool>
}

struct SidebarSelectionKey: FocusedValueKey {
	typealias Value = Binding<SidebarItem>
}

struct ActiveTabStateKey: FocusedValueKey {
	typealias Value = TabState
}

extension FocusedValues {
	var showQuickOpen: Binding<Bool>? {
		get { self[ShowQuickOpenKey.self] }
		set { self[ShowQuickOpenKey.self] = newValue }
	}

	var showThemePicker: Binding<Bool>? {
		get { self[ShowThemePickerKey.self] }
		set { self[ShowThemePickerKey.self] = newValue }
	}

	var sidebarSelection: Binding<SidebarItem>? {
		get { self[SidebarSelectionKey.self] }
		set { self[SidebarSelectionKey.self] = newValue }
	}

	var activeTabState: TabState? {
		get { self[ActiveTabStateKey.self] }
		set { self[ActiveTabStateKey.self] = newValue }
	}
}
