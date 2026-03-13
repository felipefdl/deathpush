import AppKit
import Foundation
import SwiftUI

@Observable
final class ThemeService {
  var currentThemeName: String {
    didSet {
      UserDefaults.standard.set(currentThemeName, forKey: "theme.current")
    }
  }

  var themeData: [String: Any]?

  var colorScheme: ColorScheme {
    guard let type = themeData?["type"] as? String else {
      let entry = Self.availableThemes.first { $0.name == currentThemeName }
      return entry?.isLight == true ? .light : .dark
    }
    return type == "light" || type == "hcLight" ? .light : .dark
  }

  private(set) var themeDataJSON: String?

  /// Returns 16 ANSI colors as (r, g, b) tuples in UInt8 range, or nil if theme lacks terminal colors.
  var terminalANSIColorComponents: [(r: UInt8, g: UInt8, b: UInt8)]? {
    let keys = [
      "terminal.ansiBlack", "terminal.ansiRed", "terminal.ansiGreen", "terminal.ansiYellow",
      "terminal.ansiBlue", "terminal.ansiMagenta", "terminal.ansiCyan", "terminal.ansiWhite",
      "terminal.ansiBrightBlack", "terminal.ansiBrightRed", "terminal.ansiBrightGreen", "terminal.ansiBrightYellow",
      "terminal.ansiBrightBlue", "terminal.ansiBrightMagenta", "terminal.ansiBrightCyan", "terminal.ansiBrightWhite",
    ]
    guard let colors = themeData?["colors"] as? [String: String] else { return nil }
    let result = keys.compactMap { key -> (r: UInt8, g: UInt8, b: UInt8)? in
      guard let hex = colors[key], let ns = NSColor(hex: hex) else { return nil }
      return (UInt8(ns.redComponent * 255), UInt8(ns.greenComponent * 255), UInt8(ns.blueComponent * 255))
    }
    return result.count == 16 ? result : nil
  }

  func color(forKey key: String) -> NSColor? {
    guard let colors = themeData?["colors"] as? [String: String],
          let hex = colors[key] else { return nil }
    return NSColor(hex: hex)
  }

  enum ThemeKind: String, CaseIterable {
    case dark, light, highContrast

    var label: String {
      switch self {
      case .dark: "dark themes"
      case .light: "light themes"
      case .highContrast: "high contrast"
      }
    }
  }

  struct ThemeEntry: Identifiable {
    var id: String { name }
    let name: String
    let displayName: String
    let isLight: Bool
    let kind: ThemeKind
  }

