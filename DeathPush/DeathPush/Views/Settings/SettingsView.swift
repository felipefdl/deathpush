import SwiftUI

struct SettingsView: View {
  var body: some View {
    TabView {
      AppearanceSettingsTab()
        .tabItem { Label("Appearance", systemImage: "paintbrush") }

      EditorSettingsTab()
        .tabItem { Label("Editor", systemImage: "pencil") }

      GitSettingsTab()
        .tabItem { Label("Git", systemImage: "arrow.triangle.branch") }
    }
    .frame(width: 500, height: 450)
  }
}

struct AppearanceSettingsTab: View {
  @Environment(AppState.self) private var appState

  var body: some View {
    Form {
      Section("Theme") {
        Picker("Color Theme", selection: Binding(
          get: { appState.themeService.currentThemeName },
          set: { appState.themeService.applyTheme($0) }
        )) {
          ForEach(ThemeService.availableThemes, id: \.name) { theme in
            Text(theme.displayName).tag(theme.name)
          }
        }
      }
    }
    .formStyle(.grouped)
    .padding()
  }
}

struct EditorSettingsTab: View {
  @AppStorage("editor.fontSize") private var fontSize = 13.0
  @AppStorage("editor.fontFamily") private var fontFamily = "SF Mono"
  @AppStorage("editor.lineHeight") private var lineHeight = 20.0
  @AppStorage("editor.tabSize") private var tabSize = 2
  @AppStorage("editor.wordWrap") private var wordWrap = false
  @AppStorage("editor.renderWhitespace") private var renderWhitespace = false
  @AppStorage("editor.diffMode") private var diffMode = "sideBySide"

  var body: some View {
    Form {
      Section("Font") {
        HStack {
          Text("Font Family")
          Spacer()
          TextField("", text: $fontFamily)
            .frame(width: 200)
            .textFieldStyle(.roundedBorder)
        }
        HStack {
          Text("Font Size")
          Spacer()
          Stepper("\(Int(fontSize))pt", value: $fontSize, in: 8...32)
        }
        HStack {
          Text("Line Height")
          Spacer()
          Stepper("\(Int(lineHeight))px", value: $lineHeight, in: 12...48)
        }
      }

      Section("Editor") {
        Stepper("Tab Size: \(tabSize)", value: $tabSize, in: 1...8)
        Toggle("Word Wrap", isOn: $wordWrap)
        Toggle("Render Whitespace", isOn: $renderWhitespace)
        Picker("Diff Mode", selection: $diffMode) {
          Text("Side by Side").tag("sideBySide")
          Text("Inline").tag("inline")
        }
      }
    }
    .formStyle(.grouped)
    .padding()
  }
}

struct GitSettingsTab: View {
  @AppStorage("git.autoFetch") private var autoFetch = true
  @AppStorage("git.autoFetchInterval") private var autoFetchInterval = 300
  @AppStorage("git.confirmDiscard") private var confirmDiscard = true

  @State private var userName = ""
  @State private var userEmail = ""

  var body: some View {
    Form {
      Section("Git Identity") {
        HStack {
          Text("Name")
          Spacer()
          TextField("Your Name", text: $userName)
            .frame(width: 250)
            .textFieldStyle(.roundedBorder)
            .onSubmit { saveGitConfig(key: "user.name", value: userName) }
        }
        HStack {
          Text("Email")
          Spacer()
          TextField("your@email.com", text: $userEmail)
            .frame(width: 250)
            .textFieldStyle(.roundedBorder)
            .onSubmit { saveGitConfig(key: "user.email", value: userEmail) }
        }
      }

      Section("Fetch") {
        Toggle("Auto Fetch", isOn: $autoFetch)
        if autoFetch {
          Stepper("Interval: \(autoFetchInterval / 60) min", value: $autoFetchInterval, in: 60...3600, step: 60)
        }
      }

      Section("Safety") {
        Toggle("Confirm Before Discard", isOn: $confirmDiscard)
      }
    }
    .formStyle(.grouped)
    .padding()
    .onAppear {
      userName = (try? RepositoryService.getGlobalGitConfig(key: "user.name")) ?? ""
      userEmail = (try? RepositoryService.getGlobalGitConfig(key: "user.email")) ?? ""
    }
    .onChange(of: userName) { oldValue, newValue in
      if !oldValue.isEmpty { saveGitConfig(key: "user.name", value: newValue) }
    }
    .onChange(of: userEmail) { oldValue, newValue in
      if !oldValue.isEmpty { saveGitConfig(key: "user.email", value: newValue) }
    }
  }

  private func saveGitConfig(key: String, value: String) {
    try? RepositoryService.setGlobalGitConfig(key: key, value: value)
  }
}
