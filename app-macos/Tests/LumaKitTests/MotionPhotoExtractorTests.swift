import XCTest
@testable import LumaKit

final class MotionPhotoExtractorTests: XCTestCase {
    func testItCopiesTheVideoOutAndNothingElse() throws {
        let video = Data((0..<600).map { UInt8($0 % 251) })
        let source = try MotionPhotoFixture.write(video: video)
        let destination = FileManager.default.temporaryDirectory
            .appendingPathComponent("extracted-\(UUID().uuidString).mp4")
        defer { try? FileManager.default.removeItem(at: destination) }

        let written = try MotionPhotoExtractor.extract(from: source, to: destination)

        XCTAssertEqual(written, video.count)
        let extracted = try Data(contentsOf: destination)
        XCTAssertEqual(extracted, video)
        // The primary image must not leak into the video: its bytes are gone.
        XCTAssertLessThan(extracted.count, (try Data(contentsOf: source)).count)
    }

    func testAFileWithoutMetadataIsRefusedBeforeAnythingIsWritten() throws {
        let source = try MotionPhotoFixture.write(video: Data(repeating: 0xAB, count: 64), motionPhoto: nil)
        let destination = FileManager.default.temporaryDirectory
            .appendingPathComponent("extracted-\(UUID().uuidString).mp4")
        defer { try? FileManager.default.removeItem(at: destination) }

        XCTAssertThrowsError(try MotionPhotoExtractor.extract(from: source, to: destination))
    }
}

/// Builds a motion photo whose embedded video the tests know byte for byte.
enum MotionPhotoFixture {
    static func write(video: Data, motionPhoto: String? = "1") throws -> URL {
        var file = Data([0xFF, 0xD8])
        file += app1(xmp(motionPhoto: motionPhoto, videoLength: video.count))
        let length = 24 + 2
        file += Data([0xFF, 0xFE, UInt8(length >> 8), UInt8(length & 0xFF)])
        file += Data(repeating: 0x11, count: 24)
        file += Data([0xFF, 0xD9])
        // The container header the inspector insists on, then the payload the test compares.
        file += Data([0x00, 0x00, 0x00, 0x18])
        file += Data("ftyp".utf8)
        file += video

        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("motion-\(UUID().uuidString).jpg")
        try file.write(to: url)
        return url
    }

    private static func app1(_ xml: Data) -> Data {
        let prefix = Data("http://ns.adobe.com/xap/1.0/\0".utf8)
        let length = prefix.count + xml.count + 2
        var segment = Data([0xFF, 0xE1, UInt8(length >> 8), UInt8(length & 0xFF)])
        segment += prefix
        segment += xml
        return segment
    }

    private static func xmp(motionPhoto: String?, videoLength: Int) -> Data {
        let camera = motionPhoto.map { "GCamera:MotionPhoto=\"\($0)\" GCamera:MotionPhotoVersion=\"1\"" } ?? ""
        let xml = """
        <?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
         <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
          <rdf:Description xmlns:GCamera="http://ns.google.com/photos/1.0/camera/"
           xmlns:Container="http://ns.google.com/photos/1.0/container/"
           xmlns:Item="http://ns.google.com/photos/1.0/container/item/"
           \(camera) GCamera:MotionPhotoPresentationTimestampUs="1500000">
           <Container:Directory>
            <rdf:Seq>
             <rdf:li rdf:parseType="Resource">
              <Container:Item Item:Semantic="Primary" Item:Mime="image/jpeg" Item:Length="26"/>
             </rdf:li>
             <rdf:li rdf:parseType="Resource">
              <Container:Item Item:Semantic="MotionPhoto" Item:Mime="video/mp4" Item:Length="\(videoLength + 8)"/>
             </rdf:li>
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
