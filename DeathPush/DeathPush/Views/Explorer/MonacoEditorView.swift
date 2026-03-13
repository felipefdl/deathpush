import SwiftUI
import WebKit

struct MonacoEditorView: NSViewRepresentable {
  let fileContent: FileContent?
  var themeJSON: String?
  var goToLine: Int?

  @AppStorage("editor.fontFamily") private var fontFamily = "SF Mono"
  @AppStorage("editor.fontSize") private var fontSize = 13.0
  @AppStorage("editor.lineHeight") private var lineHeight = 20.0
  @AppStorage("editor.tabSize") private var tabSize = 2
  @AppStorage("editor.wordWrap") private var wordWrap = false
  @AppStorage("editor.renderWhitespace") private var renderWhitespace = false

  func makeNSView(context: Context) -> WKWebView {
    let config = WKWebViewConfiguration()
    config.preferences.setValue(true, forKey: "developerExtrasEnabled")

    let handler = EditorBridgeMessageHandler()
    config.userContentController.add(handler, name: "editorBridge")

    let webView = WKWebView(frame: .zero, configuration: config)
    webView.setValue(false, forKey: "drawsBackground")
    context.coordinator.webView = webView
    context.coordinator.isReady = false

    if let editorURL = Bundle.main.url(forResource: "editor", withExtension: "html", subdirectory: "Monaco") {
      webView.loadFileURL(editorURL, allowingReadAccessTo: editorURL.deletingLastPathComponent())
    }

    handler.onReady = { [weak webView] in
      guard let webView else { return }
      context.coordinator.isReady = true
      context.coordinator.applyPendingUpdates(webView: webView)
    }

    return webView
  }

  func updateNSView(_ webView: WKWebView, context: Context) {
    context.coordinator.pendingThemeJSON = themeJSON
    context.coordinator.pendingContent = fileContent
    context.coordinator.pendingGoToLine = goToLine
    context.coordinator.pendingEditorOptions = editorOptionsJSON()

    if context.coordinator.isReady {
      context.coordinator.applyPendingUpdates(webView: webView)
    }
  }

  private func editorOptionsJSON() -> String {
    let wrap = wordWrap ? "\"on\"" : "\"off\""
    let ws = renderWhitespace ? "\"all\"" : "\"none\""
    let family = fontFamily.replacingOccurrences(of: "\\", with: "\\\\").replacingOccurrences(of: "\"", with: "\\\"")
    return "{\"fontFamily\":\"\(family), Menlo, Monaco, monospace\",\"fontSize\":\(Int(fontSize)),\"lineHeight\":\(Int(lineHeight)),\"tabSize\":\(tabSize),\"wordWrap\":\(wrap),\"renderWhitespace\":\(ws)}"
  }

  func makeCoordinator() -> Coordinator {
    Coordinator()
  }

  class Coordinator {
    weak var webView: WKWebView?
    var isReady = false
    var pendingContent: FileContent?
    var pendingThemeJSON: String?
    var pendingGoToLine: Int?
    var pendingEditorOptions: String?
    private var lastAppliedPath: String?
    private var lastAppliedThemeJSON: String?
    private var lastAppliedGoToLine: Int?
    private var lastAppliedEditorOptions: String?

    func applyPendingUpdates(webView: WKWebView) {
      // Apply theme BEFORE content to minimize flicker
      if let theme = pendingThemeJSON, theme != lastAppliedThemeJSON {
        webView.evaluateJavaScript("setTheme(\(theme))") { _, error in
          if let error { print("[MonacoEditor] setTheme error: \(error)") }
        }
        lastAppliedThemeJSON = theme
        pendingThemeJSON = nil
      }

      if let content = pendingContent, content.path != lastAppliedPath {
        let json = encodeFileContent(content)
        webView.evaluateJavaScript("setContent(\(json))")
        lastAppliedPath = content.path
        pendingContent = nil
        lastAppliedGoToLine = nil
      }

      if let line = pendingGoToLine, line != lastAppliedGoToLine {
        webView.evaluateJavaScript("revealLine(\(line))")
        lastAppliedGoToLine = line
        pendingGoToLine = nil
      }

      if let opts = pendingEditorOptions, opts != lastAppliedEditorOptions {
        webView.evaluateJavaScript("setEditorOptions(\(opts))")
        lastAppliedEditorOptions = opts
        pendingEditorOptions = nil
      }
    }

    private func encodeFileContent(_ content: FileContent) -> String {
      let escapedContent = escapeJS(content.content)
      let escapedPath = escapeJS(content.path)
      let lang = content.language.map { "\"\(escapeJS($0))\"" } ?? "null"
      return "{\"content\":\"\(escapedContent)\",\"path\":\"\(escapedPath)\",\"language\":\(lang)}"
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

private class EditorBridgeMessageHandler: NSObject, WKScriptMessageHandler {
  var onReady: (() -> Void)?

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
    default:
      break
    }
  }
}
