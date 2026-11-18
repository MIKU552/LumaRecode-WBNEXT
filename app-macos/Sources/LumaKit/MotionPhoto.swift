import Foundation

/// What a motion photo declares about the video it carries.
public struct MotionPhotoInfo: Equatable, Sendable {
    /// Where the embedded video starts, from the beginning of the file.
    public let videoOffset: Int
    /// How many bytes the video occupies, to the end of the file.
    public let videoLength: Int
    /// Presentation timestamp the camera recorded, in microseconds.
    public let presentationTimestampUs: Int
    /// True when the file is an Ultra HDR image with a gain map next to the video.
    public let isUltraHDR: Bool
}

/// Why a file is not a motion photo.
public enum MotionPhotoError: Error, Equatable, LocalizedError {
    /// The JPEG segment chain does not end where it should.
    case malformedJPEG
    /// The XMP packet is missing or says nothing about a motion photo.
    case missingMetadata
    /// The metadata is present but contradicts the file layout.
    case inconsistentLayout

    public var errorDescription: String? {
        switch self {
        case .malformedJPEG:
            return "The JPEG segment chain is malformed."
        case .missingMetadata:
            return "The file carries no motion photo metadata."
        case .inconsistentLayout:
            return "The motion photo metadata does not match the file layout."
        }
    }
}

/// Reads the motion photo metadata a camera writes into a JPEG.
///
/// This stays in the host by design: the core never sees frame bytes, only the facts a
/// conversion needs. The rules below are the same ones the Windows host applies, so a file
/// is recognised the same way on both.
public enum MotionPhotoInspector {
    /// XMP packets sit in an APP1 segment behind this header.
    private static let xmpPrefix = Data("http://ns.adobe.com/xap/1.0/\0".utf8)

    /// Largest primary image the standard allows before the video.
    private static let offsetLimit = 268_435_456

    /// Inspects a file, reading only what the metadata needs.
    public static func inspect(fileURL: URL) throws -> MotionPhotoInfo {
        let handle = try FileHandle(forReadingFrom: fileURL)
        defer { try? handle.close() }
        let total = try handle.seekToEnd()
        try handle.seek(toOffset: 0)
        let prefix = try handle.read(upToCount: min(Int(total), 1_048_576)) ?? Data()

        let info = try inspect(prefix: prefix, totalLength: Int(total)) { offset in
            try handle.seek(toOffset: UInt64(offset))
            return try handle.read(upToCount: 12) ?? Data()
        }
        // The primary image must end exactly where the video starts; that check needs the
        // whole prefix, so it only runs when the prefix covers the offset.
        if info.videoOffset <= prefix.count {
            let primary = try primaryLength(prefix)
            let expected = info.isUltraHDR
                ? info.videoOffset - (try gainMapLength(prefix, from: primary, offset: info.videoOffset))
                : info.videoOffset
            guard primary == expected else {
                throw MotionPhotoError.inconsistentLayout
            }
        }
        return info
    }

    /// The metadata-only half, over bytes the caller already read.
    ///
    /// `prefix` is the head of the file, `totalLength` its size, and `videoHeader` the first
    /// twelve bytes at the video offset once it is known.
    static func inspect(
        prefix: Data,
        totalLength: Int,
        videoHeader: (Int) throws -> Data
    ) throws -> MotionPhotoInfo {
        guard let marker = prefix.range(of: xmpPrefix)?.lowerBound, marker >= 4 else {
            throw MotionPhotoError.missingMetadata
        }
        let size = Int(prefix[marker - 2]) << 8 | Int(prefix[marker - 1])
        guard size > xmpPrefix.count, marker - 2 + size <= prefix.count else {
            throw MotionPhotoError.missingMetadata
        }
        let xml = prefix.subdata(in: (marker + xmpPrefix.count)..<(marker - 2 + size))
        let packet = try XMPPacket(xml)

        guard packet.motionPhoto == "1", packet.version == "1" else {
            throw MotionPhotoError.missingMetadata
        }
        guard let raw = packet.timestamp, let timestamp = Int(raw), timestamp >= 0 else {
            throw MotionPhotoError.missingMetadata
        }
        let items = packet.items
        let ultra = items.count == 3
        let expected = ultra ? ["Primary", "GainMap", "MotionPhoto"] : ["Primary", "MotionPhoto"]
        guard items.count == expected.count,
              items.map({ $0["Semantic"] ?? "" }) == expected,
              items.first?["Mime"] == "image/jpeg",
              items.last?["Mime"] == "video/mp4",
              let length = items.last?["Length"].flatMap(Int.init),
              length > 0, length < totalLength
        else {
            throw MotionPhotoError.inconsistentLayout
        }
        if ultra, items[1]["Mime"] != "image/jpeg" {
            throw MotionPhotoError.inconsistentLayout
        }

        let offset = totalLength - length
        guard offset <= offsetLimit else {
            throw MotionPhotoError.inconsistentLayout
        }
        let header = try videoHeader(offset)
        guard header.count >= 8, header.subdata(in: 4..<8) == Data("ftyp".utf8) else {
            throw MotionPhotoError.inconsistentLayout
        }
        return MotionPhotoInfo(
            videoOffset: offset,
            videoLength: length,
            presentationTimestampUs: timestamp,
            isUltraHDR: ultra
        )
    }

