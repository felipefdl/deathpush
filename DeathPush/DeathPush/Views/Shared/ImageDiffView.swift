import SwiftUI

struct ImageDiffView: View {
  let diff: DiffContent

  var body: some View {
    HStack(spacing: 0) {
      ImageDiffPanel(
        label: "ORIGINAL",
        dataURI: diff.original,
        emptyText: "New file"
      )

      Divider()

      ImageDiffPanel(
        label: "MODIFIED",
        dataURI: diff.modified,
        emptyText: "Deleted"
      )
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity)
  }
}

private struct ImageDiffPanel: View {
  let label: String
  let dataURI: String
  let emptyText: String

  var body: some View {
    VStack(spacing: 0) {
      Text(label)
        .font(.caption.bold())
        .foregroundStyle(.secondary)
        .frame(maxWidth: .infinity)
        .padding(.vertical, 6)
        .background(.quaternary)

      Divider()

      if dataURI.isEmpty {
        Text(emptyText)
          .font(.body.italic())
          .foregroundStyle(.tertiary)
          .frame(maxWidth: .infinity, maxHeight: .infinity)
      } else if let image = parseDataURI(dataURI) {
        let meta = imageMetadata(image: image, dataURI: dataURI)

        ScrollView([.horizontal, .vertical]) {
          ZStack {
            CheckerboardBackground()
            Image(nsImage: image)
              .resizable()
              .aspectRatio(contentMode: .fit)
          }
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .padding()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)

        Divider()

        Text(meta)
          .font(.caption)
          .foregroundStyle(.secondary)
          .frame(maxWidth: .infinity)
          .padding(.vertical, 4)
          .background(.quaternary)
      } else {
        ContentUnavailableView(
          "Cannot Display Image",
          systemImage: "photo",
          description: Text("The image format is not supported.")
        )
      }
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity)
  }

  private func parseDataURI(_ uri: String) -> NSImage? {
    guard uri.hasPrefix("data:"),
          let commaIndex = uri.firstIndex(of: ",") else {
      return nil
    }
    let base64String = String(uri[uri.index(after: commaIndex)...])
    guard let data = Data(base64Encoded: base64String) else { return nil }
    return NSImage(data: data)
  }

  private func imageMetadata(image: NSImage, dataURI: String) -> String {
    let width: Int
    let height: Int

    if let data = image.tiffRepresentation,
       let rep = NSBitmapImageRep(data: data) {
      width = rep.pixelsWide
      height = rep.pixelsHigh
    } else {
      width = Int(image.size.width)
      height = Int(image.size.height)
    }

    let sizeStr: String
    if let commaIndex = dataURI.firstIndex(of: ",") {
      let base64Part = dataURI[dataURI.index(after: commaIndex)...]
      let bytes = Int(floor(Double(base64Part.count) * 3.0 / 4.0))
      if bytes >= 1_048_576 {
        sizeStr = String(format: "%.1f MB", Double(bytes) / 1_048_576.0)
      } else if bytes >= 1024 {
        sizeStr = String(format: "%.1f KB", Double(bytes) / 1024.0)
      } else {
        sizeStr = "\(bytes) B"
      }
    } else {
      sizeStr = "unknown"
    }

    return "\(width) x \(height) - \(sizeStr)"
  }
}

private struct CheckerboardBackground: View {
  private let cellSize: CGFloat = 8
  private let lightColor = Color(white: 0.85)
  private let darkColor = Color(white: 0.65)

  var body: some View {
    Canvas { context, size in
      let cols = Int(ceil(size.width / cellSize))
      let rows = Int(ceil(size.height / cellSize))
      for row in 0..<rows {
        for col in 0..<cols {
          let color = (row + col).isMultiple(of: 2) ? lightColor : darkColor
          let rect = CGRect(
            x: CGFloat(col) * cellSize,
            y: CGFloat(row) * cellSize,
            width: cellSize,
            height: cellSize
          )
          context.fill(Path(rect), with: .color(color))
        }
      }
    }
  }
}
