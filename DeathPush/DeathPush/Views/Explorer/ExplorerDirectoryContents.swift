import SwiftUI

struct ExplorerDirectoryContents: View {
  let directoryPath: String?
  @Binding var selectedPath: String?
  let filterText: String
  let repoService: RepositoryService?
  let contextMenus: ExplorerContextMenus

  private var cacheKey: String {
    directoryPath ?? "__root__"
  }

  private var entries: [ExplorerEntry] {
    let cached = repoService?.explorerCache[cacheKey] ?? []
    if filterText.isEmpty { return cached }
    let query = filterText.lowercased()
    return cached.filter { entry in
      if entry.isDirectory { return true }
      return entry.name.lowercased().contains(query)
    }
  }

  private var gitStatusByPath: [String: FileStatus] {
    repoService?.gitStatusByPath ?? [:]
  }

  var body: some View {
    ForEach(entries, id: \.path) { entry in
      if entry.isDirectory {
        ExplorerFolderRow(
          entry: entry,
          depth: 0,
          repoService: repoService,
          contextMenus: contextMenus
        )
      } else {
        ExplorerFileRow(
          entry: entry,
          depth: 0,
          isSelected: selectedPath == entry.path,
          gitStatus: gitStatusByPath[entry.path],
          onSelect: { selectedPath = entry.path }
        )
        .contextMenu {
          contextMenus.fileMenu(entry: entry)
        }
      }
    }
  }
}
