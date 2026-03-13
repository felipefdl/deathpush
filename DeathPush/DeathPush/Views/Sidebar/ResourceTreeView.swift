import SwiftUI

// MARK: - Tree Node

struct SCMTreeNode: Identifiable {
	let id: String
	let name: String
	var folders: [SCMTreeNode]
	var files: [FileEntry]
}

func buildSCMTree(from files: [FileEntry]) -> [SCMTreeNode] {
	let root = SCMTreeBuilder(name: "", fullPath: "")

	for file in files {
		let parts = file.path.split(separator: "/").map(String.init)
		var current = root

		for i in 0..<(parts.count - 1) {
			let dirName = parts[i]
			if let existing = current.children[dirName] {
				current = existing
			} else {
				let fullPath = parts[0...i].joined(separator: "/")
				let node = SCMTreeBuilder(name: dirName, fullPath: fullPath)
				current.children[dirName] = node
				current.childOrder.append(dirName)
				current = node
			}
		}

		current.files.append(file)
	}

	return root.toNodes()
}

private class SCMTreeBuilder {
	let name: String
	let fullPath: String
	var children: [String: SCMTreeBuilder] = [:]
	var childOrder: [String] = []
	var files: [FileEntry] = []

	init(name: String, fullPath: String) {
		self.name = name
		self.fullPath = fullPath
	}

	func toNodes() -> [SCMTreeNode] {
		var nodes: [SCMTreeNode] = []

		for childName in childOrder.sorted() {
			guard let child = children[childName] else { continue }
			nodes.append(SCMTreeNode(
				id: child.fullPath,
				name: child.name,
				folders: child.toNodes(),
				files: child.files.sorted {
					let a = String($0.path.split(separator: "/").last ?? "")
					let b = String($1.path.split(separator: "/").last ?? "")
					return a.localizedCaseInsensitiveCompare(b) == .orderedAscending
				}
			))
		}

		return nodes
	}
}

// MARK: - Flat Item

struct FlatSCMItem: Identifiable {
	enum Kind {
		case folder(SCMTreeNode)
		case file(FileEntry)
	}

	let kind: Kind
	let depth: Int

	var id: String {
		switch kind {
		case .folder(let node): "folder:\(node.id)"
		case .file(let file): "file:\(file.path)"
		}
	}
}

private func flattenSCMTree(folders: [SCMTreeNode], files: [FileEntry], depth: Int, expanded: Set<String>) -> [FlatSCMItem] {
	var items: [FlatSCMItem] = []

	for folder in folders {
		items.append(FlatSCMItem(kind: .folder(folder), depth: depth))
		if expanded.contains(folder.id) {
			items.append(contentsOf: flattenSCMTree(
				folders: folder.folders,
				files: folder.files,
				depth: depth + 1,
				expanded: expanded
			))
		}
	}

	for file in files {
		items.append(FlatSCMItem(kind: .file(file), depth: depth))
	}

	return items
}

// MARK: - Tree View

struct ResourceTreeView: View {
	let files: [FileEntry]
	let groupKind: ResourceGroupKind
	@Binding var selectedFilePath: String?
	let contextMenuBuilder: (FileEntry) -> AnyView
	@State private var expandedFolders: Set<String> = []

	var body: some View {
		let tree = buildSCMTree(from: files)
		let flatItems = flattenSCMTree(folders: tree, files: [], depth: 0, expanded: expandedFolders)

		ForEach(flatItems) { item in
			switch item.kind {
			case .folder(let node):
				SCMTreeFolderRow(
					node: node,
					depth: item.depth,
					isExpanded: expandedFolders.contains(node.id),
					onToggle: { toggleFolder(node.id) }
				)
			case .file(let file):
				ResourceItemView(
					file: file,
					depth: item.depth,
					isSelected: selectedFilePath == file.path,
					showDirectoryPath: false,
					onSelect: { selectedFilePath = file.path }
				)
				.tag(file.path)
				.contextMenu {
					contextMenuBuilder(file)
				}
			}
		}
		.onAppear {
			let tree = buildSCMTree(from: files)
			expandedFolders = Set(tree.map(\.id))
		}
	}

	private func toggleFolder(_ id: String) {
		if expandedFolders.contains(id) {
			expandedFolders.remove(id)
		} else {
			expandedFolders.insert(id)
		}
	}
}

// MARK: - Folder Row

struct SCMTreeFolderRow: View {
	let node: SCMTreeNode
	let depth: Int
	let isExpanded: Bool
	let onToggle: () -> Void

	var body: some View {
		Button(action: onToggle) {
			HStack(spacing: 4) {
				Image(systemName: "chevron.right")
					.font(.system(size: 10, weight: .semibold))
					.foregroundStyle(.tertiary)
					.rotationEffect(.degrees(isExpanded ? 90 : 0))
					.animation(.easeInOut(duration: 0.15), value: isExpanded)
					.frame(width: 14)

				FileIconView(fileName: node.name, isDirectory: true, isExpanded: isExpanded)

				Text(node.name)
					.font(.system(size: 13))
					.lineLimit(1)
					.truncationMode(.middle)

				Spacer()
			}
			.padding(.leading, CGFloat(depth) * 16)
			.contentShape(Rectangle())
		}
		.buttonStyle(.plain)
		.listRowSeparator(.hidden)
		.listRowInsets(EdgeInsets(top: 1, leading: 4, bottom: 1, trailing: 8))
	}
}
