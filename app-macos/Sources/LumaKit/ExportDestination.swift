import Foundation

/// Where a conversion writes, and what it is called.
///
/// The naming rules live here rather than in the view so a typed name cannot escape the
/// chosen folder and a second run does not silently replace the first.
public enum ExportDestination {
    /// The default stem for a converted file: the source's own name plus a suffix.
    public static func defaultName(for source: URL) -> String {
        let stem = source.deletingPathExtension().lastPathComponent
        return stem.isEmpty ? "converted" : "\(stem)-converted"
    }

    /// Cleans a typed name: a path separator or a colon cannot be part of a file name.
    public static func sanitized(_ name: String) -> String {
        let cleaned = name
            .replacingOccurrences(of: "/", with: "-")
            .replacingOccurrences(of: ":", with: "-")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return cleaned.isEmpty ? "converted" : cleaned
    }

    /// The file a run would write.
    public static func url(directory: URL, name: String, fileExtension: String) -> URL {
        directory
            .appendingPathComponent(sanitized(name))
            .appendingPathExtension(fileExtension)
    }

    /// The first free name: `name.ext`, then `name 2.ext`, `name 3.ext`…
    public static func uniqueURL(
        directory: URL,
        name: String,
        fileExtension: String,
        exists: (URL) -> Bool
    ) -> URL {
        let base = url(directory: directory, name: name, fileExtension: fileExtension)
        if !exists(base) {
            return base
        }
        for index in 2...999 {
            let candidate = url(
                directory: directory,
                name: "\(sanitized(name)) \(index)",
                fileExtension: fileExtension
            )
            if !exists(candidate) {
                return candidate
            }
        }
        return base
    }
}
