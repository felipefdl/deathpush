import AppKit
import Foundation

@Observable
final class IconThemeService {
  private var iconDefinitions: [String: String] = [:]
  private var fileNameMap: [String: String] = [:]
  private var fileExtensionMap: [String: String] = [:]
  private var folderNameMap: [String: String] = [:]
  private var folderNameExpandedMap: [String: String] = [:]
  private var rootFolderNameMap: [String: String] = [:]
  private var rootFolderNameExpandedMap: [String: String] = [:]

  private var languageIdMap: [String: String] = [:]

  private var lightFileNameMap: [String: String] = [:]
  private var lightFileExtensionMap: [String: String] = [:]
  private var lightFolderNameMap: [String: String] = [:]
  private var lightFolderNameExpandedMap: [String: String] = [:]

  // Maps file extensions to VS Code language IDs when the extension doesn't match
  // the languageId key directly. Most cases (html, css, rust, etc.) work by trying
  // the extension as a languageId key. This table handles the mismatches.
  private static let extensionToLanguageId: [String: String] = [
    "js": "javascript",
    "mjs": "javascript",
    "cjs": "javascript",
    "jsx": "javascriptreact",
    "ts": "typescript",
    "mts": "typescript",
    "cts": "typescript",
    "tsx": "typescriptreact",
    "py": "python",
    "pyw": "python",
    "rb": "ruby",
    "rs": "rust",
    "c": "c",
    "h": "c",
    "cc": "cpp",
    "cxx": "cpp",
    "hpp": "cpp",
    "hh": "cpp",
    "cs": "csharp",
    "kt": "kotlin",
    "kts": "kotlin",
    "ex": "elixir",
    "exs": "elixir",
    "erl": "erlang",
    "hrl": "erlang",
    "hs": "haskell",
    "lhs": "haskell",
    "clj": "clojure",
    "cljs": "clojure",
    "cljc": "clojure",
    "sc": "scala",
    "sh": "shellscript",
    "bash": "shellscript",
    "zsh": "shellscript",
    "fish": "shellscript",
    "ps1": "powershell",
    "psm1": "powershell",
    "bat": "bat",
    "cmd": "bat",
    "ml": "ocaml",
    "mli": "ocaml",
    "fs": "fsharp",
    "fsx": "fsharp",
    "pl": "perl",
    "pm": "perl",
    "gql": "graphql",
    "coffee": "coffeescript",
    "pug": "jade",
    "hbs": "handlebars",
    "ejs": "html",
    "erb": "ruby",
    "m": "objective-c",
    "mm": "objective-cpp",
    "jl": "julia",
    "rmd": "rsweave",
    "cls": "latex-class",
    "sty": "latex-package",
    "bib": "bibtex",
  ]

  private var defaultFileIconId = "file"
  private var defaultFolderIconId = "folder"
  private var defaultFolderExpandedIconId = "folder-open"
  private var defaultRootFolderIconId = "folder-root"
  private var defaultRootFolderExpandedIconId = "folder-root-open"

  private var imageCache: [String: NSImage] = [:]
  private let svgBaseURL: URL?

  init() {
    svgBaseURL = Bundle.main.url(forResource: "material", withExtension: nil, subdirectory: "IconThemes")
    loadThemeJSON()
  }

  func icon(
    for fileName: String,
    isDirectory: Bool,
    isExpanded: Bool = false,
    isRoot: Bool = false,
    isLight: Bool = false
  ) -> NSImage? {
    let iconId = resolveIconId(
      fileName: fileName,
      isDirectory: isDirectory,
      isExpanded: isExpanded,
      isRoot: isRoot,
      isLight: isLight
    )
    guard let iconId else { return nil }
    return loadImage(for: iconId)
  }

  // MARK: - Resolution

  private func resolveIconId(
    fileName: String,
    isDirectory: Bool,
    isExpanded: Bool,
    isRoot: Bool,
    isLight: Bool
  ) -> String? {
    let key = fileName.lowercased()

    if isDirectory {
      return resolveFolderIconId(name: key, isExpanded: isExpanded, isRoot: isRoot, isLight: isLight)
    }
    return resolveFileIconId(name: key, isLight: isLight)
  }

