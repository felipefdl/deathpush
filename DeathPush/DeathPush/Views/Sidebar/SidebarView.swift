import SwiftUI

struct SidebarView: View {
	@Environment(TabState.self) private var tabState

	@Namespace private var tabNamespace

	var body: some View {
		@Bindable var tab = tabState

		VStack(spacing: 0) {
			// Tab picker
			HStack(spacing: 0) {
				ForEach(SidebarItem.allCases, id: \.self) { item in
					Button {
						withAnimation(.easeInOut(duration: 0.2)) {
							tab.sidebarSelection = item
						}
					} label: {
						VStack(spacing: 4) {
							Text(item.rawValue)
								.textCase(.uppercase)
								.font(.caption.bold())
								.foregroundStyle(tab.sidebarSelection == item ? .primary : .secondary)
								.padding(.top, 8)

							if tab.sidebarSelection == item {
								Rectangle()
									.fill(Color.accentColor)
									.frame(height: 2)
									.matchedGeometryEffect(id: "underline", in: tabNamespace)
							} else {
								Color.clear
									.frame(height: 2)
							}
						}
					}
					.buttonStyle(.plain)
					.frame(maxWidth: .infinity)
				}
			}

			Divider()

			// Content
			switch tab.sidebarSelection {
			case .changes:
				SCMView(selectedFilePath: $tab.selectedFilePath)
			case .history:
				HistorySidebarView(selectedCommitId: $tab.selectedCommitId)
			case .explorer:
				ExplorerTreeView(selectedFilePath: $tab.explorerSelectedPath)
			}
		}
		.frame(minWidth: 290, idealWidth: 380)
	}
}

extension SidebarItem {
	var systemImage: String {
		switch self {
		case .changes: "arrow.triangle.branch"
		case .history: "clock.arrow.circlepath"
		case .explorer: "folder"
		}
	}
}