  static let availableThemes: [ThemeEntry] = [
    ThemeEntry(name: "deathayu-dark", displayName: "Ayu Dark (Death)", isLight: false, kind: .dark),
    ThemeEntry(name: "deathayu-light", displayName: "Ayu Light (Death)", isLight: true, kind: .light),
    ThemeEntry(name: "dark_modern", displayName: "Dark Modern", isLight: false, kind: .dark),
    ThemeEntry(name: "dark_plus", displayName: "Dark+", isLight: false, kind: .dark),
    ThemeEntry(name: "dark_vs", displayName: "Dark (Visual Studio)", isLight: false, kind: .dark),
    ThemeEntry(name: "light_modern", displayName: "Light Modern", isLight: true, kind: .light),
    ThemeEntry(name: "light_plus", displayName: "Light+", isLight: true, kind: .light),
    ThemeEntry(name: "light_vs", displayName: "Light (Visual Studio)", isLight: true, kind: .light),
    ThemeEntry(name: "abyss-color-theme", displayName: "Abyss", isLight: false, kind: .dark),
    ThemeEntry(name: "ayu-dark", displayName: "Ayu Dark", isLight: false, kind: .dark),
    ThemeEntry(name: "ayu-light", displayName: "Ayu Light", isLight: true, kind: .light),
    ThemeEntry(name: "ayu-mirage", displayName: "Ayu Mirage", isLight: false, kind: .dark),
    ThemeEntry(name: "catppuccin-mocha", displayName: "Catppuccin Mocha", isLight: false, kind: .dark),
    ThemeEntry(name: "catppuccin-frappe", displayName: "Catppuccin Frappe", isLight: false, kind: .dark),
    ThemeEntry(name: "catppuccin-macchiato", displayName: "Catppuccin Macchiato", isLight: false, kind: .dark),
    ThemeEntry(name: "catppuccin-latte", displayName: "Catppuccin Latte", isLight: true, kind: .light),
    ThemeEntry(name: "dracula", displayName: "Dracula", isLight: false, kind: .dark),
    ThemeEntry(name: "dracula-soft", displayName: "Dracula Soft", isLight: false, kind: .dark),
    ThemeEntry(name: "kimbie-dark-color-theme", displayName: "Kimbie Dark", isLight: false, kind: .dark),
    ThemeEntry(name: "monokai-color-theme", displayName: "Monokai", isLight: false, kind: .dark),
    ThemeEntry(name: "dimmed-monokai-color-theme", displayName: "Monokai Dimmed", isLight: false, kind: .dark),
    ThemeEntry(name: "nord", displayName: "Nord", isLight: false, kind: .dark),
    ThemeEntry(name: "one-dark-pro", displayName: "One Dark Pro", isLight: false, kind: .dark),
    ThemeEntry(name: "Red-color-theme", displayName: "Red", isLight: false, kind: .dark),
    ThemeEntry(name: "quietlight-color-theme", displayName: "Quiet Light", isLight: true, kind: .light),
    ThemeEntry(name: "solarized-dark-color-theme", displayName: "Solarized Dark", isLight: false, kind: .dark),
    ThemeEntry(name: "solarized-light-color-theme", displayName: "Solarized Light", isLight: true, kind: .light),
    ThemeEntry(name: "tomorrow-night-blue-color-theme", displayName: "Tomorrow Night Blue", isLight: false, kind: .dark),
    ThemeEntry(name: "hc_black", displayName: "High Contrast Dark", isLight: false, kind: .highContrast),
    ThemeEntry(name: "hc_light", displayName: "High Contrast Light", isLight: true, kind: .highContrast),
  ]

  init() {
    self.currentThemeName = UserDefaults.standard.string(forKey: "theme.current") ?? "deathayu-dark"
    loadTheme(currentThemeName)
  }

  func applyTheme(_ name: String) {
    currentThemeName = name
    loadTheme(name)
  }

  // MARK: - Theme Loading & Include Resolution

  private static let includeMap: [String: String] = [
    "./dark_vs.json": "dark_vs",
    "./dark_plus.json": "dark_plus",
    "./light_vs.json": "light_vs",
    "./light_plus.json": "light_plus",
  ]

  private func loadTheme(_ name: String) {
    guard let json = Self.loadThemeJSON(name) else {
      themeData = nil
      themeDataJSON = nil
      return
    }

    let entry = Self.availableThemes.first { $0.name == name }
    let themeType = Self.themeType(for: entry)

    // Resolve include chain (merge base colors + tokenColors)
    let (resolvedColors, resolvedTokenColors) = Self.resolveIncludeChain(json)

    // Apply default color fallbacks, then theme colors on top
    let defaults = (themeType == "light" || themeType == "hcLight") ? Self.defaultLightColors : Self.defaultDarkColors
    var mergedColors = defaults
    mergedColors.merge(resolvedColors) { _, new in new }

    // Force terminal background to match editor background (same as Tauri)
    mergedColors.removeValue(forKey: "terminal.background")

    let resolved: [String: Any] = [
      "name": name.lowercased().replacingOccurrences(of: "_", with: "-"),
      "type": themeType,
      "colors": mergedColors,
      "tokenColors": resolvedTokenColors,
    ]

    themeData = resolved
    do {
      let serialized = try JSONSerialization.data(withJSONObject: resolved, options: [.sortedKeys])
      themeDataJSON = String(data: serialized, encoding: .utf8)
      print("[ThemeService] loaded '\(name)' type=\(themeType) colors=\(mergedColors.count) tokenColors=\(resolvedTokenColors.count) json=\(themeDataJSON?.count ?? 0) bytes")
    } catch {
      print("[ThemeService] JSON serialization failed for '\(name)': \(error)")
      themeDataJSON = nil
    }
  }

