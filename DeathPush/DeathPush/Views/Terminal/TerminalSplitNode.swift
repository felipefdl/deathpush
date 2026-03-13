import Foundation

enum SplitAxis {
	case horizontal, vertical
}

enum SplitDirection {
	case right, left, down, up

	var axis: SplitAxis {
		switch self {
		case .right, .left: return .horizontal
		case .down, .up: return .vertical
		}
	}

	var newPaneIsFirst: Bool {
		switch self {
		case .left, .up: return true
		case .right, .down: return false
		}
	}
}

indirect enum TerminalSplitNode: Identifiable {
	case leaf(id: UUID, sessionId: UUID)
	case split(id: UUID, axis: SplitAxis, first: TerminalSplitNode, second: TerminalSplitNode)

	var id: UUID {
		switch self {
		case .leaf(let id, _): return id
		case .split(let id, _, _, _): return id
		}
	}

	func splitLeaf(sessionId: UUID, newSessionId: UUID, direction: SplitDirection) -> TerminalSplitNode {
		switch self {
		case .leaf(_, let sid) where sid == sessionId:
			let existing = TerminalSplitNode.leaf(id: UUID(), sessionId: sid)
			let newLeaf = TerminalSplitNode.leaf(id: UUID(), sessionId: newSessionId)
			if direction.newPaneIsFirst {
				return .split(id: UUID(), axis: direction.axis, first: newLeaf, second: existing)
			} else {
				return .split(id: UUID(), axis: direction.axis, first: existing, second: newLeaf)
			}
		case .leaf:
			return self
		case .split(let id, let axis, let first, let second):
			return .split(
				id: id,
				axis: axis,
				first: first.splitLeaf(sessionId: sessionId, newSessionId: newSessionId, direction: direction),
				second: second.splitLeaf(sessionId: sessionId, newSessionId: newSessionId, direction: direction)
			)
		}
	}

	func removeLeaf(sessionId: UUID) -> TerminalSplitNode? {
		switch self {
		case .leaf(_, let sid) where sid == sessionId:
			return nil
		case .leaf:
			return self
		case .split(let id, let axis, let first, let second):
			let newFirst = first.removeLeaf(sessionId: sessionId)
			let newSecond = second.removeLeaf(sessionId: sessionId)
			switch (newFirst, newSecond) {
			case (nil, nil): return nil
			case (nil, let remaining?): return remaining
			case (let remaining?, nil): return remaining
			case (let f?, let s?): return .split(id: id, axis: axis, first: f, second: s)
			}
		}
	}

	func allSessionIds() -> [UUID] {
		switch self {
		case .leaf(_, let sessionId):
			return [sessionId]
		case .split(_, _, let first, let second):
			return first.allSessionIds() + second.allSessionIds()
		}
	}

	func firstSessionId() -> UUID {
		switch self {
		case .leaf(_, let sessionId):
			return sessionId
		case .split(_, _, let first, _):
			return first.firstSessionId()
		}
	}

	func siblingSessionId(of targetId: UUID) -> UUID? {
		switch self {
		case .leaf:
			return nil
		case .split(_, _, let first, let second):
			let firstIds = first.allSessionIds()
			let secondIds = second.allSessionIds()
			if firstIds.contains(targetId) {
				return second.firstSessionId()
			}
			if secondIds.contains(targetId) {
				return first.firstSessionId()
			}
			return first.siblingSessionId(of: targetId) ?? second.siblingSessionId(of: targetId)
		}
	}

	func containsSession(_ sessionId: UUID) -> Bool {
		allSessionIds().contains(sessionId)
	}

	func allSplitNodeIds() -> [UUID] {
		switch self {
		case .leaf:
			return []
		case .split(let id, _, let first, let second):
			return [id] + first.allSplitNodeIds() + second.allSplitNodeIds()
		}
	}
}
