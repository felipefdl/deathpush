import SwiftUI

struct ThemePickerView: View {
  @Environment(AppState.self) private var appState
  @Environment(\.dismiss) private var dismiss
  @State private var query = ""
  @State private var selectedIndex = 0
  @State private var originalThemeName = ""

  private var flatThemes: [ThemeService.ThemeEntry] {
    let filtered: [ThemeService.ThemeEntry]
    if query.isEmpty {
      filtered = ThemeService.availableThemes
    } else {
      filtered = ThemeService.availableThemes.filter {
        $0.displayName.localizedCaseInsensitiveContains(query)
      }
    }

    var result: [ThemeService.ThemeEntry] = []
    for kind in ThemeService.ThemeKind.allCases {
      let group = filtered.filter { $0.kind == kind }
      result.append(contentsOf: group)
    }
    return result
  }

  var body: some View {
    VStack(spacing: 0) {
      HStack {
        Image(systemName: "magnifyingglass")
          .foregroundStyle(.secondary)
        TextField("Search themes...", text: $query)
          .textFieldStyle(.plain)
          .font(.title3)
          .onSubmit { confirmSelection() }
      }
      .padding(12)

      Divider()

      if flatThemes.isEmpty {
        ContentUnavailableView.search(text: query)
      } else {
        ScrollViewReader { proxy in
          List(selection: $selectedIndex) {
            ForEach(ThemeService.ThemeKind.allCases, id: \.self) { kind in
              let themesInGroup = flatThemes.enumerated().filter { $0.element.kind == kind }
              if !themesInGroup.isEmpty {
                Section {
                  ForEach(themesInGroup, id: \.element.name) { index, theme in
                    HStack {
                      Text(theme.displayName)
                        .font(.body)
                        .lineLimit(1)
                      Spacer()
                      if theme.name == appState.themeService.currentThemeName {
                        Image(systemName: "checkmark")
                          .foregroundStyle(.accent)
                          .font(.caption)
                      }
                    }
                    .tag(index)
                    .id(index)
                  }
                } header: {
                  Text(kind.label)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textCase(nil)
                }
              }
            }
          }
          .listStyle(.plain)
          .scrollEdgeEffectStyle(.soft, for: .top)
          .onChange(of: selectedIndex) {
            withAnimation {
              proxy.scrollTo(selectedIndex, anchor: .center)
            }
          }
        }
      }
    }
    .frame(width: 480, height: 400)
    .onAppear {
      originalThemeName = appState.themeService.currentThemeName
      if let index = flatThemes.firstIndex(where: { $0.name == originalThemeName }) {
        selectedIndex = index
      }
    }
    .onKeyPress(.escape) {
      revertAndDismiss()
      return .handled
    }
    .onKeyPress(.upArrow) {
      moveSelection(-1)
      return .handled
    }
    .onKeyPress(.downArrow) {
      moveSelection(1)
      return .handled
    }
    .onChange(of: query) {
      selectedIndex = 0
      applyPreview()
    }
    .onChange(of: selectedIndex) {
      applyPreview()
    }
  }

  private func moveSelection(_ delta: Int) {
    guard !flatThemes.isEmpty else { return }
    let newIndex = selectedIndex + delta
    if newIndex >= 0, newIndex < flatThemes.count {
      selectedIndex = newIndex
    }
  }

  private func applyPreview() {
    guard selectedIndex >= 0, selectedIndex < flatThemes.count else { return }
    appState.themeService.applyTheme(flatThemes[selectedIndex].name)
  }

  private func confirmSelection() {
    dismiss()
  }

  private func revertAndDismiss() {
    appState.themeService.applyTheme(originalThemeName)
    dismiss()
  }
}
