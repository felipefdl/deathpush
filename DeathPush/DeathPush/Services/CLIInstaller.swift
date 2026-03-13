import AppKit

enum CLIInstaller {
  private static let installDir = "/usr/local/bin"
  private static let cliScript = """
    #!/bin/bash
    # DeathPush CLI launcher
    set -euo pipefail
    TARGET="${1:-.}"
    if [[ "$TARGET" != /* ]]; then
      TARGET="$(cd "$TARGET" 2>/dev/null && pwd)" || {
        echo "Error: Directory '$1' does not exist" >&2
        exit 1
      }
    fi
    open "deathpush://$TARGET"
    """

  static func install() {
    let script = """
      do shell script "mkdir -p \(installDir) && \
      echo '\(cliScript.replacingOccurrences(of: "'", with: "'\\''"))' > \(installDir)/dp && \
      chmod +x \(installDir)/dp && \
      ln -sf \(installDir)/dp \(installDir)/deathpush" \
      with administrator privileges
      """

    var error: NSDictionary?
    if let appleScript = NSAppleScript(source: script) {
      appleScript.executeAndReturnError(&error)
      if let error {
        let message = error[NSAppleScript.errorMessage] as? String ?? "Unknown error"
        showAlert(title: "Installation Failed", message: message, style: .critical)
      } else {
        showAlert(
          title: "CLI Installed",
          message: "Commands 'dp' and 'deathpush' are now available in your terminal.\n\nUsage: dp [path]",
          style: .informational
        )
      }
    }
  }

  private static func showAlert(title: String, message: String, style: NSAlert.Style) {
    let alert = NSAlert()
    alert.messageText = title
    alert.informativeText = message
    alert.alertStyle = style
    alert.addButton(withTitle: "OK")
    alert.runModal()
  }
}
