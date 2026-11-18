import Foundation

/// Finds the FFmpeg the app should run.
///
/// `LUMARECODE_FFMPEG` wins so a user can point at a build with the filters the core asks
/// for; otherwise the three locations a macOS install normally uses are tried in order.
enum FFmpegLocator {
    static let candidates = [
        "/opt/homebrew/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/usr/bin/ffmpeg",
    ]

    /// The FFprobe that ships beside whichever FFmpeg the app runs.
    static func locateProbe() -> String? {
        guard let ffmpeg = locate() else {
            return nil
        }
        let probe = URL(fileURLWithPath: ffmpeg)
            .deletingLastPathComponent()
            .appendingPathComponent("ffprobe")
            .path
        return FileManager.default.isExecutableFile(atPath: probe) ? probe : nil
    }

    static func locate() -> String? {
        if let configured = ProcessInfo.processInfo.environment["LUMARECODE_FFMPEG"],
           FileManager.default.isExecutableFile(atPath: configured) {
            return configured
        }
        return candidates.first { FileManager.default.isExecutableFile(atPath: $0) }
    }
}
