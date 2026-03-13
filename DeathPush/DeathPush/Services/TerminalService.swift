import Foundation
import SwiftUI

@MainActor @Observable
final class TerminalSession: Identifiable {
  let id = UUID()
  var title = "zsh"
  let workingDirectory: String?
  var shellPid: pid_t = 0
  var shellName: String = "zsh"
  var titleSetByEscape: Date?

  init(workingDirectory: String? = nil) {
    self.workingDirectory = workingDirectory
  }
}

@MainActor @Observable
final class TerminalService {
  var sessions: [TerminalSession] = []
  var activeSessionId: UUID?

  private var pollingTask: Task<Void, Never>?

  func spawnSession(workingDirectory: String? = nil) {
    let session = TerminalSession(workingDirectory: workingDirectory)
    sessions.append(session)
    activeSessionId = session.id
  }

  func killSession(_ id: UUID) {
    sessions.removeAll { $0.id == id }
    if activeSessionId == id {
      activeSessionId = sessions.last?.id
    }
    if sessions.isEmpty {
      stopPolling()
    }
  }

  func killAllSessions() {
    stopPolling()
    sessions.removeAll()
    activeSessionId = nil
  }

  func startPolling() {
    pollingTask?.cancel()
    pollingTask = Task { @MainActor [weak self] in
      while !Task.isCancelled {
        try? await Task.sleep(for: .seconds(2))
        guard !Task.isCancelled, let self else { break }
        guard let session = self.sessions.first(where: { $0.id == self.activeSessionId }),
              session.shellPid > 0 else { continue }
        // Only update if escape sequence hasn't set title in last 3 seconds
        if let escapeDate = session.titleSetByEscape, Date().timeIntervalSince(escapeDate) < 3 {
          continue
        }
        let pid = session.shellPid
        let shell = session.shellName
        let name = await Task.detached { Self.foregroundProcessName(shellPid: pid, shellName: shell) }.value
        if session.title != name {
          session.title = name
        }
      }
    }
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
