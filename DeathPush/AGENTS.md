# DeathPush macOS Native App

Native macOS 26 (Tahoe) Git client built with SwiftUI and Liquid Glass. Shares a Rust backend with the Tauri v2 app via UniFFI.

## Architecture

```
DeathPushApp (SwiftUI @main)
  -> ContentView (routes: WelcomeScreen vs RepositoryView)
    -> AppState (@Observable, single source of truth)
      -> RepositoryService (@Observable, wraps all Rust FFI calls)
      -> EventBridge (Rust -> Swift event listener)

Rust FFI Layer:
  Swift Views -> RepositoryService -> UniFFI free functions -> deathpush-ffi -> deathpush-core
```

### Key Layers

| Layer | Role | Pattern |
|-------|------|---------|
| Views | UI rendering only | Thin, delegate to services |
| Services | Business logic | `@Observable final class`, owns state |
| Bridge | Rust FFI | UniFFI-generated, never edit manually |
| Rust | Git operations | Session-scoped, all ops take `sessionId` |

### Session Model

Every `RepositoryService` creates a unique `sessionId` (UUID) on init and calls `createSession()`. All FFI functions are scoped to this ID for multi-window support. Call `destroy()` to clean up the Rust-side session.

## Build & Run

### Prerequisites

- Xcode 26 beta (macOS 26 SDK)
- Rust toolchain (`rustup target add aarch64-apple-darwin`)
- Monaco bundled: `./scripts/bundle-monaco.sh` (one-time after npm install)

### Build Phases (Xcode, in order)

1. **Build Rust Library** (`scripts/build-rust.sh`) -- compiles `deathpush-ffi` crate
2. **Generate UniFFI Bindings** (`scripts/generate-bindings.sh`) -- produces Swift + header files
3. **Compile Sources** (Xcode default)
4. **Copy Monaco Resources** (`scripts/copy-monaco.sh`) -- preserves `vs/` directory structure

### Common Build Issues

- **`cargo: command not found`**: Both Rust scripts source `$HOME/.cargo/env`. If you move your Rust install, update the scripts.
- **Monaco loads blank**: The `vs/` directory must preserve its hierarchy. Xcode's auto-sync flattens resources; that's why we use a custom copy script instead.
- **Linker errors for FFI symbols**: Check `LIBRARY_SEARCH_PATHS` matches the Rust target dir (debug vs release).

## Code Conventions

### Swift

- **Minimum target**: macOS 26.0 -- use all new APIs freely, no backwards compatibility
- **Swift version**: 6.2+ with strict concurrency (`SWIFT_DEFAULT_ACTOR_ISOLATION = MainActor`)
- **State**: `@Observable final class` for services, `@State` for view-local, `@Environment` for injection
- **Views**: Thin -- no business logic, delegate to services via `try? repoService?.method()`
- **Indentation**: Tabs (Xcode default), match existing files
- **File naming**: PascalCase matching the primary type (`RepositoryService.swift`, `SCMView.swift`)
- **Folder structure**: Feature-based under `Views/` (Sidebar, Main, History, Terminal, etc.)

### Liquid Glass (macOS 26)

Glass applies to the **navigation layer only** -- never on content rows, diff output, or terminal text.

| Element | API |
|---------|-----|
| Primary action buttons (commit, sync) | `.buttonStyle(.glassProminent)` |
| Secondary action buttons | `.buttonStyle(.glass)` |
| Grouped navigation controls | `GlassEffectContainer(spacing:) { }` |
| Badges, indicators | `.glassEffect(.regular)` |
| Tinted banners (merge/error) | `.glassEffect(.regular.tint(.color))` |
| Toolbar, sidebar, popovers, sheets | Automatic (zero code) |
| File list rows, diff, terminal | **No glass** |

**Important**: `.glass` and `.glassProminent` are different types (`GlassButtonStyle` vs `GlassProminentButtonStyle`). You cannot use them in a ternary -- use if/else:

```swift
// WRONG -- won't compile
.buttonStyle(isActive ? .glassProminent : .glass)

// CORRECT
if isActive {
  Button(...).buttonStyle(.glassProminent)
} else {
  Button(...).buttonStyle(.glass)
}
```

### Error Handling

- FFI calls from views: `try? repoService?.method()` (silent, fire-and-forget)
- FFI calls from AppState: `do { } catch { errorMessage = error.localizedDescription }` (show alert)
- Never retry failed operations in a loop

### Async / Concurrency

- `@MainActor` is the default actor isolation (build setting)
- EventBridge methods are `nonisolated` and dispatch to MainActor via `Task { @MainActor in }`
- Use `[weak self]` in EventBridge closures to avoid retain cycles with AppState
- Rust FFI functions are synchronous (blocking) -- wrap in `Task { }` when called from async contexts

## Generated Files -- Do Not Edit

```
DeathPush/Bridge/Generated/DeathPushCore.swift   # UniFFI Swift wrappers
DeathPush/Bridge/Generated/deathpush_core.swift   # UniFFI lower-level FFI
DeathPush/Bridge/FFI/DeathPushFFI.h               # C header for linker
DeathPush/Bridge/FFI/deathpush_coreFFI.h          # C header for linker
DeathPush/Bridge/FFI/deathpush_coreFFI.modulemap  # Module map
```

