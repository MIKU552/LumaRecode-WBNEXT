import Foundation

/// Locates ffprobe and reads one document from it.
///
/// The application never bundles FFmpeg: the user supplies it, and the path can be
/// overridden with `LUMARECODE_FFPROBE`. Probing only reads; nothing is written.
enum ProbeRunner {
    /// Error surfaced to the interface when probing cannot proceed.
    enum Failure: LocalizedError {
        case ffprobeMissing
        case failed(String)

        var errorDescription: String? {
            switch self {
            case .ffprobeMissing:
                "ffprobe was not found. Install FFmpeg or set LUMARECODE_FFPROBE."
            case .failed(let message):
                message.isEmpty ? "ffprobe failed." : message
            }
        }
    }

    /// The executable to use, if one exists.
    static func locate() -> URL? {
        if let configured = ProcessInfo.processInfo.environment["LUMARECODE_FFPROBE"],
           !configured.isEmpty,
           FileManager.default.isExecutableFile(atPath: configured) {
            return URL(fileURLWithPath: configured)
        }

        let candidates = [
            "/opt/homebrew/bin/ffprobe",
            "/usr/local/bin/ffprobe",
            "/usr/bin/ffprobe"
        ]
        return candidates
            .first { FileManager.default.isExecutableFile(atPath: $0) }
            .map { URL(fileURLWithPath: $0) }
    }

    /// Runs ffprobe and returns its JSON document.
    static func document(path: String) throws -> String {
        guard let tool = locate() else {
            throw Failure.ffprobeMissing
        }

        let process = Process()
        process.executableURL = tool
        process.arguments = ["-v", "error", "-print_format", "json", "-show_streams", "-show_format", path]
        let output = Pipe()
        let errors = Pipe()
        process.standardOutput = output
        process.standardError = errors

        try process.run()
        let data = output.fileHandleForReading.readDataToEndOfFile()
        let errorData = errors.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            let message = String(data: errorData, encoding: .utf8) ?? ""
            throw Failure.failed(message.trimmingCharacters(in: .whitespacesAndNewlines))
        }
        guard let text = String(data: data, encoding: .utf8) else {
            throw Failure.failed("ffprobe returned text that is not UTF-8.")
        }
        return text
    }
}
