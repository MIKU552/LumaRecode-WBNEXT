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
        // A 10-bit source may be shared at 8 or 10 bits; HDR would filter to 10.
        XCTAssertEqual(described.choices.depths, [8, 10])
        XCTAssertFalse(described.audio.flac)
    }

    func testArchiveKeepsTenBitAndShareDeliversEight() throws {
        var archive = OutputOptions()
        archive.purpose = "archive"
        let archived = try client.resolve(source: tenBitHlg, options: archive)
        XCTAssertEqual(archived.bitDepth, 10)
        XCTAssertEqual(archived.pixelFormat, "yuv420p10le")
        // Archive defaults to MKV so the original audio tracks survive intact.
        XCTAssertEqual(archived.container, "mkv")
        XCTAssertEqual(archived.suffix, ".mkv")

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

    // The registry refuses a partial table, so this fixture carries every selector the
    // shipped document has. A real conversion reads the shipped file instead.
    private let presetTable = """
    {
      "share": {
        "avc": {"encoder": "h264_qsv", "preset": "4", "quality": 22},
        "hevc": {"encoder": "hevc_qsv", "preset": "4", "quality": 24},
        "av1": {"encoder": "av1_qsv", "preset": "4", "quality": 26}
      },
      "archive": {
        "avc": {"encoder": "libx264", "preset": "medium", "quality": 21},
        "hevc": {"encoder": "libx265", "preset": "medium", "quality": 23,
                 "encoder_parameters": {"rdoq-level": 2}},
        "av1": {"encoder": "libsvtav1", "preset": "4", "quality": 32}
      },
      "archive_fast_av1": {"encoder": "libsvtav1", "preset": "6", "quality": 32},
      "audio": {
        "archive_default": "copy",
        "share_default": {"avc": "aac", "hevc": "aac", "av1": "opus"}
      }
    }
    """

    func testPlannerBuildsAnArchiveCommandFromCoreDecisions() throws {
        let source: [String: Any] = [
            "pixel_format": "yuv420p",
            "kind": "sdr",
            "components": [8, 8, 8],
            "log2_chroma_w": 1,
            "log2_chroma_h": 1,
        ]
        let job = ConversionJob(input: "/tmp/in.mp4", output: "/tmp/out.mkv", presets: presetTable)
        let plan = try ConversionPlanner(ffmpeg: "/opt/homebrew/bin/ffmpeg")
            .plan(job, probed: ProbeResult(source: source, videoIndex: 0, frameRate: 30))

        XCTAssertEqual(plan.arguments.first, "/opt/homebrew/bin/ffmpeg")
        XCTAssertTrue(plan.arguments.contains("libx265"))
        // Eight seconds at 30 fps, decided by the core rather than by this test.
        XCTAssertTrue(plan.commandLine.contains("-g 240"))
        XCTAssertTrue(plan.arguments.contains("-x265-params"))
        XCTAssertTrue(plan.commandLine.contains("rdoq-level=2"))
        XCTAssertEqual(plan.format.sourceKind, "sdr")
    }

    func testPlannerCarriesTheMasteringDisplayThrough() throws {
        let source: [String: Any] = ["pixel_format": "yuv420p10le", "kind": "pq", "components": [10, 10, 10]]
        let display: [String: Any] = [
            "side_data_type": "Mastering display metadata",
            "green_x": "13250/50000",
            "green_y": "34500/50000",
            "blue_x": "7500/50000",
            "blue_y": "3000/50000",
            "red_x": "34000/50000",
            "red_y": "16000/50000",
            "white_point_x": "15635/50000",
            "white_point_y": "16450/50000",
            "max_luminance": "10000000/10000",
            "min_luminance": "50/10000",
        ]
        let job = ConversionJob(
            input: "/tmp/in.mkv",
            output: "/tmp/out.mkv",
            presets: presetTable,
            outputKind: "hdr"
        )
        let probed = ProbeResult(source: source, videoIndex: 0, frameRate: 30, sideData: [display])
        let plan = try ConversionPlanner(ffmpeg: "/usr/bin/false").plan(job, probed: probed)

        // The tokens come from the probe's side data, spelled by the core.
        XCTAssertTrue(plan.commandLine.contains("master-display=G(13250,34500)"))
        XCTAssertTrue(plan.commandLine.contains("hdr10-opt=1"))
    }

    func testPlannerSurfacesTheCoreRejection() {
        let source: [String: Any] = ["pixel_format": "yuv420p", "kind": "sdr", "components": [8, 8, 8]]
        let job = ConversionJob(input: "/tmp/in.mp4", output: "/tmp/out.mkv", presets: presetTable, outputKind: "hdr")

        XCTAssertThrowsError(
            try ConversionPlanner(ffmpeg: "/usr/bin/false")
                .plan(job, probed: ProbeResult(source: source, videoIndex: 0, frameRate: 30))
        ) { error in
            XCTAssertEqual((error as? CoreError)?.code, "policy.hdr_depth")
        }
    }

    func testPlannerOnlyRunsArchivePresets() {
        let source: [String: Any] = ["pixel_format": "yuv420p", "kind": "sdr", "components": [8, 8, 8]]
        let job = ConversionJob(input: "/tmp/in.mp4", output: "/tmp/out.mp4", presets: presetTable, purpose: "share")

        XCTAssertThrowsError(
            try ConversionPlanner(ffmpeg: "/usr/bin/false")
                .plan(job, probed: ProbeResult(source: source, videoIndex: 0, frameRate: 30))
        ) { error in
            guard let conversion = error as? ConversionError, case .unsupported = conversion else {
                return XCTFail("expected an unsupported error, got \(error)")
            }
        }
    }
}
