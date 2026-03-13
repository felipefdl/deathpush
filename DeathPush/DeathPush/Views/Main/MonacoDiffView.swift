import SwiftUI
import WebKit

struct MonacoDiffView: NSViewRepresentable {
	let diff: DiffContent?
	let diffMode: DiffMode
	var contentVersion: Int = 0
	var themeJSON: String?
	var hunkMetaJSON: String?
	var onLineSelection: ((Int, Int, Int, Int) -> Void)?
	var onLineSelectionCleared: (() -> Void)?

	func makeNSView(context: Context) -> WKWebView {
		let config = WKWebViewConfiguration()
		config.preferences.setValue(true, forKey: "developerExtrasEnabled")

		let handler = DiffBridgeMessageHandler()
		config.userContentController.add(handler, name: "diffBridge")

		let webView = WKWebView(frame: .zero, configuration: config)
		webView.setValue(false, forKey: "drawsBackground")
		context.coordinator.webView = webView
		context.coordinator.isReady = false

		if let monacoURL = Bundle.main.url(forResource: "index", withExtension: "html", subdirectory: "Monaco") {
			webView.loadFileURL(monacoURL, allowingReadAccessTo: monacoURL.deletingLastPathComponent())
		}

		// Listen for "ready" message from JS
		handler.onReady = { [weak webView] in
			guard let webView else { return }
			context.coordinator.isReady = true
			context.coordinator.applyPendingUpdates(webView: webView)
		}

		handler.onLineSelection = { [weak coordinator = context.coordinator] hunkIndex, start, end, editorEnd in
			coordinator?.onLineSelection?(hunkIndex, start, end, editorEnd)
		}
		handler.onLineSelectionCleared = { [weak coordinator = context.coordinator] in
			coordinator?.onLineSelectionCleared?()
		}

		return webView
	}

	func updateNSView(_ webView: WKWebView, context: Context) {
		context.coordinator.pendingThemeJSON = themeJSON
		context.coordinator.pendingDiff = diff
		context.coordinator.pendingDiffMode = diffMode
		context.coordinator.pendingContentVersion = contentVersion
		context.coordinator.pendingHunkMetaJSON = hunkMetaJSON
		context.coordinator.onLineSelection = onLineSelection
		context.coordinator.onLineSelectionCleared = onLineSelectionCleared

		if context.coordinator.isReady {
			context.coordinator.applyPendingUpdates(webView: webView)
		}
	}

	func makeCoordinator() -> Coordinator {
		Coordinator()
	}

	class Coordinator {
		weak var webView: WKWebView?
		var isReady = false
		var pendingDiff: DiffContent?
		var pendingDiffMode: DiffMode?
		var pendingThemeJSON: String?
		var pendingHunkMetaJSON: String?
		var pendingContentVersion: Int = 0
		var onLineSelection: ((Int, Int, Int, Int) -> Void)?
		var onLineSelectionCleared: (() -> Void)?
		private var lastAppliedDiffPath: String?
		private var lastAppliedDiffMode: DiffMode?
		private var lastAppliedThemeJSON: String?
		private var lastAppliedContentVersion: Int = 0
		private var lastAppliedHunkMetaJSON: String?

		func applyPendingUpdates(webView: WKWebView) {
			// Apply theme BEFORE content to minimize flicker
			if let theme = pendingThemeJSON, theme != lastAppliedThemeJSON {
				webView.evaluateJavaScript("setTheme(\(theme))") { _, error in
					if let error { print("[MonacoDiff] setTheme error: \(error)") }
				}
				lastAppliedThemeJSON = theme
				pendingThemeJSON = nil
			}

			if let diff = pendingDiff,
			   diff.path != lastAppliedDiffPath || pendingContentVersion != lastAppliedContentVersion {
				let json = encodeDiffContent(diff)
				webView.evaluateJavaScript("setDiffContent(\(json))")
				lastAppliedDiffPath = diff.path
				lastAppliedContentVersion = pendingContentVersion
				pendingDiff = nil

				// Send hunk metadata after content is set
				if let hunkJSON = pendingHunkMetaJSON {
					webView.evaluateJavaScript("enableLineStaging(\(hunkJSON))")
					lastAppliedHunkMetaJSON = hunkJSON
					pendingHunkMetaJSON = nil
				}
			} else if let hunkJSON = pendingHunkMetaJSON, hunkJSON != lastAppliedHunkMetaJSON {
				webView.evaluateJavaScript("enableLineStaging(\(hunkJSON))")
				lastAppliedHunkMetaJSON = hunkJSON
				pendingHunkMetaJSON = nil
			}

			if let mode = pendingDiffMode, mode != lastAppliedDiffMode {
				webView.evaluateJavaScript("setDiffMode('\(mode.rawValue)')")
				lastAppliedDiffMode = mode
				pendingDiffMode = nil
			}
		}

		private func encodeDiffContent(_ diff: DiffContent) -> String {
			let original = escapeJS(diff.original)
			let modified = escapeJS(diff.modified)
			let path = escapeJS(diff.path)
			let lang = diff.originalLanguage.map { "\"\(escapeJS($0))\"" } ?? "null"
			return "{\"original\":\"\(original)\",\"modified\":\"\(modified)\",\"path\":\"\(path)\",\"originalLanguage\":\(lang)}"
		}

		private func escapeJS(_ str: String) -> String {
			str
				.replacingOccurrences(of: "\\", with: "\\\\")
				.replacingOccurrences(of: "\"", with: "\\\"")
				.replacingOccurrences(of: "\n", with: "\\n")
				.replacingOccurrences(of: "\r", with: "\\r")
				.replacingOccurrences(of: "\t", with: "\\t")
		}
	}
}

enum DiffMode: String {
  case inline
  case sideBySide
}

private class DiffBridgeMessageHandler: NSObject, WKScriptMessageHandler {
	var onReady: (() -> Void)?
	var onLineSelection: ((Int, Int, Int, Int) -> Void)?
	var onLineSelectionCleared: (() -> Void)?

	func userContentController(
		_ userContentController: WKUserContentController,
		didReceive message: WKScriptMessage
	) {
		guard let body = message.body as? [String: Any],
		      let type = body["type"] as? String else { return }

		switch type {
		case "ready":
			Task { @MainActor in
				onReady?()
			}
		case "lineSelection":
			if let hunkIndex = body["hunkIndex"] as? Int,
			   let startLine = body["startLine"] as? Int,
			   let endLine = body["endLine"] as? Int,
			   let editorEndLine = body["editorEndLine"] as? Int {
				Task { @MainActor in
					onLineSelection?(hunkIndex, startLine, endLine, editorEndLine)
				}
			}
		case "lineSelectionCleared":
			Task { @MainActor in
				onLineSelectionCleared?()
			}
		default:
			break
		}
	}
}
