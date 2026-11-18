import Foundation

/// Takes the video back out of a motion photo.
///
/// The bytes are already in the file the camera wrote; this copies the declared range and
/// checks that it matches before and after, so a truncated or moved file fails instead of
/// producing a short video nobody notices.
public enum MotionPhotoExtractor {
    /// Copies the embedded video to `destination`; returns how many bytes were written.
    @discardableResult
    public static func extract(from fileURL: URL, to destination: URL) throws -> Int {
        let info = try MotionPhotoInspector.inspect(fileURL: fileURL)
        let handle = try FileHandle(forReadingFrom: fileURL)
        defer { try? handle.close() }
        try handle.seek(toOffset: UInt64(info.videoOffset))

        FileManager.default.createFile(atPath: destination.path, contents: nil)
        let output = try FileHandle(forWritingTo: destination)
        defer { try? output.close() }

        var remaining = info.videoLength
        while remaining > 0 {
            let chunk = try handle.read(upToCount: min(remaining, 1 << 20)) ?? Data()
            guard !chunk.isEmpty else {
                throw MotionPhotoError.inconsistentLayout
            }
            try output.write(contentsOf: chunk)
            remaining -= chunk.count
        }
        return info.videoLength
    }
}
