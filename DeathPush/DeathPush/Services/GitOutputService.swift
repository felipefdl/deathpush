import SwiftUI

struct GitCommandEntry: Identifiable {
	let id = UUID()
	let command: String
	let durationMs: UInt64
	let timestamp: String
	let receivedAt = Date()
}

@Observable
final class GitOutputService {
	var entries: [GitCommandEntry] = []
	private let maxEntries = 200

	func append(command: String, durationMs: UInt64, timestamp: String) {
		entries.append(GitCommandEntry(command: command, durationMs: durationMs, timestamp: timestamp))
		if entries.count > maxEntries {
			entries.removeFirst(entries.count - maxEntries)
		}
	}

	func clear() { entries.removeAll() }
}
