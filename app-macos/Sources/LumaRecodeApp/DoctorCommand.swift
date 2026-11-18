import Foundation
import LumaKit

/// Prints what this machine's FFmpeg can do, one probe at a time.
///
/// The probes run through the core as a sequence, so the progress the user sees comes from
/// the same event stream a long conversion uses. The checks below are readings of the
/// reports FFmpeg printed, not rules: the rules live in the core.
enum DoctorCommand {
    static func run(_ arguments: [String]) -> Int32 {
        guard let ffmpeg = FFmpegLocator.locate() else {
            FileHandle.standardError.write(Data("error: ffmpeg was not found.\n".utf8))
            return ConvertCommand.unavailable
        }

        let probes: [(name: String, command: [String])] = [
            ("version", [ffmpeg, "-hide_banner", "-version"]),
            ("encoders", [ffmpeg, "-hide_banner", "-encoders"]),
            ("filters", [ffmpeg, "-hide_banner", "-filters"]),
        ]

        let sink = ProgressSink { step, total, _ in
            let name = probes[min(step, probes.count - 1)].name
            print("[\(step + 1)/\(total)] \(name)")
        }
        sink.install()
        defer { ProgressSink.clear() }

        let client = CoreClient()
        let report: RunReport
        do {
            report = try client.run(
                commands: probes.map(\.command),
                captureStdout: true
            )
        } catch let error as CoreError {
            print("error: \(error.code)")
            return ConvertCommand.rejected
        } catch {
            print("error: \(error.localizedDescription)")
            return 1
        }

        guard report.ok else {
            let failed = report.outcomes.first { !$0.ok }
            print("error: \(failed?.message ?? "a probe failed")")
            return ConvertCommand.rejected
        }

        print("ffmpeg: \(ffmpeg)")
        if let identity = try? client.identity() {
            print("core: shared \(identity.version) · ABI \(identity.abi)")
        } else {
            print("core: unavailable")
        }

        let encoders = report.outcomes[1].stdout ?? ""
        let filters = report.outcomes[2].stdout ?? ""
        let checks: [(kind: String, name: String, present: Bool)] = [
            ("encoder", "hevc_videotoolbox", encoders.contains("hevc_videotoolbox")),
            ("encoder", "h264_videotoolbox", encoders.contains("h264_videotoolbox")),
            ("encoder", "libx265", encoders.contains("libx265")),
            ("filter", "zscale", filters.contains(" zscale ")),
            ("filter", "lut3d", filters.contains(" lut3d ")),
            ("filter", "libplacebo", filters.contains(" libplacebo ")),
        ]
        for check in checks {
            let mark = check.present ? "yes" : "no "
            print("\(check.kind)  \(check.name.padding(toLength: 20, withPad: " ", startingAt: 0)) \(mark)")
        }

        let missing = checks.filter { !$0.present }.map(\.name)
        if missing.isEmpty {
            print("all checks pass")
        } else {
            print("missing: \(missing.joined(separator: ", "))")
            print("an FFmpeg built with those parts is needed for the conversions that use them")
        }
        return 0
    }
}
