import Foundation
import LumaKit

/// The first real conversion command on macOS.
///
/// Headless and synchronous: ffprobe reads the file, the shared core decides every value,
/// `ConversionPlanner` builds the argument vector and FFmpeg encodes. Exit codes match the
/// Windows CLI so a script can tell a refusal from a crash.
enum ConvertCommand {
    static let rejected: Int32 = 2
    static let unavailable: Int32 = 3

    static func run(_ arguments: [String]) -> Int32 {
        do {
            return try execute(arguments)
        } catch let error as CoreError {
            fail("error: \(error.code)")
            return rejected
        } catch let error as ConversionError {
            fail("error: \(error.localizedDescription)")
            if case .toolMissing = error {
                return unavailable
            }
            if case .missingFilter = error {
                return unavailable
            }
            return rejected
        } catch {
            fail("error: \(error.localizedDescription)")
            return 1
        }
    }

    private static func execute(_ arguments: [String]) throws -> Int32 {
        let job = try parse(arguments)
        let candidates = ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg", "/usr/bin/ffmpeg"]
        guard let ffmpeg = locate("LUMARECODE_FFMPEG", candidates: candidates) else {
            throw ConversionError.toolMissing("ffmpeg")
        }

        let document = try ProbeRunner.document(path: job.input, includeFirstFrame: true)
        let probed = try CoreClient().probe(document: document)
        let plan = try ConversionPlanner(ffmpeg: ffmpeg).plan(job, probed: probed)
        print(plan.commandLine)
        // The plan names every filter it needs; a build without one must say so here rather
        // than fail inside FFmpeg with "No such filter".
        for filter in ["libplacebo"] where plan.commandLine.contains("\(filter)=") {
            guard hasFilter(filter, ffmpeg: ffmpeg) else {
                throw ConversionError.missingFilter(filter)
            }
        }
        try ConversionRunner.run(plan)
        print("wrote \(job.output)")
        return 0
    }

    private static func parse(_ arguments: [String]) throws -> ConversionJob {
        var flags: [String: String] = [:]
        var positional: [String] = []
        var index = 0
        while index < arguments.count {
            let argument = arguments[index]
            if argument.hasPrefix("-") {
                guard index + 1 < arguments.count else {
                    throw ConversionError.unsupported("Missing value for \(argument).")
                }
                flags[argument] = arguments[index + 1]
                index += 2
            } else {
                positional.append(argument)
                index += 1
            }
        }

        guard let input = positional.first, let output = flags["-o"] ?? flags["--destination"] else {
            throw ConversionError.unsupported(
                "usage: LumaRecode convert <input> -o <output> [--purpose archive] [--codec hevc|av1|avc] [--output sdr|hdr] [--input-kind auto|sdr|hlg|pq] [--transform auto|none|hdr-sdr|log-hdr|log-sdr] [--log-profile pocket3|samsung|milog|applelog|applelog2] [--lut-dir <path>] [--presets <path>]"
            )
        }
        guard let presets = presetsDocument(flags["--presets"]) else {
            throw ConversionError.unsupported(
                "The preset table is unavailable; pass --presets <path> or set LUMARECODE_PRESETS."
            )
        }

        return ConversionJob(
            input: input,
            output: output,
            presets: presets,
            purpose: flags["--purpose"] ?? "archive",
            codec: flags["--codec"] ?? "hevc",
            outputKind: flags["--output"] ?? "sdr",
            inputKind: flags["--input-kind"] ?? "auto",
            transform: flags["--transform"] ?? "auto",
            logProfile: flags["--log-profile"] ?? "",
            lutDirectory: flags["--lut-dir"]
                ?? ProcessInfo.processInfo.environment["LUMARECODE_ASSETS"]
                ?? "Assets"
        )
    }

    /// Reads the shipped preset table from the first place that has it.
    private static func presetsDocument(_ configured: String?) -> String? {
        var candidates: [String] = []
        if let configured {
            candidates.append(configured)
        }
        if let environment = ProcessInfo.processInfo.environment["LUMARECODE_PRESETS"] {
            candidates.append(environment)
        }
        candidates.append("Assets/recommended-presets.json")
        for path in candidates where FileManager.default.fileExists(atPath: path) {
            if let text = try? String(contentsOfFile: path, encoding: .utf8) {
                return text
            }
        }
        return nil
    }

    private static func locate(_ variable: String, candidates: [String]) -> String? {
        if let configured = ProcessInfo.processInfo.environment[variable],
           FileManager.default.isExecutableFile(atPath: configured) {
            return configured
        }
        return candidates.first { FileManager.default.isExecutableFile(atPath: $0) }
    }

    /// Reads the filter list from the FFmpeg that will run the plan.
    private static func hasFilter(_ name: String, ffmpeg: String) -> Bool {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: ffmpeg)
        process.arguments = ["-hide_banner", "-filters"]
        let output = Pipe()
        process.standardOutput = output
        process.standardError = FileHandle.nullDevice
        guard (try? process.run()) != nil else {
            return false
        }
        let data = output.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        let listing = String(data: data, encoding: .utf8) ?? ""
        return listing.contains(" \(name) ")
    }

    private static func fail(_ message: String) {
        FileHandle.standardError.write(Data((message + "\n").utf8))
    }
}
