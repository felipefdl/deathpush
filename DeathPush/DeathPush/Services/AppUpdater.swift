import Foundation
import Sparkle

/// Auto-update manager using Sparkle framework.
/// Sparkle must be added as an SPM dependency: https://github.com/sparkle-project/Sparkle
///
/// To complete setup:
/// 1. Add Sparkle package in Xcode (File > Add Package Dependencies)
/// 2. Generate an EdDSA key pair: ./bin/generate_keys (from Sparkle package)
/// 3. Set the public key in Info.plist under SUPublicEDKey
/// 4. Set SUFeedURL in Info.plist to the appcast.xml URL
/// 5. Sign releases with: ./bin/sign_update YourApp.dmg
final class AppUpdater {
	private let updaterController = SPUStandardUpdaterController(
		startingUpdater: true,
		updaterDelegate: nil,
		userDriverDelegate: nil
	)

	func checkForUpdates() {
		updaterController.checkForUpdates(nil)
	}

	var canCheckForUpdates: Bool {
		updaterController.updater.canCheckForUpdates
	}
}
