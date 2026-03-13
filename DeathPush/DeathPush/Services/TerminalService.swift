import SwiftUI

@Observable
final class TerminalSession: Identifiable {
  let id = UUID()
  var title = "zsh"
  let workingDirectory: String?

  init(workingDirectory: String? = nil) {
    self.workingDirectory = workingDirectory
  }
}

@Observable
final class TerminalService {
  var sessions: [TerminalSession] = []
  var activeSessionId: UUID?

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
  }

  func killAllSessions() {
    sessions.removeAll()
    activeSessionId = nil
  }
}
