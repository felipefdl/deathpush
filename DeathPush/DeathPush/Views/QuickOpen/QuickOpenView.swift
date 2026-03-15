import SwiftUI

struct QuickOpenView: View {
  @Environment(TabState.self) private var tabState
  @State private var query = ""
  @State private var results: [FuzzyFileResult] = []
  @State private var contentResults: [ContentSearchResult] = []
  @State private var selectedIndex = 0

  let onSelect: (String, Int?) -> Void

  private var isContentSearch: Bool { query.hasPrefix("#") }
  private var contentQuery: String {
    isContentSearch ? String(query.dropFirst()) : ""
  }

  private var parsedQuery: (search: String, line: Int?) {
    guard !isContentSearch else { return (query, nil) }
    guard let colonRange = query.range(of: ":", options: .backwards),
          let line = Int(query[colonRange.upperBound...]) else {
      return (query, nil)
    }
    return (String(query[..<colonRange.lowerBound]), line)
  }

  var body: some View {
    VStack(spacing: 0) {
      HStack {
        Image(systemName: "magnifyingglass")
          .foregroundStyle(.secondary)
        TextField("Search files (file:line, # content)...", text: $query)
          .textFieldStyle(.plain)
          .font(.title3)
          .onSubmit {
            if isContentSearch {
              if let result = contentResults[safe: selectedIndex] {
                onSelect(result.path, Int(result.lineNumber))
              }
            } else if parsedQuery.search.isEmpty {
              if let recentFile = tabState.recentFiles[safe: selectedIndex] {
                onSelect(recentFile.path, nil)
              }
            } else {
              if let result = results[safe: selectedIndex] {
                onSelect(result.path, parsedQuery.line)
              }
            }
          }
      }
      .padding(12)

      Divider()

      if isContentSearch {
        contentResultsList
      } else {
        fileResultsList
      }
    }
    .frame(width: 640, height: 420)
    .onChange(of: query) { _, newValue in
      guard let sessionId = tabState.repoService?.sessionId else { return }
      if isContentSearch {
        let q = contentQuery
        guard !q.isEmpty else {
          contentResults = []
          selectedIndex = 0
          return
        }
        contentResults = (try? searchFileContents(sessionId: sessionId, query: q, maxResults: 50)) ?? []
        results = []
      } else {
        let searchTerm = parsedQuery.search
        results = (try? fuzzyFindFiles(sessionId: sessionId, query: searchTerm, maxResults: 50)) ?? []
        contentResults = []
      }
      selectedIndex = 0
    }
  }

  @ViewBuilder
  private var fileResultsList: some View {
    if parsedQuery.search.isEmpty {
      recentFilesList
    } else if results.isEmpty {
      ContentUnavailableView.search(text: parsedQuery.search)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    } else {
      List(Array(results.enumerated()), id: \.element.path, selection: $selectedIndex) { index, result in
        HStack {
          FileIconView(fileName: URL(fileURLWithPath: result.path).lastPathComponent)
          Text(result.path)
            .font(.body.monospaced())
            .lineLimit(1)
          if let line = parsedQuery.line {
            Text(":\(line)")
              .font(.body.monospaced())
              .foregroundStyle(.secondary)
          }
          Spacer()
          Text(parentPath(of: result.path))
            .font(.caption)
            .foregroundStyle(.tertiary)
        }
        .tag(index)
      }
      .listStyle(.plain)
      .scrollEdgeEffectStyle(.soft, for: .top)
    }
  }

  @ViewBuilder
  private var recentFilesList: some View {
    if tabState.recentFiles.isEmpty {
      ContentUnavailableView("No Recent Files", systemImage: "clock")
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    } else {
      List(Array(tabState.recentFiles.enumerated()), id: \.element.path, selection: $selectedIndex) { index, recentFile in
        HStack {
          FileIconView(fileName: URL(fileURLWithPath: recentFile.path).lastPathComponent)
          Text(URL(fileURLWithPath: recentFile.path).lastPathComponent)
            .font(.body.monospaced())
            .lineLimit(1)
          Spacer()
          Text(parentPath(of: recentFile.path))
            .font(.caption)
            .foregroundStyle(.tertiary)
        }
        .tag(index)
      }
      .listStyle(.plain)
      .scrollEdgeEffectStyle(.soft, for: .top)
    }
  }

  @ViewBuilder
  private var contentResultsList: some View {
    if contentResults.isEmpty && !contentQuery.isEmpty {
      ContentUnavailableView.search(text: contentQuery)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    } else {
      List(Array(contentResults.enumerated()), id: \.offset, selection: $selectedIndex) { index, result in
        HStack(spacing: 6) {
          FileIconView(fileName: URL(fileURLWithPath: result.path).lastPathComponent)
          Text("\(result.path):\(result.lineNumber)")
            .font(.body.monospaced())
            .foregroundStyle(.primary)
            .lineLimit(1)
          Text(result.lineContent.trimmingCharacters(in: .whitespaces))
            .font(.caption.monospaced())
            .foregroundStyle(.secondary)
            .lineLimit(1)
            .truncationMode(.tail)
        }
        .tag(index)
      }
      .listStyle(.plain)
      .scrollEdgeEffectStyle(.soft, for: .top)
    }
  }

  private func parentPath(of path: String) -> String {
    let components = path.split(separator: "/").dropLast()
    return components.joined(separator: "/")
  }
}

extension Collection {
  subscript(safe index: Index) -> Element? {
    indices.contains(index) ? self[index] : nil
  }
}
