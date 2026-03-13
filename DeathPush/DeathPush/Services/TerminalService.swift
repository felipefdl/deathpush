import Foundation
import SwiftUI
import UserNotifications

@MainActor @Observable
final class TerminalSession: Identifiable {
	let id = UUID()
	var title = "zsh"
	let workingDirectory: String?
	var shellPid: pid_t = 0
	var shellName: String = "zsh"
	var titleSetByEscape: Date?
	var commandStartTime: Date?
	var lastCommandName: String?
	var hasBell = false

	init(workingDirectory: String? = nil) {
		self.workingDirectory = workingDirectory
	}
}

@MainActor @Observable
final class TerminalService {
	private(set) var sessionStore: [UUID: TerminalSession] = [:]
	var groups: [TerminalSplitNode] = []
	var activeGroupIndex: Int?
	var focusedSessionId: UUID?
	var splitRatios: [UUID: CGFloat] = [:]

	var sessions: [TerminalSession] { Array(sessionStore.values) }

	var activeGroup: TerminalSplitNode? {
		guard let idx = activeGroupIndex, groups.indices.contains(idx) else { return nil }
		return groups[idx]
	}

	private var pollingTask: Task<Void, Never>?
	var isVisible: Bool = false {
		didSet {
			guard isVisible != oldValue else { return }
			if isVisible {
				startPolling()
			} else {
				stopPolling()
			}
		}
	}

	func spawnSession(workingDirectory: String? = nil) {
		let session = TerminalSession(workingDirectory: workingDirectory)
		sessionStore[session.id] = session
		let node = TerminalSplitNode.leaf(id: UUID(), sessionId: session.id)
		groups.append(node)
		activeGroupIndex = groups.count - 1
		focusedSessionId = session.id
	}

	func killSession(_ id: UUID) {
		guard let groupIdx = groups.firstIndex(where: { $0.containsSession(id) }) else { return }

		let sibling = groups[groupIdx].siblingSessionId(of: id)
		if let newTree = groups[groupIdx].removeLeaf(sessionId: id) {
			groups[groupIdx] = newTree
			if focusedSessionId == id {
				focusedSessionId = sibling ?? newTree.firstSessionId()
			}
		} else {
			groups.remove(at: groupIdx)
			if groups.isEmpty {
				activeGroupIndex = nil
				focusedSessionId = nil
				stopPolling()
			} else {
				let newIdx = min(groupIdx, groups.count - 1)
				activeGroupIndex = newIdx
				focusedSessionId = groups[newIdx].firstSessionId()
			}
		}

		sessionStore.removeValue(forKey: id)
		pruneOrphanedRatios()
	}

	func killGroup(at index: Int) {
		guard groups.indices.contains(index) else { return }
		let sessionIds = groups[index].allSessionIds()
		groups.remove(at: index)
		for sid in sessionIds {
			sessionStore.removeValue(forKey: sid)
		}
		if groups.isEmpty {
			activeGroupIndex = nil
			focusedSessionId = nil
			stopPolling()
		} else {
			let newIdx = min(index, groups.count - 1)
			activeGroupIndex = newIdx
			focusedSessionId = groups[newIdx].firstSessionId()
		}
		pruneOrphanedRatios()
	}

	private func pruneOrphanedRatios() {
		let liveSplitIds = Set(groups.flatMap { $0.allSplitNodeIds() })
		splitRatios = splitRatios.filter { liveSplitIds.contains($0.key) }
	}

	func killAllSessions() {
		stopPolling()
		groups.removeAll()
		sessionStore.removeAll()
		splitRatios.removeAll()
		activeGroupIndex = nil
		focusedSessionId = nil
	}

	func splitSession(sessionId: UUID, direction: SplitDirection, workingDirectory: String? = nil) {
		guard let groupIdx = groups.firstIndex(where: { $0.containsSession(sessionId) }) else { return }

		let newSession = TerminalSession(workingDirectory: workingDirectory)
		sessionStore[newSession.id] = newSession

		groups[groupIdx] = groups[groupIdx].splitLeaf(
			sessionId: sessionId,
			newSessionId: newSession.id,
			direction: direction
		)
		focusedSessionId = newSession.id
	}

	func groupTitle(_ group: TerminalSplitNode) -> String {
		let targetId = focusedSessionId ?? group.firstSessionId()
		if group.containsSession(targetId), let session = sessionStore[targetId] {
			return session.title
		}
		return sessionStore[group.firstSessionId()]?.title ?? "zsh"
	}

	func groupHasBell(_ group: TerminalSplitNode) -> Bool {
		group.allSessionIds().contains { sid in
			sessionStore[sid]?.hasBell == true
		}
	}

