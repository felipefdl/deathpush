import SwiftUI

struct FileIconView: View {
  let fileName: String
  var isDirectory: Bool = false
  var isExpanded: Bool = false

  @Environment(AppState.self) private var appState

  var body: some View {
    if let nsImage = appState.iconThemeService.icon(
      for: fileName,
      isDirectory: isDirectory,
      isExpanded: isExpanded,
      isLight: appState.themeService.colorScheme == .light
    ) {
      Image(nsImage: nsImage)
        .frame(width: 16, height: 16)
    } else {
      Image(systemName: isDirectory ? "folder" : "doc")
        .font(.caption)
        .foregroundStyle(.secondary)
        .frame(width: 16)
    }
  }
}
