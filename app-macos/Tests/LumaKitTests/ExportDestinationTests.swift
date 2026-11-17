import XCTest
@testable import LumaKit

final class ExportDestinationTests: XCTestCase {
    func testTheDefaultNameKeepsTheStem() {
        let source = URL(fileURLWithPath: "/Users/ana/Footage/clip.MOV")
        XCTAssertEqual(ExportDestination.defaultName(for: source), "clip-converted")
        XCTAssertEqual(ExportDestination.defaultName(for: URL(fileURLWithPath: "/")), "converted")
    }

    func testATypedNameCannotEscapeTheChosenFolder() {
        XCTAssertEqual(ExportDestination.sanitized("../x/y:z"), "..-x-y-z")
        XCTAssertEqual(ExportDestination.sanitized("   "), "converted")
    }

    func testTheTargetFollowsTheChosenContainer() {
        let directory = URL(fileURLWithPath: "/tmp/out")
        let url = ExportDestination.url(directory: directory, name: "clip", fileExtension: "mp4")
        XCTAssertEqual(url.path, "/tmp/out/clip.mp4")
    }

    func testACollisionGetsTheNextFreeNumber() {
        let directory = URL(fileURLWithPath: "/tmp/out")
        let taken: Set<String> = [
            "/tmp/out/clip-converted.mkv",
            "/tmp/out/clip-converted 2.mkv",
        ]
        let url = ExportDestination.uniqueURL(
            directory: directory,
            name: "clip-converted",
            fileExtension: "mkv"
        ) { taken.contains($0.path) }
        XCTAssertEqual(url.lastPathComponent, "clip-converted 3.mkv")
    }

    func testAFreeNameIsUsedAsTyped() {
        let directory = URL(fileURLWithPath: "/tmp/out")
        let url = ExportDestination.uniqueURL(
            directory: directory,
            name: "clip-converted",
            fileExtension: "mkv"
        ) { _ in false }
        XCTAssertEqual(url.lastPathComponent, "clip-converted.mkv")
    }
}
