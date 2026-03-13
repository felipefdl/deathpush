import SwiftUI

struct ErrorToastView: View {
	let message: String
	let onDismiss: () -> Void

	@State private var isVisible = false

	var body: some View {
		HStack(spacing: 8) {
			Image(systemName: "exclamationmark.triangle.fill")
				.foregroundStyle(.red)
			Text(message)
				.font(.callout)
				.lineLimit(3)
			Spacer()
			Button(action: {
				withAnimation(.easeInOut(duration: 0.25)) {
					isVisible = false
				}
				DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
					onDismiss()
				}
			}) {
				Image(systemName: "xmark")
					.font(.caption)
			}
			.buttonStyle(.plain)
			.foregroundStyle(.secondary)
		}
		.padding(12)
		.glassEffect(.regular.interactive())
		.padding(.horizontal, 16)
		.opacity(isVisible ? 1 : 0)
		.offset(y: isVisible ? 0 : -20)
		.onAppear {
			withAnimation(.easeInOut(duration: 0.25)) {
				isVisible = true
			}
		}
		.task {
			try? await Task.sleep(for: .seconds(6))
			guard isVisible else { return }
			withAnimation(.easeInOut(duration: 0.25)) {
				isVisible = false
			}
			DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
				onDismiss()
			}
		}
	}
}
