import SwiftUI
import SwiftTerm
import UserNotifications

struct TerminalPanelView: View {
	@Environment(AppState.self) private var appState
	@Environment(TabState.self) private var tabState

	private var terminalService: TerminalService {
		tabState.terminalService
	}

	var body: some View {
		VStack(spacing: 0) {
			// Glass tab bar -- one entry per group
			GlassEffectContainer(spacing: 4) {
				HStack(spacing: 4) {
					ForEach(Array(terminalService.groups.enumerated()), id: \.element.id) { index, group in
						if index == terminalService.activeGroupIndex {
							Button(action: { terminalService.activeGroupIndex = index }) {
								HStack(spacing: 4) {
									Image(systemName: "terminal")
									Text(terminalService.groupTitle(group))
										.font(.caption)
										.lineLimit(1)
								}
							}
							.buttonStyle(.glassProminent)
							.controlSize(.small)
							.overlay {
								if terminalService.groupHasBell(group) {
									SiriGlowBorder()
								}
							}
							.contextMenu {
								Button("Kill Terminal") { terminalService.killGroup(at: index) }
							}
						} else {
							Button(action: {
								terminalService.activeGroupIndex = index
								let sessionIds = group.allSessionIds()
								for sid in sessionIds {
									terminalService.sessionStore[sid]?.hasBell = false
								}
								if terminalService.focusedSessionId == nil || !group.containsSession(terminalService.focusedSessionId!) {
									terminalService.focusedSessionId = group.firstSessionId()
								}
							}) {
								HStack(spacing: 4) {
									Image(systemName: "terminal")
									Text(terminalService.groupTitle(group))
										.font(.caption)
										.lineLimit(1)
								}
							}
							.buttonStyle(.glass)
							.controlSize(.small)
							.overlay {
								if terminalService.groupHasBell(group) {
									SiriGlowBorder()
								}
							}
							.contextMenu {
								Button("Kill Terminal") { terminalService.killGroup(at: index) }
							}
						}
					}

					Button(action: { spawnSession() }) {
						Image(systemName: "plus")
					}
					.buttonStyle(.glass)
					.controlSize(.small)

					Spacer()

					if !terminalService.groups.isEmpty {
						Button(action: { terminalService.killAllSessions() }) {
							Image(systemName: "trash")
						}
						.buttonStyle(.glass)
						.tint(.red)
						.controlSize(.small)
						.help("Kill All")
					}
				}
				.padding(.horizontal, 8)
				.padding(.vertical, 4)
			}

			Divider()

			// Terminal content -- ZStack keeps all group trees alive
			ZStack {
				if terminalService.groups.isEmpty {
					ContentUnavailableView(
						"No Terminal",
						systemImage: "terminal",
						description: Text("Click + to create a new terminal.")
					)
				} else {
					ForEach(Array(terminalService.groups.enumerated()), id: \.element.id) { index, group in
						let isActive = index == terminalService.activeGroupIndex
						TerminalSplitView(
							node: group,
							sessionStore: terminalService.sessionStore,
							themeService: appState.themeService,
							focusedSessionId: terminalService.focusedSessionId,
							splitRatios: Binding(
								get: { terminalService.splitRatios },
								set: { terminalService.splitRatios = $0 }
							),
							onFocus: { sessionId in
								terminalService.focusedSessionId = sessionId
							},
							onSplit: { sessionId, direction in
								terminalService.splitSession(
									sessionId: sessionId,
									direction: direction,
									workingDirectory: tabState.repoService?.status?.root
								)
							},
							onKill: { sessionId in
								terminalService.killSession(sessionId)
							}
						)
						.zIndex(isActive ? 1 : 0)
						.opacity(isActive ? 1 : 0)
						.allowsHitTesting(isActive)
					}
				}
			}
			.padding(.leading, 6)
			.background(
				Color(nsColor: appState.themeService.color(forKey: "terminal.background")
					?? appState.themeService.color(forKey: "editor.background")
					?? .black)
			)
		}
		.onAppear {
			if terminalService.groups.isEmpty {
				spawnSession()
			}
		}
		.onChange(of: tabState.showTerminal, initial: true) { _, visible in
			terminalService.isVisible = visible
		}
		.onReceive(NotificationCenter.default.publisher(for: .newTerminalSession)) { _ in
			spawnSession()
		}
		.onReceive(NotificationCenter.default.publisher(for: .focusTerminalSession)) { notification in
			guard let sessionId = notification.userInfo?["sessionId"] as? UUID else { return }
			guard let groupIdx = terminalService.groups.firstIndex(where: { $0.containsSession(sessionId) }) else { return }
			tabState.showTerminal = true
			terminalService.activeGroupIndex = groupIdx
			terminalService.focusedSessionId = sessionId
			terminalService.sessionStore[sessionId]?.hasBell = false
		}
		.onReceive(NotificationCenter.default.publisher(for: .terminalSessionExited)) { notification in
			guard let sessionId = notification.userInfo?["sessionId"] as? UUID else { return }
			guard terminalService.sessionStore[sessionId] != nil else { return }
			terminalService.killSession(sessionId)
		}
	}

