import XCTest
@testable import LumaKit

final class MotionPhotoInspectorTests: XCTestCase {
    func testItReadsTheVideoOffsetAndTimestamp() throws {
        let file = Fixture.make(videoLength: 400, timestamp: "1500000")
        let info = try MotionPhotoInspector.inspect(fileURL: try file.write())

        XCTAssertEqual(info.videoOffset, file.videoOffset)
        XCTAssertEqual(info.videoLength, 400)
        XCTAssertEqual(info.presentationTimestampUs, 1_500_000)
        XCTAssertFalse(info.isUltraHDR)
    }

    func testAnUltraHdrFileIsMarkedAndItsGainMapMustLine() throws {
        let file = Fixture.make(videoLength: 400, ultra: true)
        let info = try MotionPhotoInspector.inspect(fileURL: try file.write())

        XCTAssertTrue(info.isUltraHDR)
        XCTAssertEqual(info.videoOffset, file.videoOffset)
    }

    func testAFileWithoutTheMetadataIsRejected() throws {
        let file = Fixture.make(videoLength: 400, motionPhoto: nil)
        XCTAssertThrowsError(try MotionPhotoInspector.inspect(fileURL: try file.write())) { error in
            XCTAssertEqual(error as? MotionPhotoError, .missingMetadata)
        }
    }

    func testAVideoThatDoesNotStartWithAnMp4BoxIsRejected() throws {
        let file = Fixture.make(videoLength: 400, box: "moov")
        XCTAssertThrowsError(try MotionPhotoInspector.inspect(fileURL: try file.write())) { error in
            XCTAssertEqual(error as? MotionPhotoError, .inconsistentLayout)
        }
    }

    func testThePrimaryImageEndsAtItsEndMarker() {
        let primary = Fixture.jpeg(payload: 32)
        XCTAssertEqual(try? MotionPhotoInspector.primaryLength(primary), primary.count)
    }
}

/// Builds motion photo files whose layout the inspector can be checked against.
private enum Fixture {
    struct FixtureFile {
        let bytes: Data
        let videoOffset: Int

        /// Writes the fixture beside the other temporary files; they are a few hundred bytes.
        func write() throws -> URL {
            let url = FileManager.default.temporaryDirectory
                .appendingPathComponent("motion-\(UUID().uuidString).jpg")
            try bytes.write(to: url)
            return url
        }
    }

    /// A complete JPEG: start marker, one comment segment with the payload, end marker.
    static func jpeg(payload: Int) -> Data {
        var data = Data([0xFF, 0xD8])
        data += comment(payload)
        data += Data([0xFF, 0xD9])
        return data
    }

    /// One comment segment; a JPEG may hold several, and this keeps the markers honest.
    static func comment(_ payload: Int) -> Data {
        let length = payload + 2
        var data = Data([0xFF, 0xFE, UInt8(length >> 8), UInt8(length & 0xFF)])
        data += Data(repeating: 0x11, count: payload)
        return data
    }

    static func make(
        videoLength: Int,
        ultra: Bool = false,
        timestamp: String = "1000",
        motionPhoto: String? = "1",
        box: String = "ftyp"
    ) -> FixtureFile {
        // The primary image is one JPEG and holds the XMP packet; the gain map, when there
        // is one, is a second JPEG right after it.
        var file = Data([0xFF, 0xD8])
        file += app1(xmp(ultra: ultra, timestamp: timestamp, motionPhoto: motionPhoto))
        file += comment(24)
        file += Data([0xFF, 0xD9])
        if ultra {
            file += jpeg(payload: 16)
        }
        let videoOffset = file.count
        file += Data([0x00, 0x00, 0x00, 0x18])
        file += Data(box.utf8)
        file += Data(repeating: 0x22, count: max(videoLength - 8, 0))
        return FixtureFile(bytes: file, videoOffset: videoOffset)
    }

    private static func app1(_ xml: Data) -> Data {
        let prefix = Data("http://ns.adobe.com/xap/1.0/\0".utf8)
        let length = prefix.count + xml.count + 2
        var segment = Data([0xFF, 0xE1, UInt8(length >> 8), UInt8(length & 0xFF)])
        segment += prefix
        segment += xml
        return segment
    }

    private static func xmp(ultra: Bool, timestamp: String, motionPhoto: String?) -> Data {
        let camera = motionPhoto.map { "GCamera:MotionPhoto=\"\($0)\" GCamera:MotionPhotoVersion=\"1\"" } ?? ""
        let items: [(String, String, Int)] = ultra
            ? [("Primary", "image/jpeg", 24), ("GainMap", "image/jpeg", 16), ("MotionPhoto", "video/mp4", 0)]
            : [("Primary", "image/jpeg", 24), ("MotionPhoto", "video/mp4", 0)]
        let length = 400
        let list = items.enumerated().map { index, item in
            let size = item.0 == "MotionPhoto" ? length : item.2
            return """
                  <rdf:li rdf:parseType="Resource">
                   <Container:Item Item:Semantic="\(item.0)" Item:Mime="\(item.1)" Item:Length="\(size)"/>
                  </rdf:li>
            """
        }.joined(separator: "\n")
        let xml = """
        <?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
         <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
          <rdf:Description xmlns:GCamera="http://ns.google.com/photos/1.0/camera/"
           xmlns:Container="http://ns.google.com/photos/1.0/container/"
           xmlns:Item="http://ns.google.com/photos/1.0/container/item/"
           \(camera) GCamera:MotionPhotoPresentationTimestampUs="\(timestamp)">
           <Container:Directory>
            <rdf:Seq>
        \(list)
            </rdf:Seq>
           </Container:Directory>
          </rdf:Description>
         </rdf:RDF>
        </x:xmpmeta>
        <?xpacket end="w"?>
        """
        return Data(xml.utf8)
    }
}