	func startPolling() {
		pollingTask?.cancel()
		pollingTask = Task { @MainActor [weak self] in
			let processNotificationKey = "terminal.processNotification"
			while !Task.isCancelled {
				try? await Task.sleep(for: .seconds(2))
				guard !Task.isCancelled, let self else { break }

				// Collect session info for parallel foreground process lookups
				let sessionInfos: [(id: UUID, pid: pid_t, shell: String, skipTitle: Bool)] = self.sessions.compactMap { session in
					guard session.shellPid > 0 else { return nil }
					let skipTitle = session.titleSetByEscape.map { Date().timeIntervalSince($0) < 3 } ?? false
					return (session.id, session.shellPid, session.shellName, skipTitle)
				}

				// Poll all sessions concurrently off main thread
				let results: [(UUID, String)] = await withTaskGroup(of: (UUID, String).self) { group in
					for info in sessionInfos {
						group.addTask {
							let name = Self.foregroundProcessName(shellPid: info.pid, shellName: info.shell)
							return (info.id, name)
						}
					}
					var collected: [(UUID, String)] = []
					for await result in group {
						collected.append(result)
					}
					return collected
				}

				// Apply results on main thread
				let skipTitleMap = Dictionary(uniqueKeysWithValues: sessionInfos.map { ($0.id, $0.skipTitle) })
				for (sessionId, name) in results {
					guard let session = self.sessionStore[sessionId] else { continue }
					let isIdle = (name == session.shellName)
					if isIdle {
						if let startTime = session.commandStartTime {
							let duration = Date().timeIntervalSince(startTime)
							let enabled = UserDefaults.standard.object(forKey: processNotificationKey) as? Bool ?? true
							session.hasBell = true
							if duration > 5, enabled, !NSApp.isActive {
								let commandName = session.lastCommandName ?? "command"
								Self.sendTerminalNotification(
									title: "\(commandName) completed",
									body: "Finished in \(Self.formatDuration(duration))",
									sessionId: session.id
								)
							}
							session.commandStartTime = nil
							session.lastCommandName = nil
						}
					} else {
						if session.commandStartTime == nil {
							session.commandStartTime = Date()
							session.lastCommandName = name
						}
					}
					let skipTitle = skipTitleMap[sessionId] ?? false
					if !skipTitle, session.title != name {
						session.title = name
					}
				}
			}
		}
	}

	nonisolated static func sendTerminalNotification(title: String, body: String, sessionId: UUID? = nil) {
		let content = UNMutableNotificationContent()
		content.title = title
		content.body = body
		content.sound = .default
		if let sessionId {
			content.userInfo = ["sessionId": sessionId.uuidString]
		}
		let request = UNNotificationRequest(
			identifier: UUID().uuidString,
			content: content,
			trigger: nil
		)
		UNUserNotificationCenter.current().add(request)
	}

	nonisolated static func formatDuration(_ seconds: TimeInterval) -> String {
		let total = Int(seconds)
		if total < 60 { return "\(total)s" }
		let min = total / 60
		let sec = total % 60
		if sec == 0 { return "\(min)m" }
		return "\(min)m \(sec)s"
	}

	func stopPolling() {
		pollingTask?.cancel()
		pollingTask = nil
	}

	nonisolated static func foregroundProcessName(shellPid: pid_t, shellName: String) -> String {
		let pgrepProc = Process()
		pgrepProc.executableURL = URL(fileURLWithPath: "/usr/bin/pgrep")
		pgrepProc.arguments = ["-P", String(shellPid)]
		let pgrepPipe = Pipe()
		pgrepProc.standardOutput = pgrepPipe
		pgrepProc.standardError = Pipe()
		do {
			try pgrepProc.run()
			pgrepProc.waitUntilExit()
		} catch {
			return shellName
		}
		guard pgrepProc.terminationStatus == 0 else { return shellName }
		let pgrepData = pgrepPipe.fileHandleForReading.readDataToEndOfFile()
		let pgrepOutput = String(data: pgrepData, encoding: .utf8) ?? ""
		guard let lastPid = pgrepOutput.trimmingCharacters(in: .whitespacesAndNewlines).components(separatedBy: .newlines).last(where: { !$0.isEmpty }) else {
			return shellName
		}
		let psProc = Process()
		psProc.executableURL = URL(fileURLWithPath: "/bin/ps")
		psProc.arguments = ["-o", "comm=", "-p", lastPid.trimmingCharacters(in: .whitespaces)]
		let psPipe = Pipe()
		psProc.standardOutput = psPipe
		psProc.standardError = Pipe()
		do {
			try psProc.run()
			psProc.waitUntilExit()
		} catch {
			return shellName
		}
		let psData = psPipe.fileHandleForReading.readDataToEndOfFile()
		let name = (String(data: psData, encoding: .utf8) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
		if name.isEmpty { return shellName }
		return (name as NSString).lastPathComponent
	}
}