	private func spawnSession() {
		terminalService.spawnSession(
			workingDirectory: tabState.repoService?.status?.root
		)
	}
}

private struct SiriGlowBorder: View {
	@State private var rotation: Double = 0

	private let colors: [SwiftUI.Color] = [
		Color(red: 1.0, green: 0.2, blue: 0.4),
		Color(red: 1.0, green: 0.5, blue: 0.2),
		Color(red: 1.0, green: 0.85, blue: 0.3),
		Color(red: 0.3, green: 0.85, blue: 0.5),
		Color(red: 0.2, green: 0.6, blue: 1.0),
		Color(red: 0.5, green: 0.3, blue: 1.0),
		Color(red: 0.9, green: 0.2, blue: 0.8),
		Color(red: 1.0, green: 0.2, blue: 0.4),
	]

	var body: some View {
		RoundedRectangle(cornerRadius: 6)
			.stroke(
				AngularGradient(
					colors: colors,
					center: .center,
					angle: .degrees(rotation)
				),
				lineWidth: 1.5
			)
			.shadow(color: colors[Int(rotation / 45) % colors.count].opacity(0.6), radius: 4)
			.allowsHitTesting(false)
			.onAppear {
				withAnimation(.linear(duration: 3).repeatForever(autoreverses: false)) {
					rotation = 360
				}
			}
	}
}

private class NotifyingTerminalView: LocalProcessTerminalView {
	var onBell: (() -> Void)?
	var contextMenuProvider: (() -> NSMenu)?

	override func bell(source: Terminal) {
		// bell() is called from SwiftTerm's processing queue.
		// Dispatch the @MainActor mutation, then fall through to system bell.
		DispatchQueue.main.async { [weak self] in
			self?.onBell?()
		}
		super.bell(source: source)
	}

	override func menu(for event: NSEvent) -> NSMenu? {
		contextMenuProvider?()
	}
}

struct SwiftTermContainerView: NSViewRepresentable {
	let session: TerminalSession
	let themeService: ThemeService
	var isActive: Bool
	var isFocused: Bool = false
	var onFocus: (() -> Void)?
	var onSplit: ((SplitDirection) -> Void)?
	var onKill: (() -> Void)?

	@AppStorage("terminal.fontFamily") private var fontFamily = "SF Mono"
	@AppStorage("terminal.fontSize") private var fontSize = 13.0
	@AppStorage("terminal.cursorStyle") private var cursorStyle = "block"
	@AppStorage("terminal.cursorBlink") private var cursorBlink = true
	@AppStorage("terminal.optionAsMeta") private var optionAsMeta = false
	@AppStorage("terminal.mouseReporting") private var mouseReporting = true
	@AppStorage("terminal.scrollback") private var scrollback = 5000
	@AppStorage("terminal.boldAsBright") private var boldAsBright = true
	@AppStorage("terminal.bellNotification") private var bellNotification = true

