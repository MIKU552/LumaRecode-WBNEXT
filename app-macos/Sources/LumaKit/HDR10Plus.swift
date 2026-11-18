import Foundation

/// Reads the HDR10+ metadata a clip carries and writes the document x265 reads back.
///
/// FFprobe already walks the stream and prints one line per frame, so the host asks it for
/// those lines and lets the core decide what the document says. Nothing here parses bits:
/// that lives in the core, where both hosts share it.
public enum HDR10PlusScanner {
    /// The line FFprobe prints for the metadata this reads.
    static let marker = "HDR Dynamic Metadata SMPTE2094-40"

    /// Runs FFprobe over `path` and returns the document plus how many frames it covered.
    public static func document(
        ffprobe: String,
        path: String,
        client: CoreClient = CoreClient()
    ) throws -> (frames: Int, document: Data) {
        let report = try client.run(
            commands: [[
                ffprobe, "-v", "error",
                "-select_streams", "v:0",
                "-show_frames",
                "-show_entries", "frame=side_data",
                "-of", "compact",
                path,
            ]],
            captureStdout: true
        )
        guard report.ok, let text = report.outcomes.first?.stdout else {
            let message = report.outcomes.first?.message ?? "ffprobe failed"
            throw ConversionError.failed(message.trimmingCharacters(in: .whitespacesAndNewlines))
        }

        let lines = text
            .split(separator: "\n", omittingEmptySubsequences: true)
            .map(String.init)
            .filter { $0.hasPrefix("frame") && $0.contains(marker) }
        guard !lines.isEmpty else {
            throw ConversionError.failed("The clip carries no HDR10+ metadata.")
        }

        let object = try client.hdr10PlusDocument(lines: lines)
        let data = try JSONSerialization.data(
            withJSONObject: object,
            options: [.sortedKeys, .withoutEscapingSlashes, .prettyPrinted]
        )
        return (lines.count, data)
    }
}
