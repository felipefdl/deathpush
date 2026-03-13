import SwiftUI

struct TerminalSplitView: View {
	let node: TerminalSplitNode
	let sessionStore: [UUID: TerminalSession]
	let themeService: ThemeService
	let focusedSessionId: UUID?
	var isTopLevel: Bool = true
	@Binding var splitRatios: [UUID: CGFloat]
	let onFocus: (UUID) -> Void
	let onSplit: (UUID, SplitDirection) -> Void
	let onKill: (UUID) -> Void

	private var hasSplits: Bool {
		if case .split = node { return true }
		return !isTopLevel
	}

	var body: some View {
		switch node {
		case .leaf(_, let sessionId):
			if let session = sessionStore[sessionId] {
				SwiftTermContainerView(
					session: session,
					themeService: themeService,
					isActive: true,
					isFocused: sessionId == focusedSessionId,
					onFocus: { onFocus(sessionId) },
					onSplit: { direction in onSplit(sessionId, direction) },
					onKill: { onKill(sessionId) }
				)
				.opacity(hasSplits && sessionId != focusedSessionId ? 0.5 : 1.0)
			}
		case .split(let splitId, let axis, let first, let second):
			GeometryReader { geo in
				let totalSize = axis == .horizontal ? geo.size.width : geo.size.height
				let ratio = splitRatios[splitId] ?? 0.5
				let dividerThickness: CGFloat = 6
				let available = totalSize - dividerThickness
				let firstSize = max(40, available * ratio)
				let secondSize = max(40, available - firstSize)

				if axis == .horizontal {
					HStack(spacing: 0) {
						childView(node: first)
							.frame(width: firstSize)
						SplitDividerHandle(axis: axis, splitId: splitId, totalSize: totalSize, splitRatios: $splitRatios)
						childView(node: second)
							.frame(width: secondSize)
					}
					.coordinateSpace(name: "split-\(splitId.uuidString)")
				} else {
					VStack(spacing: 0) {
						childView(node: first)
							.frame(height: firstSize)
						SplitDividerHandle(axis: axis, splitId: splitId, totalSize: totalSize, splitRatios: $splitRatios)
						childView(node: second)
							.frame(height: secondSize)
					}
					.coordinateSpace(name: "split-\(splitId.uuidString)")
				}
			}
		}
	}

	private func childView(node: TerminalSplitNode) -> some View {
		TerminalSplitView(
			node: node,
			sessionStore: sessionStore,
			themeService: themeService,
			focusedSessionId: focusedSessionId,
			isTopLevel: false,
			splitRatios: $splitRatios,
			onFocus: onFocus,
			onSplit: onSplit,
			onKill: onKill
		)
	}
}

// MARK: - Divider with NSView-based cursor rect

private struct SplitDividerHandle: View {
	let axis: SplitAxis
	let splitId: UUID
	let totalSize: CGFloat
	@Binding var splitRatios: [UUID: CGFloat]

	var body: some View {
		CursorRectView(cursor: axis == .horizontal ? .resizeLeftRight : .resizeUpDown)
			.frame(
				width: axis == .horizontal ? 6 : nil,
				height: axis == .vertical ? 6 : nil
			)
			.contentShape(Rectangle())
			.gesture(
				DragGesture(minimumDistance: 1, coordinateSpace: .named("split-\(splitId.uuidString)"))
					.onChanged { value in
						guard totalSize > 0 else { return }
						let position = axis == .horizontal ? value.location.x : value.location.y
						let newRatio = min(max(position / totalSize, 0.1), 0.9)
						let currentRatio = splitRatios[splitId] ?? 0.5
						guard abs(newRatio - currentRatio) > 0.005 else { return }
						splitRatios[splitId] = newRatio
					}
			)
	}
}

private struct CursorRectView: NSViewRepresentable {
	let cursor: NSCursor

	func makeNSView(context: Context) -> CursorTrackingNSView {
		let view = CursorTrackingNSView()
		view.activeCursor = cursor
		return view
	}

	func updateNSView(_ nsView: CursorTrackingNSView, context: Context) {
		if nsView.activeCursor != cursor {
			nsView.activeCursor = cursor
		}
	}
}

private class CursorTrackingNSView: NSView {
	var activeCursor: NSCursor = .arrow {
		didSet {
			window?.invalidateCursorRects(for: self)
		}
	}

	private var cursorTrackingArea: NSTrackingArea?

	override func updateTrackingAreas() {
		super.updateTrackingAreas()
		if let existing = cursorTrackingArea {
			removeTrackingArea(existing)
		}
		let area = NSTrackingArea(
			rect: bounds,
			options: [.cursorUpdate, .mouseEnteredAndExited, .activeInActiveApp, .inVisibleRect],
			owner: self,
			userInfo: nil
		)
		addTrackingArea(area)
		cursorTrackingArea = area
	}

	override func resetCursorRects() {
		addCursorRect(bounds, cursor: activeCursor)
	}

	override func cursorUpdate(with event: NSEvent) {
		activeCursor.set()
	}

	override func mouseEntered(with event: NSEvent) {
		activeCursor.set()
	}

	override func mouseExited(with event: NSEvent) {
		NSCursor.arrow.set()
	}
}