	func makeNSView(context: Context) -> LocalProcessTerminalView {
		let terminalView = NotifyingTerminalView(frame: .zero)
		let coordinator = context.coordinator
		terminalView.onBell = { [weak coordinator] in
			coordinator?.handleBell()
		}
		terminalView.contextMenuProvider = { [weak coordinator] in
			coordinator?.buildContextMenu() ?? NSMenu()
		}
		terminalView.processDelegate = context.coordinator
		context.coordinator.terminalView = terminalView

		// Configure scrollback and cursor on the existing terminal instance
		terminalView.terminal.changeHistorySize(scrollback)
		terminalView.terminal.options.cursorStyle = resolveCursorStyle()

		// Register OSC 9 handler for desktop notifications (used by Claude Code, iTerm2 protocol)
		terminalView.terminal.registerOscHandler(code: 9) { [weak coordinator] (data: ArraySlice<UInt8>) in
			guard let text = String(bytes: data, encoding: .utf8) else { return }
			if text.hasPrefix("4;") { return }
			coordinator?.handleOscNotification(text)
		}

		// Configure appearance
		terminalView.font = resolveFont()
		applyThemeColors(to: terminalView)

		// Apply runtime settings
		terminalView.optionAsMetaKey = optionAsMeta
		terminalView.allowMouseReporting = mouseReporting
		terminalView.useBrightColors = boldAsBright

		// Start shell process with the full resolved environment.
		// TERM_PROGRAM and related vars are stripped: Claude Code and other TUI apps
		// detect recognized terminals (iTerm2, Ghostty, etc.) and activate rendering
		// features (Unicode width tables, escape sequences) that SwiftTerm doesn't support.
		let shell = ProcessInfo.processInfo.environment["SHELL"] ?? "/bin/zsh"
		let shellName = (shell as NSString).lastPathComponent

		let resolvedEnv = getResolvedEnvironment()
		let env: [String]
		if resolvedEnv.isEmpty {
			env = Terminal.getEnvironmentVariables(termName: "xterm-256color")
		} else {
			var vars = resolvedEnv
			vars["TERM"] = "xterm-256color"
			// Strip terminal identity vars that cause TUI apps to use
			// rendering features SwiftTerm doesn't support
			vars.removeValue(forKey: "TERM_PROGRAM")
			vars.removeValue(forKey: "TERM_PROGRAM_VERSION")
			for key in vars.keys where key.hasPrefix("GHOSTTY_") {
				vars.removeValue(forKey: key)
			}
			env = vars.map { "\($0.key)=\($0.value)" }
		}

		let shellArgs: [String] = (shellName == "zsh" || shellName == "bash") ? ["--login"] : []

		terminalView.startProcess(
			executable: shell,
			args: shellArgs,
			environment: env,
			execName: "-" + shellName,
			currentDirectory: session.workingDirectory
		)

		session.shellPid = terminalView.process.shellPid
		session.shellName = (shell as NSString).lastPathComponent
		context.coordinator.monitorProcess(pid: terminalView.process.shellPid)

		context.coordinator.lastAppliedThemeName = themeService.currentThemeName
		context.coordinator.lastAppliedFontKey = "\(fontFamily)-\(fontSize)"
		context.coordinator.lastAppliedCursorKey = "\(cursorStyle)-\(cursorBlink)"
		context.coordinator.lastAppliedOptionAsMeta = optionAsMeta
		context.coordinator.lastAppliedMouseReporting = mouseReporting
		context.coordinator.lastAppliedBoldAsBright = boldAsBright
		context.coordinator.bellNotification = bellNotification
		context.coordinator.onFocus = onFocus
		context.coordinator.onSplit = onSplit
		context.coordinator.onKill = onKill
		return terminalView
	}

	func updateNSView(_ nsView: LocalProcessTerminalView, context: Context) {
		if themeService.currentThemeName != context.coordinator.lastAppliedThemeName {
			applyThemeColors(to: nsView)
			context.coordinator.lastAppliedThemeName = themeService.currentThemeName
		}

		let currentFontKey = "\(fontFamily)-\(fontSize)"
		if currentFontKey != context.coordinator.lastAppliedFontKey {
			nsView.font = resolveFont()
			context.coordinator.lastAppliedFontKey = currentFontKey
		}

		let currentCursorKey = "\(cursorStyle)-\(cursorBlink)"
		if currentCursorKey != context.coordinator.lastAppliedCursorKey {
			applyCursorStyle(to: nsView)
			context.coordinator.lastAppliedCursorKey = currentCursorKey
		}

		if optionAsMeta != context.coordinator.lastAppliedOptionAsMeta {
			nsView.optionAsMetaKey = optionAsMeta
			context.coordinator.lastAppliedOptionAsMeta = optionAsMeta
		}

		if mouseReporting != context.coordinator.lastAppliedMouseReporting {
			nsView.allowMouseReporting = mouseReporting
			context.coordinator.lastAppliedMouseReporting = mouseReporting
		}

		if boldAsBright != context.coordinator.lastAppliedBoldAsBright {
			nsView.useBrightColors = boldAsBright
			context.coordinator.lastAppliedBoldAsBright = boldAsBright
		}

		context.coordinator.bellNotification = bellNotification
		context.coordinator.isActiveSession = isActive
		context.coordinator.onFocus = onFocus
		context.coordinator.onSplit = onSplit
		context.coordinator.onKill = onKill
	}

