import SwiftUI
import SwiftTerm

struct TerminalPanelView: View {
	@Environment(AppState.self) private var appState
	@Environment(TabState.self) private var tabState

	private var terminalService: TerminalService {
		tabState.terminalService
	}

	var body: some View {
		VStack(spacing: 0) {
			// Glass tab bar
			GlassEffectContainer(spacing: 4) {
				HStack(spacing: 4) {
					ForEach(terminalService.sessions) { session in
						if session.id == terminalService.activeSessionId {
							Button(action: { terminalService.activeSessionId = session.id }) {
								HStack(spacing: 4) {
									Image(systemName: "terminal")
									Text(session.title)
										.font(.caption)
										.lineLimit(1)
								}
							}
							.buttonStyle(.glassProminent)
							.controlSize(.small)
							.contextMenu {
								Button("Kill Terminal") { terminalService.killSession(session.id) }
							}
						} else {
							Button(action: { terminalService.activeSessionId = session.id }) {
								HStack(spacing: 4) {
									Image(systemName: "terminal")
									Text(session.title)
										.font(.caption)
										.lineLimit(1)
								}
							}
							.buttonStyle(.glass)
							.controlSize(.small)
							.contextMenu {
								Button("Kill Terminal") { terminalService.killSession(session.id) }
							}
						}
					}

					Button(action: { spawnSession() }) {
						Image(systemName: "plus")
					}
					.buttonStyle(.glass)
					.controlSize(.small)

					Spacer()

					if !terminalService.sessions.isEmpty {
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

			// Terminal content -- ZStack keeps all NSViews alive
			ZStack {
				if terminalService.sessions.isEmpty {
					ContentUnavailableView(
						"No Terminal",
						systemImage: "terminal",
						description: Text("Click + to create a new terminal.")
					)
				} else {
					ForEach(terminalService.sessions) { session in
						SwiftTermContainerView(session: session, themeService: appState.themeService)
							.opacity(session.id == terminalService.activeSessionId ? 1 : 0)
							.allowsHitTesting(session.id == terminalService.activeSessionId)
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
			if terminalService.sessions.isEmpty {
				spawnSession()
			}
		}
	}

	private func spawnSession() {
		terminalService.spawnSession(
			workingDirectory: tabState.repoService?.status?.root
		)
	}
}

struct SwiftTermContainerView: NSViewRepresentable {
	let session: TerminalSession
	let themeService: ThemeService

	func makeNSView(context: Context) -> LocalProcessTerminalView {
		let terminalView = LocalProcessTerminalView(frame: .zero)
		terminalView.processDelegate = context.coordinator

		// Configure appearance
		terminalView.font = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
		applyThemeColors(to: terminalView)

		// Start shell process
		let shell = ProcessInfo.processInfo.environment["SHELL"] ?? "/bin/zsh"
		let env = Terminal.getEnvironmentVariables(termName: "xterm-256color")

		terminalView.startProcess(
			executable: shell,
			args: [],
			environment: env,
			execName: "-" + (shell as NSString).lastPathComponent,
			currentDirectory: session.workingDirectory
		)

		context.coordinator.lastAppliedThemeName = themeService.currentThemeName
		return terminalView
	}

	func updateNSView(_ nsView: LocalProcessTerminalView, context: Context) {
		if themeService.currentThemeName != context.coordinator.lastAppliedThemeName {
			applyThemeColors(to: nsView)
			context.coordinator.lastAppliedThemeName = themeService.currentThemeName
		}
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

		init(session: TerminalSession) {
			self.session = session
		}

		nonisolated func sizeChanged(source: LocalProcessTerminalView, newCols: Int, newRows: Int) {}

		nonisolated func setTerminalTitle(source: LocalProcessTerminalView, title: String) {
			Task { @MainActor in
				session.title = title.isEmpty ? "zsh" : title
			}
		}

		nonisolated func hostCurrentDirectoryUpdate(source: TerminalView, directory: String?) {}
		nonisolated func processTerminated(source: TerminalView, exitCode: Int32?) {}
	}
}
