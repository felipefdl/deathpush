import SwiftUI

enum InlineCreationMode: Equatable {
  case newFile(parentPath: String)
  case newFolder(parentPath: String)
}

struct FlatExplorerItem {
  let entry: ExplorerEntry
  let depth: Int
}

struct ExplorerTreeView: View {
  @Binding var selectedFilePath: String?
  @Environment(TabState.self) private var tabState
  @State private var filterText = ""
  @State private var creationMode: InlineCreationMode?
  @State private var creationName = ""
  @State private var renameTarget: ExplorerEntry?
  @State private var renameName = ""

  private var repoService: RepositoryService? {
    tabState.repoService
  }

  private var gitStatusByPath: [String: FileStatus] {
    repoService?.gitStatusByPath ?? [:]
  }

  private var flatItems: [FlatExplorerItem] {
    var items: [FlatExplorerItem] = []
    buildFlatList(directoryPath: nil, depth: 0, into: &items)
    return items
  }

  private func buildFlatList(directoryPath: String?, depth: Int, into items: inout [FlatExplorerItem]) {
    let cacheKey = directoryPath ?? "__root__"
    let entries = repoService?.explorerCache[cacheKey] ?? []
    let filtered: [ExplorerEntry]
    if filterText.isEmpty {
      filtered = entries
    } else {
      let query = filterText.lowercased()
      filtered = entries.filter { entry in
        if entry.isDirectory { return true }
        return entry.name.lowercased().contains(query)
      }
    }

    for entry in filtered {
      items.append(FlatExplorerItem(entry: entry, depth: depth))
      if entry.isDirectory, repoService?.explorerExpandedPaths.contains(entry.path) == true {
        buildFlatList(directoryPath: entry.path, depth: depth + 1, into: &items)
      }
    }
  }

  private var contextMenus: ExplorerContextMenus {
    ExplorerContextMenus(
      repoService: repoService,
      onNewFile: { parentPath in
        creationMode = .newFile(parentPath: parentPath)
        creationName = ""
      },
      onNewFolder: { parentPath in
        creationMode = .newFolder(parentPath: parentPath)
        creationName = ""
      }
    )
  }

  var body: some View {
    VStack(spacing: 0) {
      // Filter bar
      HStack(spacing: 6) {
        Image(systemName: "line.3.horizontal.decrease")
          .font(.caption)
          .foregroundStyle(.secondary)
        TextField("Filter", text: $filterText)
          .textFieldStyle(.plain)
          .font(.callout)
      }
      .padding(.horizontal, 10)
      .padding(.vertical, 6)

      Divider()

      List(selection: $selectedFilePath) {
        ForEach(flatItems, id: \.entry.path) { item in
          if item.entry.isDirectory {
            ExplorerFolderRow(
              entry: item.entry,
              depth: item.depth,
              repoService: repoService,
              contextMenus: contextMenus
            )
          } else {
            ExplorerFileRow(
              entry: item.entry,
              depth: item.depth,
              isSelected: selectedFilePath == item.entry.path,
              gitStatus: gitStatusByPath[item.entry.path],
              onSelect: { selectedFilePath = item.entry.path }
            )
            .contextMenu {
              contextMenus.fileMenu(entry: item.entry)
            }
          }
        }
      }
      .listStyle(.plain)
      .scrollEdgeEffectStyle(.soft, for: .top)
      .contextMenu {
        // Root-level context menu
        Button("New File...") {
          let root = repoService?.status?.root ?? ""
          creationMode = .newFile(parentPath: root)
          creationName = ""
        }
        Button("New Folder...") {
          let root = repoService?.status?.root ?? ""
          creationMode = .newFolder(parentPath: root)
          creationName = ""
        }
      }
    }
    .onAppear {
      _ = try? repoService?.listExplorerDirectory(path: nil)
    }
    .sheet(item: creationModeBinding) { mode in
      ExplorerCreationSheet(
        mode: mode,
        repoService: repoService,
        onDismiss: { creationMode = nil }
      )
    }
  }

  private var creationModeBinding: Binding<InlineCreationMode?> {
    Binding(
      get: { creationMode },
      set: { creationMode = $0 }
    )
  }
}

// Make InlineCreationMode Identifiable for .sheet(item:)
extension InlineCreationMode: Identifiable {
  var id: String {
    switch self {
    case .newFile(let path): "file:\(path)"
    case .newFolder(let path): "folder:\(path)"
    }
  }
}

struct ExplorerCreationSheet: View {
  let mode: InlineCreationMode
  let repoService: RepositoryService?
  let onDismiss: () -> Void
  @State private var name = ""

  private var title: String {
    switch mode {
    case .newFile: "New File"
    case .newFolder: "New Folder"
    }
  }

  private var parentPath: String {
    switch mode {
    case .newFile(let path): path
    case .newFolder(let path): path
    }
  }

  var body: some View {
    VStack(spacing: 12) {
      Text(title)
        .font(.headline)

      TextField("Name", text: $name)
        .textFieldStyle(.roundedBorder)
        .onSubmit { create() }

      HStack {
        Button("Cancel") { onDismiss() }
          .buttonStyle(.glass)
          .keyboardShortcut(.cancelAction)
        Spacer()
        Button("Create") { create() }
          .buttonStyle(.glassProminent)
          .keyboardShortcut(.defaultAction)
          .disabled(name.isEmpty)
      }
    }
    .padding()
    .frame(width: 350)
  }

  private func create() {
    guard !name.isEmpty else { return }
    let fullPath = URL(fileURLWithPath: parentPath).appendingPathComponent(name).path

    switch mode {
    case .newFile:
      try? repoService?.createExplorerFile(path: fullPath)
    case .newFolder:
      try? repoService?.createExplorerDirectory(path: fullPath)
    }

    onDismiss()
  }
}