	private func resolveFont() -> NSFont {
		NSFont(name: fontFamily, size: CGFloat(fontSize))
			?? NSFont.monospacedSystemFont(ofSize: CGFloat(fontSize), weight: .regular)
	}

	private func resolveCursorStyle() -> CursorStyle {
		switch cursorStyle {
		case "underline": return cursorBlink ? .blinkUnderline : .steadyUnderline
		case "bar": return cursorBlink ? .blinkBar : .steadyBar
		default: return cursorBlink ? .blinkBlock : .steadyBlock
		}
	}

	/// Send DECSCUSR escape sequence to change cursor style at runtime
	private func applyCursorStyle(to terminalView: LocalProcessTerminalView) {
		let code: Int
		switch cursorStyle {
		case "underline": code = cursorBlink ? 3 : 4
		case "bar": code = cursorBlink ? 5 : 6
		default: code = cursorBlink ? 1 : 2
		}
		terminalView.feed(text: "\u{1b}[\(code) q")
	}

	func makeCoordinator() -> Coordinator {
		Coordinator(session: session)
	}

	private func applyThemeColors(to terminalView: LocalProcessTerminalView) {
		terminalView.nativeBackgroundColor = themeService.color(forKey: "terminal.background")
			?? themeService.color(forKey: "editor.background")
			?? .black
		terminalView.nativeForegroundColor = themeService.color(forKey: "terminal.foreground")
			?? themeService.color(forKey: "editor.foreground")
			?? .white
		if let components = themeService.terminalANSIColorComponents {
			installTerminalANSIColors(components, on: terminalView)
		}
	}

	class Coordinator: NSObject, LocalProcessTerminalViewDelegate {
		let session: TerminalSession
		var lastAppliedThemeName: String?
		var lastAppliedFontKey: String?
		var lastAppliedCursorKey: String?
		var lastAppliedOptionAsMeta: Bool?
		var lastAppliedMouseReporting: Bool?
		var lastAppliedBoldAsBright: Bool?
		var bellNotification = true
		var isActiveSession = false
		weak var terminalView: LocalProcessTerminalView?
		var onFocus: (() -> Void)?
		var onSplit: ((SplitDirection) -> Void)?
		var onKill: (() -> Void)?

		private var findObserver: NSObjectProtocol?
		private var mouseMonitor: Any?
		private var processSource: DispatchSourceProcess?

		init(session: TerminalSession) {
			self.session = session
			super.init()
			findObserver = NotificationCenter.default.addObserver(
				forName: .findInTerminal,
				object: nil,
				queue: .main
			) { [weak self] notification in
				guard let self,
					  let sessionId = notification.userInfo?["sessionId"] as? UUID,
					  sessionId == self.session.id else { return }
				self.showFindBar()
			}
			mouseMonitor = NSEvent.addLocalMonitorForEvents(matching: [.leftMouseDown, .rightMouseDown]) { [weak self] event in
				guard let self, let terminalView = self.terminalView else { return event }
				let locationInView = terminalView.convert(event.locationInWindow, from: nil)
				if terminalView.bounds.contains(locationInView) {
					self.handleFocus()
				}
				return event
			}
		}

		func monitorProcess(pid: pid_t) {
			guard pid > 0 else { return }
			let sessionId = session.id
			let source = DispatchSource.makeProcessSource(identifier: pid, eventMask: .exit, queue: .main)
			source.setEventHandler {
				NotificationCenter.default.post(
					name: .terminalSessionExited,
					object: nil,
					userInfo: ["sessionId": sessionId]
				)
			}
			source.resume()
			processSource = source
		}

		deinit {
			processSource?.cancel()
			if let findObserver {
				NotificationCenter.default.removeObserver(findObserver)
			}
			if let mouseMonitor {
				NSEvent.removeMonitor(mouseMonitor)
			}
		}

