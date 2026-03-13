import Combine
import SwiftUI

struct GitOutputPopoverView: View {
	@Environment(AppState.self) private var appState

	var body: some View {
		VStack(spacing: 0) {
			HStack {
				Text("Git Output")
					.font(.headline)
				Spacer()
				Button("Clear") {
					appState.gitOutputService.clear()
				}
				.buttonStyle(.plain)
				.font(.caption)
				.foregroundStyle(.secondary)
			}
			.padding(10)

			Divider()

			List {
				if appState.gitOutputService.entries.isEmpty {
					Text("No git commands recorded")
						.font(.callout)
						.foregroundStyle(.tertiary)
						.frame(maxWidth: .infinity)
						.padding(.top, 24)
						.listRowSeparator(.hidden)
				} else {
					ForEach(appState.gitOutputService.entries.reversed()) { entry in
						GitCommandRow(entry: entry)
							.listRowInsets(EdgeInsets(top: 4, leading: 10, bottom: 4, trailing: 10))
					}
				}
			}
			.listStyle(.plain)
			.scrollEdgeEffectStyle(.soft, for: .top)
		}
	}
}

// MARK: - Command Row

private struct GitCommandRow: View {
	let entry: GitCommandEntry
	@State private var now = Date()
	private let timer = Timer.publish(every: 10, on: .main, in: .common).autoconnect()

	var body: some View {
		VStack(alignment: .leading, spacing: 3) {
			Text(entry.command)
				.font(.system(.caption, design: .monospaced))
				.lineLimit(2)
				.foregroundStyle(.primary)

			HStack(spacing: 6) {
				Text(formatDuration(entry.durationMs))
					.foregroundStyle(.secondary)
				Text(relativeTime(from: entry.receivedAt, now: now))
					.foregroundStyle(.tertiary)
			}
			.font(.caption2)
		}
		.onReceive(timer) { now = $0 }
	}
}

// MARK: - Helpers

private func formatDuration(_ ms: UInt64) -> String {
	if ms < 1000 {
		return "\(ms)ms"
	} else {
		let seconds = Double(ms) / 1000.0
		return String(format: "%.1fs", seconds)
	}
}

private func relativeTime(from date: Date, now: Date) -> String {
	let seconds = Int(now.timeIntervalSince(date))
	if seconds < 5 { return "just now" }
	if seconds < 60 { return "\(seconds)s ago" }
	let minutes = seconds / 60
	if minutes < 60 { return "\(minutes)m ago" }
	let hours = minutes / 60
	return "\(hours)h ago"
}
