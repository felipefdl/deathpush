import SwiftUI

struct ExplorerContextMenus {
  let repoService: RepositoryService?
  let onNewFile: (String) -> Void
  let onNewFolder: (String) -> Void
  let explorerClipboard: ExplorerClipboard?
  let onCopy: ([String]) -> Void
  let onCut: ([String]) -> Void
  let onPaste: (String) -> Void
  let onImport: (String) -> Void

  @ViewBuilder
  func fileMenu(entry: ExplorerEntry) -> some View {
    let parentDir = URL(fileURLWithPath: entry.path).deletingLastPathComponent().path

    Button("New File...") { onNewFile(parentDir) }
    Button("New Folder...") { onNewFolder(parentDir) }

    Divider()

    Button("Copy") { onCopy([entry.path]) }
    Button("Cut") { onCut([entry.path]) }
    if explorerClipboard != nil {
      Button("Paste") {
        let pasteDir = entry.path.components(separatedBy: "/").dropLast().joined(separator: "/")
        onPaste(pasteDir)
      }
    }

    Divider()

    Button("Open in Editor") {
      try? openInEditor(sessionId: repoService?.sessionId ?? "", path: entry.path)
    }

    Divider()

    Button("Duplicate") {
      _ = try? repoService?.duplicateExplorerEntry(path: entry.path)
    }

    Button("Reveal in Finder") {
      try? revealInFileManager(sessionId: repoService?.sessionId ?? "", path: entry.path)
    }

    Button("Copy Path") {
      NSPasteboard.general.clearContents()
      NSPasteboard.general.setString(entry.path, forType: .string)
    }

    Button("Copy Relative Path") {
      let root = repoService?.status?.root ?? ""
      let relative = entry.path.hasPrefix(root)
        ? String(entry.path.dropFirst(root.count + 1))
        : entry.path
      NSPasteboard.general.clearContents()
      NSPasteboard.general.setString(relative, forType: .string)
    }

    Divider()

    Button("Add to .gitignore") {
      let root = repoService?.status?.root ?? ""
      let pattern = entry.path.hasPrefix(root)
        ? String(entry.path.dropFirst(root.count + 1))
        : entry.name
      try? repoService?.addExplorerEntryToGitignore(pattern: pattern)
    }

    Divider()

    Button("Move to Trash", role: .destructive) {
      try? repoService?.deleteExplorerEntry(path: entry.path)
    }
  }

  @ViewBuilder
  func folderMenu(entry: ExplorerEntry) -> some View {
    Button("New File...") { onNewFile(entry.path) }
    Button("New Folder...") { onNewFolder(entry.path) }

    Divider()

    Button("Copy") { onCopy([entry.path]) }
    Button("Cut") { onCut([entry.path]) }
    if explorerClipboard != nil {
      Button("Paste") { onPaste(entry.path) }
    }

    Divider()

    Button("Import Files...") { onImport(entry.path) }

    Divider()

    Button("Duplicate") {
      _ = try? repoService?.duplicateExplorerEntry(path: entry.path)
    }

    Button("Reveal in Finder") {
      try? revealInFileManager(sessionId: repoService?.sessionId ?? "", path: entry.path)
    }

    Button("Copy Path") {
      NSPasteboard.general.clearContents()
      NSPasteboard.general.setString(entry.path, forType: .string)
    }

    Button("Copy Relative Path") {
      let root = repoService?.status?.root ?? ""
      let relative = entry.path.hasPrefix(root)
        ? String(entry.path.dropFirst(root.count + 1))
        : entry.path
      NSPasteboard.general.clearContents()
      NSPasteboard.general.setString(relative, forType: .string)
    }

    Divider()

    Button("Add to .gitignore") {
      let root = repoService?.status?.root ?? ""
      let pattern = entry.path.hasPrefix(root)
        ? String(entry.path.dropFirst(root.count + 1)) + "/"
        : entry.name + "/"
      try? repoService?.addExplorerEntryToGitignore(pattern: pattern)
    }

    Divider()

    Button("Move to Trash", role: .destructive) {
      try? repoService?.deleteExplorerEntry(path: entry.path)
    }
  }
}