		func showFindBar() {
			guard let terminalView else { return }
			let item = NSMenuItem()
			item.tag = Int(NSFindPanelAction.showFindPanel.rawValue)
			terminalView.performFindPanelAction(item)
		}

		func handleBell() {
			session.hasBell = true
			guard bellNotification else { return }
			guard !NSApp.isActive || !isActiveSession else { return }
			sendNotification(
				title: "Terminal",
				body: "\(session.title) needs attention"
			)
		}

		func handleOscNotification(_ message: String) {
			// Called from SwiftTerm's processing queue via OSC handler.
			// Dispatch @MainActor mutations to the main thread.
			DispatchQueue.main.async { [weak self] in
				guard let self else { return }
				self.session.hasBell = true
				guard self.bellNotification else { return }
				guard !NSApp.isActive || !self.isActiveSession else { return }
				self.sendNotification(
					title: self.session.title,
					body: message
				)
			}
		}

		func handleFocus() {
			onFocus?()
		}

		func buildContextMenu() -> NSMenu {
			let menu = NSMenu()

			let copyItem = NSMenuItem(title: "Copy", action: #selector(performCopy(_:)), keyEquivalent: "")
			copyItem.target = self
			menu.addItem(copyItem)

			let pasteItem = NSMenuItem(title: "Paste", action: #selector(performPaste(_:)), keyEquivalent: "")
			pasteItem.target = self
			menu.addItem(pasteItem)

			menu.addItem(NSMenuItem.separator())

			let splitRight = NSMenuItem(title: "Split Right", action: #selector(splitTerminalRight(_:)), keyEquivalent: "")
			splitRight.target = self
			menu.addItem(splitRight)

			let splitLeft = NSMenuItem(title: "Split Left", action: #selector(splitTerminalLeft(_:)), keyEquivalent: "")
			splitLeft.target = self
			menu.addItem(splitLeft)

			let splitDown = NSMenuItem(title: "Split Down", action: #selector(splitTerminalDown(_:)), keyEquivalent: "")
			splitDown.target = self
			menu.addItem(splitDown)

			let splitUp = NSMenuItem(title: "Split Up", action: #selector(splitTerminalUp(_:)), keyEquivalent: "")
			splitUp.target = self
			menu.addItem(splitUp)

			menu.addItem(NSMenuItem.separator())

			let killItem = NSMenuItem(title: "Kill Terminal", action: #selector(killTerminal(_:)), keyEquivalent: "")
			killItem.target = self
			menu.addItem(killItem)

			return menu
		}

		@objc func performCopy(_ sender: Any?) {
			terminalView?.copy(sender)
		}

		@objc func performPaste(_ sender: Any?) {
			terminalView?.paste(sender)
		}

		@objc func splitTerminalRight(_ sender: Any?) {
			onSplit?(.right)
		}

		@objc func splitTerminalLeft(_ sender: Any?) {
			onSplit?(.left)
		}

		@objc func splitTerminalDown(_ sender: Any?) {
			onSplit?(.down)
		}

		@objc func splitTerminalUp(_ sender: Any?) {
			onSplit?(.up)
		}

		@objc func killTerminal(_ sender: Any?) {
			onKill?()
		}

		nonisolated func sizeChanged(source: LocalProcessTerminalView, newCols: Int, newRows: Int) {}

		nonisolated func setTerminalTitle(source: LocalProcessTerminalView, title: String) {
			Task { @MainActor in
				session.titleSetByEscape = Date()
				session.title = title.isEmpty ? "zsh" : title
			}
		}

		nonisolated func hostCurrentDirectoryUpdate(source: TerminalView, directory: String?) {}

		nonisolated func processTerminated(source: TerminalView, exitCode: Int32?) {
			let sessionId = session.id
			Task { @MainActor in
				NotificationCenter.default.post(
					name: .terminalSessionExited,
					object: nil,
					userInfo: ["sessionId": sessionId]
				)
			}
		}

		private func sendNotification(title: String, body: String) {
			let content = UNMutableNotificationContent()
			content.title = title
			content.body = body
			content.sound = .default
			content.userInfo = ["sessionId": session.id.uuidString]
			let request = UNNotificationRequest(
				identifier: UUID().uuidString,
				content: content,
				trigger: nil
			)
			UNUserNotificationCenter.current().add(request)
		}
	}
}