    /// Length of the primary image: its segments up to and including the end marker.
    static func primaryLength(_ data: Data) throws -> Int {
        var index = 2
        while index + 4 <= data.count {
            guard data[index] == 0xFF else {
                throw MotionPhotoError.malformedJPEG
            }
            let marker = data[index + 1]
            if marker == 0xD9 {
                return index + 2
            }
            if marker == 0x01 || (0xD0...0xD7).contains(marker) {
                index += 2
                continue
            }
            let length = Int(data[index + 2]) << 8 | Int(data[index + 3])
            guard length >= 2 else {
                throw MotionPhotoError.malformedJPEG
            }
            index += 2 + length
            if marker == 0xDA {
                // Entropy-coded data: walk to the end marker, byte stuffing and restarts pass.
                while index + 1 < data.count {
                    if data[index] == 0xFF, data[index + 1] == 0xD9 {
                        return index + 2
                    }
                    index += 1
                }
                throw MotionPhotoError.malformedJPEG
            }
        }
        throw MotionPhotoError.malformedJPEG
    }

    /// Length of the gain map that follows the primary image in an Ultra HDR file.
    private static func gainMapLength(_ data: Data, from primary: Int, offset: Int) throws -> Int {
        guard primary < offset else {
            throw MotionPhotoError.inconsistentLayout
        }
        let gain = try primaryLength(data.subdata(in: primary..<offset))
        guard primary + gain == offset else {
            throw MotionPhotoError.inconsistentLayout
        }
        return gain
    }
}

/// The attributes and item list a motion photo writes into its XMP packet.
private struct XMPPacket {
    let attributes: [String: String]
    let items: [[String: String]]

    init(_ xml: Data) throws {
        let delegate = Delegate()
        let parser = XMLParser(data: xml)
        parser.delegate = delegate
        guard parser.parse() else {
            throw MotionPhotoError.missingMetadata
        }
        attributes = delegate.descriptionAttributes
        items = delegate.items
    }

    var motionPhoto: String? { attribute("MotionPhoto") }
    var version: String? { attribute("MotionPhotoVersion") }
    var timestamp: String? { attribute("MotionPhotoPresentationTimestampUs") }

    /// Attributes are qualified (`GCamera:MotionPhoto`); the prefix carries no meaning here.
    private func attribute(_ local: String) -> String? {
        attributes.first { $0.key.split(separator: ":").last.map(String.init) == local }?.value
    }

    private final class Delegate: NSObject, XMLParserDelegate {
        var descriptionAttributes: [String: String] = [:]
        var items: [[String: String]] = []

        func parser(
            _ parser: XMLParser,
            didStartElement elementName: String,
            namespaceURI: String?,
            qualifiedName qName: String?,
            attributes: [String: String] = [:]
        ) {
            let local = elementName.split(separator: ":").last.map(String.init) ?? elementName
            if local == "Description" {
                descriptionAttributes.merge(attributes) { current, _ in current }
            } else if local == "Item" {
                // Item attributes are qualified too; the callers read the local names.
                items.append(attributes.reduce(into: [:]) { result, pair in
                    let key = pair.key.split(separator: ":").last.map(String.init) ?? pair.key
                    result[key] = pair.value
                })
            }
        }
    }
}