  private static func loadThemeJSON(_ name: String) -> [String: Any]? {
    guard let url = Bundle.main.url(forResource: name, withExtension: "json", subdirectory: "Monaco/themes"),
          let data = try? Data(contentsOf: url),
          let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
      return nil
    }
    return json
  }

  private static func themeType(for entry: ThemeEntry?) -> String {
    guard let entry else { return "dark" }
    switch entry.kind {
    case .light: return "light"
    case .highContrast: return entry.isLight ? "hcLight" : "hcDark"
    case .dark: return "dark"
    }
  }

  private static func resolveIncludeChain(_ json: [String: Any]) -> (colors: [String: String], tokenColors: [[String: Any]]) {
    var baseColors: [String: String] = [:]
    var baseTokenColors: [[String: Any]] = []

    if let include = json["include"] as? String,
       let resourceName = includeMap[include],
       let baseJson = loadThemeJSON(resourceName) {
      let resolved = resolveIncludeChain(baseJson)
      baseColors = resolved.colors
      baseTokenColors = resolved.tokenColors
    }

    if let overlayColors = json["colors"] as? [String: String] {
      baseColors.merge(overlayColors) { _, new in new }
    }

    let overlayTokenColors = json["tokenColors"] as? [[String: Any]] ?? []
    let mergedTokenColors = mergeTokenColors(base: baseTokenColors, overlay: overlayTokenColors)

    return (baseColors, mergedTokenColors)
  }

  private static func mergeTokenColors(base: [[String: Any]], overlay: [[String: Any]]) -> [[String: Any]] {
    var result = base
    for tc in overlay {
      let scopes = normalizeScope(tc["scope"])
      if let existingIdx = result.firstIndex(where: { normalizeScope($0["scope"]) == scopes }) {
        var existingSettings = result[existingIdx]["settings"] as? [String: Any] ?? [:]
        let newSettings = tc["settings"] as? [String: Any] ?? [:]
        existingSettings.merge(newSettings) { _, new in new }
        var merged = result[existingIdx]
        merged["settings"] = existingSettings
        result[existingIdx] = merged
      } else {
        result.append(tc)
      }
    }
    return result
  }

  private static func normalizeScope(_ scope: Any?) -> [String] {
    if let arr = scope as? [String] { return arr }
    if let str = scope as? String { return [str] }
    return []
  }

  // MARK: - Default Color Fallbacks (matches Tauri defaults.ts)

  private static let defaultDarkColors: [String: String] = [
    "editor.background": "#1E1E1E",
    "editor.foreground": "#D4D4D4",
    "sideBar.background": "#252526",
    "sideBar.foreground": "#CCCCCC",
    "sideBarSectionHeader.background": "#383838",
    "sideBarSectionHeader.foreground": "#CCCCCC",
    "panel.background": "#1E1E1E",
    "panel.border": "#80808059",
    "input.background": "#3C3C3C",
    "input.foreground": "#CCCCCC",
    "input.border": "#3C3C3C",
    "input.placeholderForeground": "#A6A6A6",
    "focusBorder": "#007FD4",
    "button.background": "#0E639C",
    "button.foreground": "#FFFFFF",
    "button.hoverBackground": "#1177BB",
    "button.secondaryBackground": "#3A3D41",
    "button.secondaryForeground": "#FFFFFF",
    "button.secondaryHoverBackground": "#45494E",
    "list.activeSelectionBackground": "#04395E",
    "list.activeSelectionForeground": "#FFFFFF",
    "list.hoverBackground": "#2A2D2E",
    "list.hoverForeground": "#CCCCCC",
    "list.inactiveSelectionBackground": "#37373D",
    "scrollbarSlider.background": "#79797966",
    "scrollbarSlider.hoverBackground": "#646464B3",
    "scrollbarSlider.activeBackground": "#BFBFBF66",
    "badge.background": "#4D4D4D",
    "badge.foreground": "#FFFFFF",
    "titleBar.activeBackground": "#3C3C3C",
    "titleBar.activeForeground": "#CCCCCC",
    "statusBar.background": "#007ACC",
    "statusBar.foreground": "#FFFFFF",
    "gitDecoration.modifiedResourceForeground": "#E2C08D",
    "gitDecoration.deletedResourceForeground": "#C74E39",
    "gitDecoration.untrackedResourceForeground": "#73C991",
    "gitDecoration.addedResourceForeground": "#81B88B",
    "gitDecoration.conflictingResourceForeground": "#E4676B",
    "gitDecoration.renamedResourceForeground": "#73C991",
    "gitDecoration.ignoredResourceForeground": "#8C8C8C",
    "gitDecoration.submoduleResourceForeground": "#8DB9E2",
    "gitDecoration.stageModifiedResourceForeground": "#E2C08D",
    "gitDecoration.stageDeletedResourceForeground": "#C74E39",
    "foreground": "#CCCCCC",
    "descriptionForeground": "#CCCCCCB3",
    "errorForeground": "#F48771",
    "editorLineNumber.foreground": "#858585",
    "editorLineNumber.activeForeground": "#C6C6C6",
    "diffEditor.insertedLineBackground": "#9BB95533",
    "diffEditor.removedLineBackground": "#ff000033",
    "diffEditor.insertedTextBackground": "#9ccc2c33",
    "diffEditor.removedTextBackground": "#ff000033",
    "diffEditor.hunkHeaderBackground": "#2D2D30",
    "inputValidation.errorBackground": "#5A1D1D",
    "inputValidation.errorForeground": "#F48771",
    "inputValidation.errorBorder": "#BE1100",
    "inputValidation.warningBackground": "#352A05",
    "inputValidation.warningForeground": "#E0C080",
    "inputValidation.warningBorder": "#7A5500",
    "editorWarning.foreground": "#CCA700",
    "terminal.foreground": "#CCCCCC",
    "terminalCursor.foreground": "#AEAFAD",
    "terminal.selectionBackground": "#FFFFFF4D",
    "terminal.ansiBlack": "#000000",
    "terminal.ansiRed": "#CD3131",
    "terminal.ansiGreen": "#0DBC79",
    "terminal.ansiYellow": "#E5E510",
    "terminal.ansiBlue": "#2472C8",
    "terminal.ansiMagenta": "#BC3FBC",
    "terminal.ansiCyan": "#11A8CD",
    "terminal.ansiWhite": "#E5E5E5",
    "terminal.ansiBrightBlack": "#666666",
    "terminal.ansiBrightRed": "#F14C4C",
    "terminal.ansiBrightGreen": "#23D18B",
    "terminal.ansiBrightYellow": "#F5F543",
    "terminal.ansiBrightBlue": "#3B8EEA",
    "terminal.ansiBrightMagenta": "#D670D6",
    "terminal.ansiBrightCyan": "#29B8DB",
    "terminal.ansiBrightWhite": "#E5E5E5",
  ]

  private static let defaultLightColors: [String: String] = [
    "editor.background": "#FFFFFF",
    "editor.foreground": "#000000",
    "sideBar.background": "#F3F3F3",
    "sideBar.foreground": "#616161",
    "sideBarSectionHeader.background": "#80808033",
    "sideBarSectionHeader.foreground": "#616161",
    "panel.background": "#FFFFFF",
    "panel.border": "#80808059",
    "input.background": "#FFFFFF",
    "input.foreground": "#616161",
    "input.border": "#CECECE",
    "input.placeholderForeground": "#767676",
    "focusBorder": "#0078D4",
    "button.background": "#0078D4",
    "button.foreground": "#FFFFFF",
    "button.hoverBackground": "#026EC1",
    "button.secondaryBackground": "#5F6A79",
    "button.secondaryForeground": "#FFFFFF",
    "button.secondaryHoverBackground": "#4C5561",
    "list.activeSelectionBackground": "#0078D4",
    "list.activeSelectionForeground": "#FFFFFF",
    "list.hoverBackground": "#E8E8E8",
    "list.hoverForeground": "#000000",
    "list.inactiveSelectionBackground": "#E4E6F1",
    "scrollbarSlider.background": "#64646466",
    "scrollbarSlider.hoverBackground": "#646464B3",
    "scrollbarSlider.activeBackground": "#00000099",
    "badge.background": "#C4C4C4",
    "badge.foreground": "#333333",
    "titleBar.activeBackground": "#DDDDDD",
    "titleBar.activeForeground": "#333333",
    "statusBar.background": "#007ACC",
    "statusBar.foreground": "#FFFFFF",
    "gitDecoration.modifiedResourceForeground": "#895503",
    "gitDecoration.deletedResourceForeground": "#AD0707",
    "gitDecoration.untrackedResourceForeground": "#007100",
    "gitDecoration.addedResourceForeground": "#587C0C",
    "gitDecoration.conflictingResourceForeground": "#6C6CC4",
    "gitDecoration.renamedResourceForeground": "#007100",
    "gitDecoration.ignoredResourceForeground": "#8E8E90",
    "gitDecoration.submoduleResourceForeground": "#1258A7",
    "gitDecoration.stageModifiedResourceForeground": "#895503",
    "gitDecoration.stageDeletedResourceForeground": "#AD0707",
    "foreground": "#616161",
    "descriptionForeground": "#717171",
    "errorForeground": "#A1260D",
    "editorLineNumber.foreground": "#237893",
    "editorLineNumber.activeForeground": "#0B216F",
    "diffEditor.insertedLineBackground": "#9BB95533",
    "diffEditor.removedLineBackground": "#FF000033",
    "diffEditor.insertedTextBackground": "#9ccc2c40",
    "diffEditor.removedTextBackground": "#ff000033",
    "diffEditor.hunkHeaderBackground": "#E7E7E7",
    "inputValidation.errorBackground": "#F2DEDE",
    "inputValidation.errorForeground": "#A1260D",
    "inputValidation.errorBorder": "#BE1100",
    "inputValidation.warningBackground": "#F6F5D2",
    "inputValidation.warningForeground": "#735C0F",
    "inputValidation.warningBorder": "#B89500",
    "editorWarning.foreground": "#BF8803",
    "terminal.foreground": "#000000",
    "terminalCursor.foreground": "#000000",
    "terminal.selectionBackground": "#00000040",
    "terminal.ansiBlack": "#000000",
    "terminal.ansiRed": "#CD3131",
    "terminal.ansiGreen": "#00BC00",
    "terminal.ansiYellow": "#949800",
    "terminal.ansiBlue": "#0451A5",
    "terminal.ansiMagenta": "#BC05BC",
    "terminal.ansiCyan": "#0598BC",
    "terminal.ansiWhite": "#555555",
    "terminal.ansiBrightBlack": "#666666",
    "terminal.ansiBrightRed": "#CD3131",
    "terminal.ansiBrightGreen": "#14CE14",
    "terminal.ansiBrightYellow": "#B5BA00",
    "terminal.ansiBrightBlue": "#0451A5",
    "terminal.ansiBrightMagenta": "#BC05BC",
    "terminal.ansiBrightCyan": "#0598BC",
    "terminal.ansiBrightWhite": "#A5A5A5",
  ]
}

// MARK: - NSColor hex parsing

private extension NSColor {
  convenience init?(hex: String) {
    var hexStr = hex.trimmingCharacters(in: .whitespacesAndNewlines)
    if hexStr.hasPrefix("#") { hexStr.removeFirst() }

    var rgb: UInt64 = 0
    guard Scanner(string: hexStr).scanHexInt64(&rgb) else { return nil }

    switch hexStr.count {
    case 6:
      self.init(
        red: CGFloat((rgb >> 16) & 0xFF) / 255,
        green: CGFloat((rgb >> 8) & 0xFF) / 255,
        blue: CGFloat(rgb & 0xFF) / 255,
        alpha: 1.0
      )
    case 8:
      self.init(
        red: CGFloat((rgb >> 24) & 0xFF) / 255,
        green: CGFloat((rgb >> 16) & 0xFF) / 255,
        blue: CGFloat((rgb >> 8) & 0xFF) / 255,
        alpha: CGFloat(rgb & 0xFF) / 255
      )
    default:
      return nil
    }
  }
}
