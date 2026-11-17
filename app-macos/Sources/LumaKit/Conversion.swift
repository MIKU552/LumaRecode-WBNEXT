import Foundation

/// What one conversion should produce.
///
/// Values use the same vocabulary as the Windows CLI, so a support report reads the
/// same on both platforms.
public struct ConversionJob: Sendable {
    public var input: String
    public var output: String
    public var purpose: String
    public var codec: String
    /// Delivery transfer function: `sdr` or `hdr`.
    public var outputKind: String
    /// Manual interpretation of the source; `auto` trusts the probe.
    public var inputKind: String
    /// Text of the shipped preset table.
    public var presets: String

    public init(
        input: String,
        output: String,
        presets: String,
        purpose: String = "archive",
        codec: String = "hevc",
        outputKind: String = "sdr",
        inputKind: String = "auto"
    ) {
        self.input = input
        self.output = output
        self.presets = presets
        self.purpose = purpose
        self.codec = codec
        self.outputKind = outputKind
        self.inputKind = inputKind
    }
}

/// The argument vector the core decided on, plus the format it resolved.
public struct ConversionPlan: Sendable {
    /// Executable first, exactly as `Process` wants it.
    public let arguments: [String]
    public let format: ResolvedFormat

    /// The command as one line, for logs and support reports.
    public var commandLine: String {
        arguments.joined(separator: " ")
    }
}

/// A conversion this build refuses to guess about.
public enum ConversionError: Error, Equatable, Sendable, LocalizedError {
    /// The request is clear but this command cannot render it yet.
    case unsupported(String)
    /// An external tool is missing.
    case toolMissing(String)
    /// FFmpeg ran and failed.
    case failed(String)

    public var errorDescription: String? {
        switch self {
        case let .unsupported(message):
            return message
        case let .toolMissing(tool):
            return "\(tool) was not found. Install FFmpeg or set LUMARECODE_FFMPEG."
        case let .failed(message):
            return message.isEmpty ? "FFmpeg failed." : message
        }
    }
}

/// Builds one FFmpeg command, asking the core for every decision.
///
/// The host contributes only what it alone knows: where the file is, which executable to
/// run, and the filter content the platform has not migrated yet. Encoder, rate control,
/// interval, stream mapping and parameter spelling all come from the shared core, so the
/// macOS command and the Windows command cannot drift apart.
public struct ConversionPlanner: Sendable {
    private let ffmpeg: String
    private let client: CoreClient

    public init(ffmpeg: String, client: CoreClient = CoreClient()) {
        self.ffmpeg = ffmpeg
        self.client = client
    }

    public func plan(_ job: ConversionJob, probed: ProbeResult) throws -> ConversionPlan {
        // The share table is measured for QSV; macOS needs its own VideoToolbox entries
        // before that path can be honest, so this command archives only.
        guard job.purpose == "archive" else {
            throw ConversionError.unsupported("Only archive presets exist for macOS so far; pass --purpose archive.")
        }

        var options = OutputOptions()
        options.purpose = job.purpose
        options.codec = job.codec
        options.output = job.outputKind
        options.transform = "none"
        options.sourceKind = job.inputKind

        let format = try client.validate(sourceJSON: probed.source, options: options)
        // A transfer change needs the tone-mapping and LUT chains, which stay in the
        // Windows host until they move into the core. Re-encoding a source as itself
        // needs no colour conversion, so the stream keeps its own tags.
        guard isPassthrough(kind: format.sourceKind, output: options.output) else {
            throw ConversionError.unsupported(
                "\(format.sourceKind) to \(options.output) needs a colour transform, which macOS does not run yet."
            )
        }

        let preset = try client.preset(document: job.presets, purpose: options.purpose, codec: options.codec)
        let share = options.purpose == "share"
        let gop = try client.gop(frameRate: probed.frameRate, share: share)
        let mapping = try client.inputMap(
            input: job.input,
            videoIndex: probed.videoIndex,
            audioMap: "0:a?",
            preserveMetadata: true,
            preserveChapters: true
        )
        let filters = try client.filterChain(["format=\(format.pixelFormat)"])
        let encoder = try client.encoderArguments(
            encoder: preset.encoder,
            preset: preset.preset,
            quality: String(preset.quality),
            share: share,
            codec: options.codec,
            depth: format.bitDepth,
            gop: gop,
            autoLowPower: false,
            timingCompatibility: false,
            asyncDepth: "4",
            pixelFormat: format.pixelFormat
        )
        // The probe's side data becomes the tokens that keep the stream's HDR declaration:
        // mastering display, light levels, and the payload a preserved dynamic layer needs.
        let dynamic = probed.source["dynamic_hdr"] as? [String: Any]
        let dolbyProfile = (dynamic?["dolby_vision"] as? [String: Any])?["profile"] as? Int
        let hdr10Plus = dynamic?["hdr10_plus"] as? Bool ?? false
        let metadata = try client.hdrMetadata(
            codec: options.codec,
            output: options.output,
            sourceKind: format.sourceKind,
            preserveDynamic: options.output == "hdr" && (dolbyProfile != nil || hdr10Plus),
            dolbyVisionProfile: dolbyProfile,
            hdr10Plus: hdr10Plus,
            sideData: probed.sideData
        )
        let values = preset.parameterPairs + metadata.parameters.map { ($0.key, $0.value) }
        let parameters = try client.parameterString(values: values, appended: metadata.appended)

        var arguments = [ffmpeg, "-hide_banner", "-nostdin", "-n", "-xerror", "-loglevel", "warning", "-nostats"]
        arguments += mapping
        arguments += ["-vf", filters]
        arguments += encoder
        if !parameters.isEmpty {
            arguments += [parameterFlag(options.codec), parameters]
        }
        arguments += ["-fps_mode:v", "passthrough", "-enc_time_base:v", "filter"]
        // Audio is copied: a first command should not silently re-encode tracks whose
        // policy decisions have not moved to macOS yet.
        arguments += ["-c:a", "copy"]
        arguments.append(job.output)
        return ConversionPlan(arguments: arguments, format: format)
    }

    /// The flag each encoder family reads its parameter string from.
    private func parameterFlag(_ codec: String) -> String {
        switch codec {
        case "hevc":
            return "-x265-params"
        case "av1":
            return "-svtav1-params"
        default:
            return "-x264-params"
        }
    }

    /// True when re-encoding the source as itself needs no colour conversion.
    private func isPassthrough(kind: String, output: String) -> Bool {
        switch kind {
        case "sdr":
            return output == "sdr"
        case "hlg", "pq":
            return output == "hdr"
        default:
            return false
        }
    }
}

/// Runs one planned conversion.
public enum ConversionRunner {
    /// Executes the plan and fails when FFmpeg reports a non-zero status.
    ///
    /// The call blocks; callers run it off the main thread. Output is read before waiting,
    /// which is safe because the command keeps FFmpeg quiet (`-loglevel warning -nostats`).
    public static func run(_ plan: ConversionPlan) throws {
        guard let executable = plan.arguments.first else {
            throw ConversionError.toolMissing("ffmpeg")
        }

        let process = Process()
        process.executableURL = URL(fileURLWithPath: executable)
        process.arguments = Array(plan.arguments.dropFirst())
        let errors = Pipe()
        process.standardError = errors
        process.standardOutput = FileHandle.nullDevice

        try process.run()
        let data = errors.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            let message = String(data: data, encoding: .utf8) ?? ""
            throw ConversionError.failed(message.trimmingCharacters(in: .whitespacesAndNewlines))
        }
    }
}
