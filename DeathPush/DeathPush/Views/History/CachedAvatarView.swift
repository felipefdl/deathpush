import SwiftUI

private let avatarCache = NSCache<NSString, NSImage>()

struct CachedAvatarView<Fallback: View>: View {
  let url: String
  let fallback: Fallback

  @State private var image: NSImage?

  var body: some View {
    Group {
      if let image {
        Image(nsImage: image)
          .resizable()
          .aspectRatio(contentMode: .fill)
      } else {
        fallback
      }
    }
    .task(id: url) {
      await loadImage()
    }
  }

  private func loadImage() async {
    let key = url as NSString
    if let cached = avatarCache.object(forKey: key) {
      image = cached
      return
    }
    guard let imageUrl = URL(string: url) else { return }
    do {
      let (data, _) = try await URLSession.shared.data(from: imageUrl)
      guard let nsImage = NSImage(data: data) else { return }
      avatarCache.setObject(nsImage, forKey: key)
      image = nsImage
    } catch {
      // Fallback view stays visible
    }
  }
}