Regenerated automatically by `scripts/generate-bindings.sh` on every build. Changes will be overwritten.

## Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| SwiftTerm | 1.11.2+ | Integrated terminal (LocalProcessTerminalView) |
| Sparkle | 2.9.0+ | Auto-updates outside App Store |
| libdeathpush_ffi.a | (local) | Rust backend static library |

## Theming

The VS Code JSON theme (colors, token rules) applies **only** to Monaco editors and the terminal. The native macOS UI (sidebar, toolbar, headers, status bar) uses system appearance via `preferredColorScheme` -- never apply `editor.background` or other theme colors to native chrome. Background color overrides should be scoped to the Monaco `WKWebView` containers only.

## Monaco Editor Integration

The diff viewer uses Monaco via WKWebView (`MonacoDiffView.swift`).

- **Bundle location**: `Resources/Monaco/` (outside Xcode auto-sync group)
- **Entry point**: `index.html` loads `vs/loader.js` + `diff-bridge.js`
- **JS -> Swift**: `window.webkit.messageHandlers.diffBridge.postMessage()`
- **Swift -> JS**: `webView.evaluateJavaScript("setDiffContent(...)")`
- **Themes**: 30+ VS Code JSON themes in `Resources/Monaco/themes/`

When updating Monaco, run `./scripts/bundle-monaco.sh` from the repo root.

## Terminal Integration

Uses SwiftTerm's `LocalProcessTerminalView` (NSViewRepresentable).

- Shell process spawned via `startProcess(executable:args:environment:execName:currentDirectory:)`
- Environment from `Terminal.getEnvironmentVariables(termName:)` returns `[String]` (array of `KEY=VALUE`), not a dictionary
- Tab management is view-local (`@State private var sessions: [TerminalSession]`)
- Glass tab bar with `GlassEffectContainer` for session switching

## Deep Linking & CLI

- **URL scheme**: `deathpush://` registered in `Info.plist`
- **Handler**: `.onOpenURL` in `DeathPushApp.swift` parses path and calls `appState.openRepository()`
- **CLI tool**: `scripts/dp` -- opens repo via `open "deathpush://$TARGET"`
- **In-app install**: File > Install Command Line Tool (creates `/usr/local/bin/dp` and `/usr/local/bin/deathpush` symlinks via osascript admin privilege escalation)

## Distribution

- **Signing**: Developer ID (outside App Store), hardened runtime, no sandbox
- **Entitlements**: `com.apple.security.cs.allow-unsigned-executable-memory` (required for Rust FFI)
- **DMG**: `scripts/create-dmg.sh` (requires `brew install create-dmg`)
- **Notarization**: `scripts/notarize.sh` (requires stored keychain credentials via `notarytool store-credentials`)
- **Auto-update**: Sparkle with appcast.xml hosted on GitHub Releases

## View Hierarchy

```
DeathPushApp
├── WindowGroup
│   └── ContentView
│       ├── WelcomeScreenView (no repo open)
│       │   └── CloneSheetView
│       └── RepositoryView (repo open)
│           ├── NavigationSplitView
│           │   ├── SidebarView (glass sidebar)
│           │   │   ├── SCMView (changes tab)
│           │   │   │   ├── CommitInputView
│           │   │   │   ├── ResourceItemView (per file)
│           │   │   │   └── Stash section
│           │   │   └── (history/explorer tabs route to detail)
│           │   └── VSplitView (detail)
│           │       ├── DiffDetailView / HistoryView / EmptyStateView
│           │       │   ├── DiffHeaderView
│           │       │   └── MonacoDiffView (WKWebView)
│           │       └── TerminalPanelView (collapsible, Cmd+J)
│           ├── DeathPushToolbar (glass toolbar)
│           ├── StatusBarView (safeAreaInset bottom)
│           ├── MergeBannerView (safeAreaInset top, conditional)
│           └── QuickOpenView (sheet, Cmd+P)
└── Settings
    └── SettingsView (TabView: Editor, Git)
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Cmd+O | Open Repository |
| Cmd+P | Quick Open |
| Cmd+1/2/3 | Changes / History / Explorer |
| Cmd+J | Toggle Terminal |
| Cmd+Return | Commit |
| Cmd+Shift+F | Fetch |
| Cmd+Shift+P | Pull |
| Cmd+Shift+U | Push |
| Cmd+Shift+A | Stage All |

## Adding a New View

1. Create `Views/<Feature>/<Name>View.swift`
2. Access state via `@Environment(AppState.self)` or `@Environment(RepositoryService.self)`
3. Use glass styles for navigation controls, plain styles for content
4. Wire into the view hierarchy (usually in `RepositoryView` or a sidebar tab)
5. Build passes automatically -- Xcode file-system-sync picks up new files

## Adding a New FFI Command

1. Add the function in `crates/deathpush-core/src/services.rs`
2. Export it in `crates/deathpush-ffi/src/lib.rs` with `#[uniffi::export]`
3. Build (`cargo build -p deathpush-ffi`) to regenerate bindings
4. Add a wrapper method on `RepositoryService` that calls the generated function
5. All FFI functions take `sessionId: String` as first parameter
