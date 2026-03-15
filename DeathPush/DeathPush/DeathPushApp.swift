import SwiftUI
import UserNotifications

final class DeathPushAppDelegate: NSObject, NSApplicationDelegate, UNUserNotificationCenterDelegate {
	func applicationDidFinishLaunching(_ notification: Notification) {
		NSWindow.allowsAutomaticWindowTabbing = true
		let center = UNUserNotificationCenter.current()
		center.delegate = self
		center.requestAuthorization(options: [.alert, .sound]) { _, _ in }
	}

	func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
		guard TabState.hasActiveTerminals else { return .terminateNow }

		let alert = NSAlert()
		alert.messageText = "Quit DeathPush?"
		alert.informativeText = "There are active terminal sessions. Quitting will terminate them."
		alert.alertStyle = .warning
		alert.addButton(withTitle: "Quit")
		alert.addButton(withTitle: "Cancel")

		let response = alert.runModal()
		return response == .alertFirstButtonReturn ? .terminateNow : .terminateCancel
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
			DispatchQueue.main.async {
				NotificationCenter.default.post(
					name: .focusTerminalSession,
					object: nil,
					userInfo: ["sessionId": sessionId]
				)
			}
		}
		completionHandler()
	}
}

@main
struct DeathPushApp: App {
	@NSApplicationDelegateAdaptor(DeathPushAppDelegate.self) var appDelegate
	@State private var appState = AppState()
	@State private var showLicenses = false
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
				.sheet(isPresented: $showLicenses) {
					LicensesView()
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

			CommandGroup(replacing: .help) {
				Button("Open Source Licenses") {
					showLicenses = true
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
	@FocusedValue(\.showRecents) private var showRecents
	@FocusedValue(\.showWorkspace) private var showWorkspace
	@FocusedValue(\.sidebarSelection) private var sidebarSelection
	@FocusedValue(\.focusWelcomeFilter) private var focusWelcomeFilter
	@FocusedValue(\.activeTabState) private var tabState

	private var terminalIsFocused: Bool {
		tabState?.showTerminal == true && tabState?.terminalService.focusedSessionId != nil
	}

	var body: some Commands {
		CommandGroup(replacing: .saveItem) {
			Button("Close Terminal Tab") {
				if let id = tabState?.terminalService.focusedSessionId {
					tabState?.terminalService.killSession(id)
				}
			}
			.keyboardShortcut("w", modifiers: .command)
			.disabled(!terminalIsFocused)
		}

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

			Button("Open Repository...") {
				let panel = NSOpenPanel()
				panel.canChooseFiles = false
				panel.canChooseDirectories = true
				panel.allowsMultipleSelection = false
				panel.message = "Select a Git repository"
				panel.prompt = "Open"

				if panel.runModal() == .OK, let url = panel.url {
					NotificationCenter.default.post(
						name: .openRepositoryDeepLink,
						object: nil,
						userInfo: ["path": url.path(percentEncoded: false)]
					)
				}
			}
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
			.disabled(showQuickOpen == nil)

			Button("Color Theme") {
				showThemePicker?.wrappedValue = true
			}
			.keyboardShortcut("t", modifiers: [.command, .shift])
			.disabled(showThemePicker == nil)

			Divider()

			Button("Recents") {
				showRecents?.wrappedValue = true
			}
			.keyboardShortcut("r", modifiers: [.command, .control])
			.disabled(showRecents == nil)

			Button("Workspace") {
				showWorkspace?.wrappedValue = true
			}
			.keyboardShortcut("e", modifiers: [.command, .control])
			.disabled(showWorkspace == nil)

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
			.disabled(sidebarSelection == nil)
		}

		CommandMenu("Git") {
			Button("Fetch") {
				try? tabState?.repoService?.fetchRemote()
			}
			.keyboardShortcut("f", modifiers: [.command, .shift])
			.disabled(tabState?.repoService == nil)

			Button("Pull") {
				try? tabState?.repoService?.pullRemote()
			}
			.keyboardShortcut("p", modifiers: [.command, .shift])
			.disabled(tabState?.repoService == nil)

			Button("Push") {
				try? tabState?.repoService?.pushRemote()
			}
			.keyboardShortcut("u", modifiers: [.command, .shift])
			.disabled(tabState?.repoService == nil)

			Divider()

			Button("Stage All") {
				try? tabState?.repoService?.stageAllFiles()
			}
			.keyboardShortcut("a", modifiers: [.command, .shift])
			.disabled(tabState?.repoService == nil)

			Button("Unstage All") {
				try? tabState?.repoService?.unstageAllFiles()
			}
			.disabled(tabState?.repoService == nil)

			Divider()

			Button("Stash") {
				try? tabState?.repoService?.saveStash()
			}
			.disabled(tabState?.repoService == nil)

			Button("Stash Pop") {
				try? tabState?.repoService?.popStash(index: 0)
			}
			.disabled(tabState?.repoService == nil)

			Divider()

			Button("Undo Last Commit") {
				try? tabState?.repoService?.undoCommit()
			}
			.disabled(tabState?.repoService == nil)
		}

		CommandMenu("Terminal") {
			Button("New Terminal") {
				if tabState?.showTerminal == false {
					tabState?.showTerminal = true
				}
				NotificationCenter.default.post(name: .newTerminalSession, object: nil)
			}
			.keyboardShortcut("j", modifiers: [.command, .shift])
			.disabled(tabState?.repoService == nil)

			Button("Toggle Terminal") {
				tabState?.showTerminal.toggle()
			}
			.keyboardShortcut("j", modifiers: .command)
			.disabled(tabState?.repoService == nil)

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
			.disabled(tabState?.showTerminal != true || tabState?.terminalService.focusedSessionId == nil)
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

struct ShowRecentsKey: FocusedValueKey {
	typealias Value = Binding<Bool>
}

struct ShowWorkspaceKey: FocusedValueKey {
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

	var showRecents: Binding<Bool>? {
		get { self[ShowRecentsKey.self] }
		set { self[ShowRecentsKey.self] = newValue }
	}

	var showWorkspace: Binding<Bool>? {
		get { self[ShowWorkspaceKey.self] }
		set { self[ShowWorkspaceKey.self] = newValue }
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
