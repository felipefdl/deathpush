import SwiftUI

struct LicenseEntry: Identifiable {
	let id = UUID()
	let name: String
	let license: String
	let url: String
}

enum LicenseCategory: String, CaseIterable {
	case swift = "Swift Dependencies"
	case rust = "Rust Dependencies"
	case javascript = "JavaScript Assets"
}

private let licenseEntries: [LicenseCategory: [LicenseEntry]] = [
	.swift: [
		LicenseEntry(name: "SwiftTerm", license: "MIT", url: "https://github.com/migueldeicaza/SwiftTerm"),
		LicenseEntry(name: "Sparkle", license: "MIT", url: "https://github.com/sparkle-project/Sparkle"),
	],
	.rust: [
		LicenseEntry(name: "git2", license: "MIT / Apache 2.0", url: "https://github.com/rust-lang/git2-rs"),
		LicenseEntry(name: "tokio", license: "MIT", url: "https://github.com/tokio-rs/tokio"),
		LicenseEntry(name: "serde", license: "MIT / Apache 2.0", url: "https://github.com/serde-rs/serde"),
		LicenseEntry(name: "uniffi", license: "MPL 2.0", url: "https://github.com/mozilla/uniffi-rs"),
		LicenseEntry(name: "nucleo-matcher", license: "MPL 2.0", url: "https://github.com/helix-editor/nucleo"),
		LicenseEntry(name: "trash", license: "MIT", url: "https://github.com/Byron/trash-rs"),
		LicenseEntry(name: "chrono", license: "MIT / Apache 2.0", url: "https://github.com/chronotope/chrono"),
		LicenseEntry(name: "tracing", license: "MIT", url: "https://github.com/tokio-rs/tracing"),
		LicenseEntry(name: "notify", license: "Artistic 2.0 / MIT", url: "https://github.com/notify-rs/notify"),
	],
	.javascript: [
		LicenseEntry(name: "Monaco Editor", license: "MIT", url: "https://github.com/microsoft/monaco-editor"),
	],
]

struct LicensesView: View {
	@Environment(\.dismiss) private var dismiss

	var body: some View {
		VStack(spacing: 0) {
			Text("Open Source Licenses")
				.font(.headline)
				.padding(.top, 16)
				.padding(.bottom, 8)

			Text("DeathPush is built with these open source projects.")
				.font(.subheadline)
				.foregroundStyle(.secondary)
				.padding(.bottom, 12)

			List {
				ForEach(LicenseCategory.allCases, id: \.self) { category in
					Section(category.rawValue) {
						if let entries = licenseEntries[category] {
							ForEach(entries) { entry in
								LicenseRow(entry: entry)
							}
						}
					}
				}
			}
			.listStyle(.inset(alternatesRowBackgrounds: true))

			HStack {
				Spacer()
				Button("Done") {
					dismiss()
				}
				.keyboardShortcut(.defaultAction)
			}
			.padding(16)
		}
		.frame(width: 500, height: 450)
	}
}

private struct LicenseRow: View {
	let entry: LicenseEntry

	var body: some View {
		HStack {
			Text(entry.name)
				.fontWeight(.medium)

			Spacer()

			Text(entry.license)
				.font(.caption)
				.padding(.horizontal, 8)
				.padding(.vertical, 3)
				.glassEffect(.regular)

			Button {
				if let url = URL(string: entry.url) {
					NSWorkspace.shared.open(url)
				}
			} label: {
				Image(systemName: "arrow.up.right.square")
			}
			.buttonStyle(.glass)
			.help(entry.url)
		}
	}
}
