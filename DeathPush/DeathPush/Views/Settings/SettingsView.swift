import AppKit
import SwiftUI
import UserNotifications

private let monospaceFontFamilies: [String] = {
  let fontManager = NSFontManager.shared
  let allFamilies = fontManager.availableFontFamilies
  let nameHints = ["Mono", "Code", "Courier", "Consol", "Nerd Font", "Terminal"]
  var result = allFamilies.filter { family in
    let lowerFamily = family.lowercased()
    if nameHints.contains(where: { lowerFamily.contains($0.lowercased()) }) {
      return true
    }
    guard let members = fontManager.availableMembers(ofFontFamily: family),
          let firstMember = members.first,
          let postscriptName = firstMember[0] as? String,
          let font = NSFont(name: postscriptName, size: 13) else { return false }
    let traits = fontManager.traits(of: font)
    return traits.contains(.fixedPitchFontMask)
  }
  // SF Mono is a system font not listed in availableFontFamilies
  if !result.contains("SF Mono") {
    result.append("SF Mono")
  }
  return result.sorted()
}()

struct MonospaceFontPicker: View {
  @Binding var selection: String

  private var families: [String] {
    if monospaceFontFamilies.contains(selection) {
      return monospaceFontFamilies
    }
    return (monospaceFontFamilies + [selection]).sorted()
  }

  var body: some View {
    Picker("Font Family", selection: $selection) {
      ForEach(families, id: \.self) { family in
        Text(family)
          .font(.custom(family, size: 13))
          .tag(family)
      }
    }
  }
}

struct SettingsView: View {
  var body: some View {
    TabView {
      AppearanceSettingsTab()
        .tabItem { Label("Appearance", systemImage: "paintbrush") }

      EditorSettingsTab()
        .tabItem { Label("Editor", systemImage: "pencil") }

      TerminalSettingsTab()
        .tabItem { Label("Terminal", systemImage: "terminal") }

      GitSettingsTab()
        .tabItem { Label("Git", systemImage: "arrow.triangle.branch") }
    }
    .frame(width: 500, height: 450)
  }
}

struct AppearanceSettingsTab: View {
  @Environment(AppState.self) private var appState

  private var darkThemes: [ThemeService.ThemeEntry] {
    ThemeService.availableThemes.filter { !$0.isLight }
  }

  private var lightThemes: [ThemeService.ThemeEntry] {
    ThemeService.availableThemes.filter { $0.isLight }
  }

  var body: some View {
    Form {
      Section("Theme") {
        Picker("Dark Theme", selection: Binding(
          get: { appState.themeService.preferredDarkTheme },
          set: { appState.themeService.preferredDarkTheme = $0 }
        )) {
          ForEach(darkThemes, id: \.name) { theme in
            Text(theme.displayName).tag(theme.name)
          }
        }

        Picker("Light Theme", selection: Binding(
          get: { appState.themeService.preferredLightTheme },
          set: { appState.themeService.preferredLightTheme = $0 }
        )) {
          ForEach(lightThemes, id: \.name) { theme in
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
        MonospaceFontPicker(selection: $fontFamily)
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

struct TerminalSettingsTab: View {
  @AppStorage("terminal.fontFamily") private var fontFamily = "SF Mono"
  @AppStorage("terminal.fontSize") private var fontSize = 13.0
  @AppStorage("terminal.cursorStyle") private var cursorStyle = "block"
  @AppStorage("terminal.cursorBlink") private var cursorBlink = true
  @AppStorage("terminal.optionAsMeta") private var optionAsMeta = false
  @AppStorage("terminal.mouseReporting") private var mouseReporting = true
  @AppStorage("terminal.scrollback") private var scrollback = 5000
  @AppStorage("terminal.boldAsBright") private var boldAsBright = true
  @AppStorage("terminal.bellNotification") private var bellNotification = true
  @AppStorage("terminal.processNotification") private var processNotification = true

  var body: some View {
    Form {
      Section("Font") {
        MonospaceFontPicker(selection: $fontFamily)
        HStack {
          Text("Font Size")
          Spacer()
          Stepper("\(Int(fontSize))pt", value: $fontSize, in: 8...32)
        }
      }

      Section("Cursor") {
        Picker("Style", selection: $cursorStyle) {
          Text("Block").tag("block")
          Text("Underline").tag("underline")
          Text("Bar").tag("bar")
        }
        Toggle("Blink", isOn: $cursorBlink)
      }

      Section("Behavior") {
        Toggle("Option as Meta Key", isOn: $optionAsMeta)
        Toggle("Mouse Reporting", isOn: $mouseReporting)
        Stepper("Scrollback: \(scrollback) lines", value: $scrollback, in: 100...100_000, step: 500)
        Text("Scrollback changes apply to new terminal sessions.")
          .font(.caption)
          .foregroundStyle(.secondary)
      }

      Section("Rendering") {
        Toggle("Bold Text as Bright Colors", isOn: $boldAsBright)
      }

      Section("Notifications") {
        Toggle("Bell Notification", isOn: $bellNotification)
          .onChange(of: bellNotification) { _, enabled in
            if enabled { requestNotificationPermission() }
          }
        Toggle("Process Completion", isOn: $processNotification)
          .onChange(of: processNotification) { _, enabled in
            if enabled { requestNotificationPermission() }
          }
        Text("Notifications are sent when the app is in the background.")
          .font(.caption)
          .foregroundStyle(.secondary)
      }
    }
    .formStyle(.grouped)
    .padding()
  }
}

private func requestNotificationPermission() {
  UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in }
}

struct GitSettingsTab: View {
  @AppStorage("git.autoFetch") private var autoFetch = true
  @AppStorage("git.autoFetchInterval") private var autoFetchInterval = 300
  @AppStorage("git.confirmDiscard") private var confirmDiscard = true
  @AppStorage("git.inlineBlame") private var inlineBlame = true

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

      Section("Blame") {
        Toggle("Inline Blame in Status Bar", isOn: $inlineBlame)
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