  private func resolveFileIconId(name: String, isLight: Bool) -> String {
    // Light filename override
    if isLight, let id = lightFileNameMap[name] { return id }

    // Exact filename match
    if let id = fileNameMap[name] { return id }

    // Extension matching (try compound extensions first: spec.ts, then ts)
    let parts = name.split(separator: ".")
    if parts.count > 1 {
      // Try compound extensions: e.g. "spec.ts", "test.js", "d.ts"
      for i in 1..<parts.count {
        let ext = parts[i...].joined(separator: ".").lowercased()
        if isLight, let id = lightFileExtensionMap[ext] { return id }
        if let id = fileExtensionMap[ext] { return id }
      }

      // Fallback: try extension as a languageId key directly (covers html, css, go, etc.),
      // then check the exception map for mismatches (js -> javascript, py -> python, etc.)
      let simpleExt = String(parts.last!).lowercased()
      if let id = languageIdMap[simpleExt] { return id }
      if let langId = Self.extensionToLanguageId[simpleExt],
         let id = languageIdMap[langId] {
        return id
      }
    }

    return defaultFileIconId
  }

  private func resolveFolderIconId(name: String, isExpanded: Bool, isRoot: Bool, isLight: Bool) -> String {
    if isRoot {
      let map = isExpanded ? rootFolderNameExpandedMap : rootFolderNameMap
      if let id = map[name] { return id }
    }

    if isLight {
      let lightMap = isExpanded ? lightFolderNameExpandedMap : lightFolderNameMap
      if let id = lightMap[name] { return id }
    }

    let map = isExpanded ? folderNameExpandedMap : folderNameMap
    if let id = map[name] { return id }

    if isRoot {
      return isExpanded ? defaultRootFolderExpandedIconId : defaultRootFolderIconId
    }
    return isExpanded ? defaultFolderExpandedIconId : defaultFolderIconId
  }

  // MARK: - Image Loading

  private func loadImage(for iconId: String) -> NSImage? {
    if let cached = imageCache[iconId] { return cached }

    guard let svgFilename = iconDefinitions[iconId],
          let baseURL = svgBaseURL else { return nil }

    let url = baseURL.appendingPathComponent(svgFilename)
    guard let image = NSImage(contentsOf: url) else { return nil }
    image.size = NSSize(width: 16, height: 16)
    imageCache[iconId] = image
    return image
  }

  // MARK: - JSON Loading

  private func loadThemeJSON() {
    guard let url = Bundle.main.url(
      forResource: "material-icon-theme",
      withExtension: "json",
      subdirectory: "IconThemes"
    ) else { return }

    guard let data = try? Data(contentsOf: url),
          let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }

    // Icon definitions
    if let defs = json["iconDefinitions"] as? [String: [String: String]] {
      for (id, value) in defs {
        if let path = value["iconPath"] {
          iconDefinitions[id] = path
        }
      }
    }

    // Default icon IDs
    if let v = json["file"] as? String { defaultFileIconId = v }
    if let v = json["folder"] as? String { defaultFolderIconId = v }
    if let v = json["folderExpanded"] as? String { defaultFolderExpandedIconId = v }
    if let v = json["rootFolder"] as? String { defaultRootFolderIconId = v }
    if let v = json["rootFolderExpanded"] as? String { defaultRootFolderExpandedIconId = v }

    // File/folder mappings (all keys lowercased)
    fileExtensionMap = extractStringMap(json, key: "fileExtensions")
    fileNameMap = extractStringMap(json, key: "fileNames")
    folderNameMap = extractStringMap(json, key: "folderNames")
    folderNameExpandedMap = extractStringMap(json, key: "folderNamesExpanded")
    rootFolderNameMap = extractStringMap(json, key: "rootFolderNames")
    rootFolderNameExpandedMap = extractStringMap(json, key: "rootFolderNamesExpanded")
    languageIdMap = extractStringMap(json, key: "languageIds")

    // Light overrides
    if let light = json["light"] as? [String: Any] {
      lightFileExtensionMap = extractStringMap(light, key: "fileExtensions")
      lightFileNameMap = extractStringMap(light, key: "fileNames")
      lightFolderNameMap = extractStringMap(light, key: "folderNames")
      lightFolderNameExpandedMap = extractStringMap(light, key: "folderNamesExpanded")
    }
  }

  private func extractStringMap(_ dict: [String: Any], key: String) -> [String: String] {
    guard let source = dict[key] as? [String: String] else { return [:] }
    var result: [String: String] = [:]
    for (k, v) in source {
      result[k.lowercased()] = v
    }
    return result
  }
}
