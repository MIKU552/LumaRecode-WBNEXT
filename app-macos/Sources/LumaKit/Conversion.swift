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
    /// `auto` infers a transfer change from the probe; `none`, `hdr-sdr` and the two
    /// `log-*` values pin it.
    public var transform: String
    /// Camera curve name; required by the `log-*` transforms.
    public var logProfile: String
    /// Directory the shipped LUT files live in.
    public var lutDirectory: String
    /// Text of the shipped preset table.
    public var presets: String

    public init(
        input: String,
        output: String,
        presets: String,
        purpose: String = "archive",
        codec: String = "hevc",
        outputKind: String = "sdr",
        inputKind: String = "auto",
        transform: String = "auto",
        logProfile: String = "",
        lutDirectory: String = "Assets"
    ) {
        self.input = input
        self.output = output
        self.presets = presets
        self.purpose = purpose
        self.codec = codec
        self.outputKind = outputKind
        self.inputKind = inputKind
        self.transform = transform
        self.logProfile = logProfile
        self.lutDirectory = lutDirectory
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
    /// The installed FFmpeg lacks a filter the plan needs.
    case missingFilter(String)
    /// The caller asked the run to stop.
    case cancelled
    /// FFmpeg ran and failed.
    case failed(String)

    public var errorDescription: String? {
        switch self {
        case let .unsupported(message):
            return message
        case let .toolMissing(tool):
            return "\(tool) was not found. Install FFmpeg or set LUMARECODE_FFMPEG."
        case let .missingFilter(filter):
            return "This FFmpeg build has no \(filter) filter; install one compiled with it."
        case .cancelled:
            return "The conversion was cancelled."
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
        var options = OutputOptions()
        options.purpose = job.purpose
        options.codec = job.codec
        options.output = job.outputKind
        options.sourceKind = job.inputKind
        // The only transform this command runs is a transfer change, and it is inferred from
        // what the probe described unless the caller pinned one.
        let declaredKind = job.inputKind == "auto" ? (probed.source["kind"] as? String ?? "unknown") : job.inputKind
        let isHdrSource = declaredKind == "hlg" || declaredKind == "pq"
        let isLogSource = declaredKind == "log"
        let transform = job.transform == "auto"
            ? (isLogSource
                ? (job.outputKind == "hdr" ? "log-hdr" : "log-sdr")
                : (isHdrSource && job.outputKind == "sdr" ? "hdr-sdr" : "none"))
            : job.transform
        options.transform = transform
        if isLogSource || transform.hasPrefix("log-") {
            options.profile = job.logProfile.isEmpty ? nil : job.logProfile
        }

        let format = try client.validate(sourceJSON: probed.source, options: options)
        switch transform {
        case "none":
            // Re-encoding a source as itself needs no colour conversion, so the stream keeps
            // its own tags.
            guard isPassthrough(kind: format.sourceKind, output: options.output) else {
                throw ConversionError.unsupported(
                    "\(format.sourceKind) to \(options.output) needs a colour transform; pass --transform hdr-sdr."
                )
            }
        case "hdr-sdr":
            guard format.sourceKind == "hlg" || format.sourceKind == "pq", options.output == "sdr" else {
                throw ConversionError.unsupported("--transform hdr-sdr needs an HDR source and an SDR output.")
            }
            guard !(probed.source["rgb"] as? Bool ?? false) else {
                throw ConversionError.unsupported("RGB sources do not convert between transfers on macOS yet.")
            }
        case "log-hdr", "log-sdr":
            guard format.sourceKind == "log", !job.logProfile.isEmpty else {
                throw ConversionError.unsupported("--transform \(transform) needs --input-kind log and --log-profile.")
            }
            guard !(probed.source["rgb"] as? Bool ?? false) else {
                throw ConversionError.unsupported("RGB sources do not run a curve on macOS yet.")
            }
        default:
            throw ConversionError.unsupported("Only none, hdr-sdr and the log transforms run on macOS so far.")
        }

        let preset = try client.preset(document: job.presets, purpose: options.purpose, codec: options.codec)
        let share = options.purpose == "share"
        // Share runs on VideoToolbox; the table's other rows are software encoders whose
        // flags the core spells, and mixing the two would hand FFmpeg unknown options.
        if share && !preset.encoder.hasSuffix("_videotoolbox") {
            throw ConversionError.unsupported(
                "macOS shares with VideoToolbox only; this preset resolves to \(preset.encoder)."
            )
        }
        let gop = try client.gop(frameRate: probed.frameRate, share: share)
        let mapping = try client.inputMap(
            input: job.input,
            videoIndex: probed.videoIndex,
            audioMap: "0:a?",
            preserveMetadata: true,
            preserveChapters: true
        )
        let dynamic = probed.source["dynamic_hdr"] as? [String: Any]
        let dolbyProfile = (dynamic?["dolby_vision"] as? [String: Any])?["profile"] as? Int
        let hdr10Plus = dynamic?["hdr10_plus"] as? Bool ?? false
        // Both tag sets and every signal-dependent entry come from the core, in the order the
        // Windows chain uses: declare the source, tone map, drop what the conversion
        // invalidates, then convert the range and write the output tags.
        let tags = try client.colorTags(
            kind: format.sourceKind,
            output: options.output,
            rgb: probed.source["rgb"] as? Bool ?? false,
            dolbyProfile5: false,
            sourceRange: probed.sourceRange,
            outputRange: format.colorRange,
            primaries: probed.source["color_primaries"] as? String,
            transfer: probed.source["color_transfer"] as? String,
            space: probed.source["color_space"] as? String
        )
        let usesCurve = transform.hasPrefix("log-")
        let usesToneMap = transform == "hdr-sdr" || transform == "log-sdr"
        var chain: [String] = []
        if usesCurve || usesToneMap {
            var curve: LogFilterInput?
            if usesCurve {
                let profile = try client.logProfile(job.logProfile)
                // A curve reads frames in the range it was graded in unless the file says
                // otherwise, so only a declared range is passed on.
                let declared = probed.source["color_range"] as? String
                curve = LogFilterInput(
                    file: lutPath(profile.file, directory: job.lutDirectory),
                    matrix: profile.matrix,
                    range: profile.range,
                    sourceSpace: probed.source["color_space"] as? String,
                    sourceRange: (declared?.isEmpty ?? true) ? nil : tags.source.range
                )
            }
            let entries = try client.renderFilters(
                log: curve,
                toneMapRange: usesToneMap ? (format.colorRange == "full" ? "pc" : "tv") : nil,
                applyDolbyVision: dolbyProfile != nil,
                dropDynamicMetadata: dolbyProfile != nil || hdr10Plus
            )
            // A curve reads the frame as the camera wrote it, so only a straight transfer
            // change needs the source tags declared first.
            if !usesCurve {
                chain.append(tags.sourceFilter)
            }
            for entry in [entries.log, entries.toneMap, entries.droppedMetadata].compactMap({ $0 }) {
                chain.append(entry)
            }
            chain += try client.colorFilters(
                tags: tags.target,
                conversionRange: format.colorRange,
                pixelFormat: format.pixelFormat,
                purpose: options.purpose,
                depth: format.bitDepth
            )
        } else {
            chain.append("format=\(format.pixelFormat)")
        }
        let filters = try client.filterChain(chain)
        let colorFlags = try client.colorArguments(tags: tags.target)
        let encoder: [String]
        if share {
            encoder = videoToolboxArguments(preset: preset, format: format, codec: options.codec, gop: gop)
        } else {
            encoder = try client.encoderArguments(
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
        }
        // The probe's side data becomes the tokens that keep the stream's HDR declaration:
        // mastering display, light levels, and the payload a preserved dynamic layer needs.
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
        let parameters = share
            ? ""
            : try client.parameterString(values: values, appended: metadata.appended)

        var arguments = [ffmpeg, "-hide_banner", "-nostdin", "-n", "-xerror", "-loglevel", "warning", "-nostats"]
        arguments += mapping
        arguments += ["-vf", filters]
        arguments += encoder
        if !parameters.isEmpty {
            arguments += [parameterFlag(options.codec), parameters]
        }
        arguments += colorFlags
        arguments += ["-fps_mode:v", "passthrough", "-enc_time_base:v", "filter"]
        arguments += try audioArguments(preset: preset, document: job.presets)
        arguments.append(job.output)
        return ConversionPlan(arguments: arguments, format: format)
    }

    /// Copies the audio or encodes it, whichever the table decided for this purpose.
    ///
    /// A share encodes to the named lossy preset, because the container it lands in cannot
    /// carry whatever the source had; an archive copies every track untouched.
    private func audioArguments(preset: PresetSelection, document: String) throws -> [String] {
        guard preset.audio.mode == "encode", let name = preset.audio.preset else {
            return ["-c:a", "copy"]
        }
        let audio = try client.audioPreset(document: document, preset: name)
        var arguments = [
            "-c:a", audio.encoder,
            "-b:a", "\(audio.bitrateKbps)k",
            "-ac", String(audio.channels),
        ]
        if audio.encoder == "libopus" {
            if let rate = audio.sampleRate {
                arguments += ["-ar", String(rate)]
            }
            if let vbr = audio.vbr {
                arguments += ["-vbr", vbr]
            }
            if let application = audio.application {
                arguments += ["-application:a", application]
            }
            if let level = audio.compressionLevel {
                arguments += ["-compression_level:a", String(level)]
            }
        }
        return arguments
    }

    /// VideoToolbox takes its own rate control: `-q:v` instead of CRF or a global quality.
    ///
    /// The Windows table's QSV flags (`-global_quality`, `-low_power`, `-async_depth`) are
    /// unknown options here, which is why the design keeps command construction with the
    /// backend instead of the rules.
    private func videoToolboxArguments(
        preset: PresetSelection,
        format: ResolvedFormat,
        codec: String,
        gop: Int
    ) -> [String] {
        var arguments = ["-c:v", preset.encoder, "-q:v", String(preset.quality)]
        for (key, value) in preset.parameterPairs {
            arguments += ["-\(key)", value]
        }
        arguments += ["-pix_fmt", format.pixelFormat, "-g", String(gop)]
        if codec == "hevc" {
            arguments += ["-profile:v", format.bitDepth == 10 ? "main10" : "main"]
        }
        return arguments
    }

    /// Resolves the LUT the core named against the directory this host keeps it in.
    private func lutPath(_ file: String, directory: String) -> String {
        guard !directory.isEmpty, !file.hasPrefix("/") else {
            return file
        }
        return URL(fileURLWithPath: directory).appendingPathComponent(file).path
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
/// One line of FFmpeg's `-progress` stream.
public enum ProgressSample {
    /// The presentation time reached so far, in microseconds.
    ///
    /// FFmpeg writes `out_time_us=` once per interval and `N/A` before the first frame, so
    /// anything that is not a non-negative integer is ignored.
    public static func microseconds(_ line: String) -> Int64? {
        let prefix = "out_time_us="
        guard line.hasPrefix(prefix) else {
            return nil
        }
        let value = line.dropFirst(prefix.count).trimmingCharacters(in: .whitespaces)
        guard let microseconds = Int64(value), microseconds >= 0 else {
            return nil
        }
        return microseconds
    }
}

/// A box the caller keeps while a run is in flight, so another thread can stop it.
public final class ConversionCancellation: @unchecked Sendable {
    private let lock = NSLock()
    private var cancelled = false

    public init() {}

    public func cancel() {
        lock.lock()
        cancelled = true
        lock.unlock()
    }

    public var isCancelled: Bool {
        lock.lock()
        defer { lock.unlock() }
        return cancelled
    }
}

/// Runs a plan and reports how far FFmpeg has got.
public enum ConversionRunner {
    /// Executes the plan and fails when FFmpeg reports a non-zero status.
    ///
    /// The call blocks; callers run it off the main thread. Progress comes from FFmpeg's own
    /// `-progress` stream, so the fraction is measured work rather than a guess, and a run
    /// that is asked to stop throws `.cancelled` once FFmpeg has exited.
    public static func run(
        _ plan: ConversionPlan,
        duration: Double = 0,
        cancellation: ConversionCancellation = ConversionCancellation(),
        progress: (Double) -> Void = { _ in }
    ) throws {
        guard let executable = plan.arguments.first else {
            throw ConversionError.toolMissing("ffmpeg")
        }

        // The report goes before the destination, where FFmpeg still reads output options.
        var arguments = Array(plan.arguments.dropFirst())
        if let destination = arguments.popLast() {
            arguments += ["-progress", "pipe:1", "-nostats"]
            arguments.append(destination)
        }

        let process = Process()
        process.executableURL = URL(fileURLWithPath: executable)
        process.arguments = arguments
        let output = Pipe()
        let errors = Pipe()
        process.standardOutput = output
        process.standardError = errors

        try process.run()
        var reported = 0.0
        var buffer = Data()
        let reader = output.fileHandleForReading
        while !cancellation.isCancelled {
            let chunk = reader.availableData
            if chunk.isEmpty {
                break
            }
            buffer.append(chunk)
            while let newline = buffer.firstIndex(of: 0x0A) {
                let line = String(decoding: buffer[buffer.startIndex..<newline], as: UTF8.self)
                buffer.removeSubrange(buffer.startIndex...newline)
                guard duration > 0, let microseconds = ProgressSample.microseconds(line) else {
                    continue
                }
                let fraction = min(max(Double(microseconds) / (duration * 1_000_000), 0), 1)
                if fraction > reported {
                    reported = fraction
                    progress(fraction)
                }
            }
        }

        if cancellation.isCancelled {
            process.terminate()
        }
        let errorData = errors.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        if cancellation.isCancelled {
            throw ConversionError.cancelled
        }
        guard process.terminationStatus == 0 else {
            let message = String(data: errorData, encoding: .utf8) ?? ""
            throw ConversionError.failed(message.trimmingCharacters(in: .whitespacesAndNewlines))
        }
    }
}

/// The preset table and the asset directory both hosts resolve against.
public enum PresetDocument {
    /// Where the shipped LUTs and preset tables live.
    public static var assetDirectory: String {
        ProcessInfo.processInfo.environment["LUMARECODE_ASSETS"] ?? "Assets"
    }

    /// Reads the shipped preset table from the first place that has it.
    public static func load(configured: String? = nil) -> String? {
        var candidates: [String] = []
        if let configured {
            candidates.append(configured)
        }
        if let environment = ProcessInfo.processInfo.environment["LUMARECODE_PRESETS"] {
            candidates.append(environment)
        }
        // The share rows name a backend: VideoToolbox here, Intel QSV in the shared table.
        candidates.append("\(assetDirectory)/recommended-presets-macos.json")
        candidates.append("\(assetDirectory)/recommended-presets.json")
        for path in candidates where FileManager.default.fileExists(atPath: path) {
            if let text = try? String(contentsOfFile: path, encoding: .utf8) {
                return text
            }
        }
        return nil
    }
}
