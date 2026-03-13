import Foundation

enum DateFormatters {
  static let iso8601WithFractional: ISO8601DateFormatter = {
    let f = ISO8601DateFormatter()
    f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    return f
  }()

  static let iso8601: ISO8601DateFormatter = {
    ISO8601DateFormatter()
  }()

  static let relative: RelativeDateTimeFormatter = {
    let f = RelativeDateTimeFormatter()
    f.unitsStyle = .abbreviated
    return f
  }()

  static func parseISO8601(_ dateStr: String) -> Date? {
    iso8601WithFractional.date(from: dateStr) ?? iso8601.date(from: dateStr)
  }

  static func relativeString(from dateStr: String) -> String {
    guard let date = parseISO8601(dateStr) else { return dateStr }
    return relative.localizedString(for: date, relativeTo: Date())
  }
}
