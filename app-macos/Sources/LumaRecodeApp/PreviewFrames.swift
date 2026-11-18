import AppKit
import AVFoundation

/// Pulls a still out of the opened file with AVFoundation.
///
/// The generator is created per request: a preview is a single frame on demand, and keeping
/// an asset alive across seeks pins the file handle for as long as the window is open.
enum PreviewFrames {
    /// Generates the frame shown at `seconds`, clamped to the asset.
    ///
    /// Called off the main actor; the caller shows the image where the UI expects one.
    static func image(path: String, seconds: Double) async throws -> NSImage {
        let asset = AVURLAsset(url: URL(fileURLWithPath: path))
        let generator = AVAssetImageGenerator(asset: asset)
        generator.appliesPreferredTrackTransform = true
        generator.requestedTimeToleranceBefore = .zero
        generator.requestedTimeToleranceAfter = .zero
        let time = CMTime(seconds: max(seconds, 0), preferredTimescale: 600)
        let (image, _) = try await generator.image(at: time)
        return NSImage(cgImage: image, size: .zero)
    }
}
