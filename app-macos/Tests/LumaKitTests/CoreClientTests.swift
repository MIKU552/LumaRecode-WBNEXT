import XCTest

@testable import LumaKit

final class CoreClientTests: XCTestCase {
    private let client = CoreClient()

    private var tenBitHlg: SourceDescription {
        var source = SourceDescription(pixelFormat: "yuv420p10le", kind: "hlg", components: [10, 10, 10])
        source.chromaWidth = 1
        source.chromaHeight = 1
        return source
    }

    func testIdentityMatchesTheCompiledAbi() throws {
        let identity = try client.identity()
        XCTAssertEqual(identity.abi, CoreClient.abiVersion)
        XCTAssertEqual(identity.version, CoreClient.version)
        XCTAssertTrue(identity.operations.contains("policy.validate"))
    }

    func testDescribeReportsSourceFactsAndChoices() throws {
        let described = try client.describe(source: tenBitHlg, options: OutputOptions())
        XCTAssertEqual(described.source.sampling, "420")
        XCTAssertEqual(described.source.depth, 10)
        XCTAssertEqual(described.source.range, "limited")
        XCTAssertEqual(described.choices.samplings, ["420"])
        XCTAssertEqual(described.choices.depths, [10])
        XCTAssertFalse(described.audio.flac)
    }

    func testArchiveKeepsTenBitAndShareDeliversEight() throws {
        var archive = OutputOptions()
        archive.purpose = "archive"
        let archived = try client.resolve(source: tenBitHlg, options: archive)
        XCTAssertEqual(archived.bitDepth, 10)
        XCTAssertEqual(archived.pixelFormat, "yuv420p10le")
        XCTAssertEqual(archived.container, "mp4")

        let shared = try client.resolve(source: tenBitHlg, options: OutputOptions())
        XCTAssertEqual(shared.bitDepth, 8)
        XCTAssertEqual(shared.suffix, ".mp4")
    }

    func testRejectionsCarryStableCodes() {
        let sdr = SourceDescription(pixelFormat: "yuv420p", kind: "sdr", components: [8, 8, 8])
        var options = OutputOptions()
        options.output = "hdr"
        XCTAssertThrowsError(try client.validate(source: sdr, options: options)) { error in
            XCTAssertEqual((error as? CoreError)?.code, "policy.hdr_depth")
        }

        var log = OutputOptions()
        log.transform = "log-hdr"
        log.output = "hdr"
        log.profile = "pocket3"
        XCTAssertThrowsError(try client.validate(source: tenBitHlg, options: log)) { error in
            XCTAssertEqual((error as? CoreError)?.code, "transform.requires_log")
        }
    }

    func testFlacIsWithheldWithoutNamedTracks() throws {
        let sdr = SourceDescription(pixelFormat: "yuv420p", kind: "sdr", components: [8, 8, 8])
        let described = try client.describe(source: sdr, options: OutputOptions())
        XCTAssertFalse(described.audio.flac)
    }
}
