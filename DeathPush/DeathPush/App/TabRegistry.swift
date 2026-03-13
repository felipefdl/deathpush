import Foundation

final class TabRegistry: @unchecked Sendable {
	static let shared = TabRegistry()

	private var entries: [String: WeakTabRef] = [:]
	private let lock = NSLock()

	private init() {}

	func register(_ tab: TabState) {
		guard let sessionId = tab.repoService?.sessionId else { return }
		lock.lock()
		defer { lock.unlock() }
		entries[sessionId] = WeakTabRef(tab)
	}

	func unregister(sessionId: String) {
		lock.lock()
		defer { lock.unlock() }
		entries.removeValue(forKey: sessionId)
	}

	func tabForSession(_ sessionId: String) -> TabState? {
		lock.lock()
		defer { lock.unlock() }
		guard let ref = entries[sessionId] else { return nil }
		if let tab = ref.tab {
			return tab
		}
		entries.removeValue(forKey: sessionId)
		return nil
	}
}

private final class WeakTabRef {
	weak var tab: TabState?

	init(_ tab: TabState) {
		self.tab = tab
	}
}
