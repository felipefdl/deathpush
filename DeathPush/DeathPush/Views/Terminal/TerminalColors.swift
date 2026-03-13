import SwiftTerm

/// Helper to install ANSI colors on a terminal view.
/// Isolated in its own file to avoid SwiftUI.Color / SwiftTerm.Color name clash.
func installTerminalANSIColors(_ components: [(r: UInt8, g: UInt8, b: UInt8)], on terminalView: LocalProcessTerminalView) {
  let colors: [Color] = components.map {
    Color(red: UInt16($0.r) * 257, green: UInt16($0.g) * 257, blue: UInt16($0.b) * 257)
  }
  terminalView.installColors(colors)
}
