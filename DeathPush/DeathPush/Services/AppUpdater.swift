import Foundation
import Sparkle

/// Auto-update manager using Sparkle framework.
/// EdDSA key is configured in Info.plist (SUPublicEDKey).
/// Appcast feed URL: SUFeedURL in Info.plist.
/// Sign releases locally with: scripts/sparkle-sign.sh build/DeathPush.dmg
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
