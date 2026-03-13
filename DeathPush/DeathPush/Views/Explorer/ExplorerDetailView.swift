import SwiftUI

struct ExplorerDetailView: View {
  let path: String
  var goToLine: Int?
  @Environment(AppState.self) private var appState
  @Environment(TabState.self) private var tabState
  @State private var fileContent: FileContent?
  @State private var errorMessage: String?

  private var repoService: RepositoryService? {
    tabState.repoService
  }

  var body: some View {
    VStack(spacing: 0) {
      ExplorerDetailHeader(path: path, repoService: repoService)

      Divider()

      if tabState.explorerShowBlame {
        BlameView(path: path)
      } else if tabState.explorerShowFileHistory {
        FileHistoryView(path: path)
      } else if let error = errorMessage, fileContent == nil {
        ContentUnavailableView(
          "Error Loading File",
          systemImage: "exclamationmark.triangle",
          description: Text(error)
        )
      } else if let content = fileContent {
        switch content.fileType {
        case "text":
          MonacoEditorView(fileContent: content, themeJSON: appState.themeService.themeDataJSON, goToLine: goToLine)
            .background(Color(nsColor: appState.themeService.color(forKey: "editor.background") ?? .black))
        case "image":
          ExplorerImageView(dataURI: content.content)
        case "binary":
          ContentUnavailableView(
            "Binary File",
            systemImage: "doc.fill",
            description: Text("This file cannot be displayed as text.")
          )
        case "large":
          ContentUnavailableView(
            "File Too Large",
            systemImage: "exclamationmark.triangle",
            description: Text("This file is too large to display.")
          )
        default:
          ContentUnavailableView(
            "Unsupported File",
            systemImage: "questionmark.circle",
            description: Text("Unable to display this file type.")
          )
        }
      } else {
        ProgressView("Loading...")
          .frame(maxWidth: .infinity, maxHeight: .infinity)
      }
    }
    .onChange(of: path, initial: true) { _, newPath in
      tabState.explorerShowBlame = false
      tabState.explorerShowFileHistory = false
      loadFile(for: newPath)
    }
  }

  private func loadFile(for filePath: String) {
    errorMessage = nil

    do {
      fileContent = try repoService?.readExplorerFile(path: filePath)
    } catch {
      errorMessage = error.localizedDescription
      fileContent = nil
    }
  }
}

struct ExplorerDetailHeader: View {
  let path: String
  let repoService: RepositoryService?
  @Environment(TabState.self) private var tabState

  var body: some View {
    @Bindable var tabState = tabState

    GlassEffectContainer(spacing: 8) {
      HStack {
        HStack(spacing: 4) {
          Image(systemName: "doc.text")
            .foregroundStyle(.secondary)
          Text(path)
            .font(.callout)
            .lineLimit(1)
            .truncationMode(.middle)
        }

        Spacer()

        blameButton
        fileHistoryButton

        Button(action: {
          try? openInEditor(sessionId: repoService?.sessionId ?? "", path: path)
        }) {
          Image(systemName: "pencil.line")
        }
        .buttonStyle(.glass)
        .controlSize(.small)
        .help("Open in Editor")

        Button(action: {
          try? revealInFileManager(sessionId: repoService?.sessionId ?? "", path: path)
        }) {
          Image(systemName: "folder")
        }
        .buttonStyle(.glass)
        .controlSize(.small)
        .help("Reveal in Finder")
      }
      .padding(.horizontal, 12)
      .padding(.vertical, 6)
    }
  }

  @ViewBuilder
  private var blameButton: some View {
    if tabState.explorerShowBlame {
      Button(action: { tabState.explorerShowBlame = false }) {
        Image(systemName: "person.text.rectangle")
      }
      .buttonStyle(.glassProminent)
      .controlSize(.small)
      .help("Git Blame")
    } else {
      Button(action: {
        tabState.explorerShowBlame = true
        tabState.explorerShowFileHistory = false
      }) {
        Image(systemName: "person.text.rectangle")
      }
      .buttonStyle(.glass)
      .controlSize(.small)
      .help("Git Blame")
    }
  }

  @ViewBuilder
  private var fileHistoryButton: some View {
    if tabState.explorerShowFileHistory {
      Button(action: { tabState.explorerShowFileHistory = false }) {
        Image(systemName: "clock.arrow.circlepath")
      }
      .buttonStyle(.glassProminent)
      .controlSize(.small)
      .help("File History")
    } else {
      Button(action: {
        tabState.explorerShowFileHistory = true
        tabState.explorerShowBlame = false
      }) {
        Image(systemName: "clock.arrow.circlepath")
      }
      .buttonStyle(.glass)
      .controlSize(.small)
      .help("File History")
    }
  }
}

struct ExplorerImageView: View {
  let dataURI: String

  var body: some View {
    if let image = parseDataURI(dataURI) {
      ScrollView([.horizontal, .vertical]) {
        Image(nsImage: image)
          .resizable()
          .aspectRatio(contentMode: .fit)
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .padding()
      }
    } else {
      ContentUnavailableView(
        "Cannot Display Image",
        systemImage: "photo",
        description: Text("The image format is not supported.")
      )
    }
  }

  private func parseDataURI(_ uri: String) -> NSImage? {
    // data:image/png;base64,iVBOR...
    guard uri.hasPrefix("data:"),
          let commaIndex = uri.firstIndex(of: ",") else {
      return nil
    }
    let base64String = String(uri[uri.index(after: commaIndex)...])
    guard let data = Data(base64Encoded: base64String) else { return nil }
    return NSImage(data: data)
  }
}
