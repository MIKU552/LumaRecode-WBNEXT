import AppKit
import Foundation
import LumaKit
import Observation

/// Drives the first workspace: reference source profiles, output choices and
/// the resolved format that the shared core returns for them.
///
/// These references are declared descriptions, not decoded files. They exist so
/// the ABI round trip and the shared rules are exercised before the AVFoundation
/// probe lands; the UI labels them as reference profiles so no one mistakes them
/// for real media.
@MainActor
@Observable
final class WorkspaceModel {
    struct Reference: Identifiable {
        let id: String
        let title: String
        let subtitle: String
        let source: SourceDescription
    }

    let references: [Reference] = [
        Reference(
            id: "sdr-8-420",
            title: "SDR · 8-bit 4:2:0",
            subtitle: "H.264 camera footage, limited range",
            source: SourceDescription(pixelFormat: "yuv420p", kind: "sdr", components: [8, 8, 8])
        ),
        Reference(
            id: "hlg-10-420",
            title: "HLG · 10-bit 4:2:0",
            subtitle: "HDR phone capture, BT.2020",
            source: {
                var source = SourceDescription(pixelFormat: "yuv420p10le", kind: "hlg", components: [10, 10, 10])
                source.chromaWidth = 1
                source.chromaHeight = 1
                return source
            }()
        ),
        Reference(
            id: "pq-12-444",
            title: "PQ · 12-bit 4:4:4",
            subtitle: "Grading master, full chroma",
            source: {
                var source = SourceDescription(pixelFormat: "yuv444p12le", kind: "pq", components: [12, 12, 12])
                source.chromaWidth = 0
                source.chromaHeight = 0
                return source
            }()
        ),
        Reference(
            id: "log-milog",
            title: "Mi-Log · 10-bit",
            subtitle: "Xiaomi camera curve, full range",
            source: {
                var source = SourceDescription(pixelFormat: "yuv420p10le", kind: "log", components: [10, 10, 10], profile: "milog")
                source.chromaWidth = 1
                source.chromaHeight = 1
                return source
            }()
        ),
        Reference(
            id: "rgb-8",
            title: "RGB · 8-bit",
            subtitle: "Screen recording, no chroma subsampling",
            source: {
                var source = SourceDescription(pixelFormat: "rgb24", kind: "sdr", components: [8, 8, 8])
                source.rgb = true
                return source
            }()
        ),
    ]

    var selectedID: Reference.ID?
    var options = OutputOptions()
    var identity: CoreIdentity?
    var description: PolicyDescription?
    var resolved: ResolvedFormat?
    var rejection: String?
    var connectionFailure: String?

    /// A file the user opened, with the facts the shared probe returned.
    struct OpenedSource: Equatable {
        let url: URL
        let summary: ProbeSummary
    }

    var opened: OpenedSource?
    var probeError: String?

    /// The frame shown for the current playhead, and why it is missing when it is.
    var preview: NSImage?
    var previewFailure: String?
    var playhead: Double = 0
    private var previewTask: Task<Void, Never>?

    /// Seconds of the opened file, from the probe rather than from the asset.
    var duration: Double {
        opened?.summary.durationSeconds ?? 0
    }

    /// What the convert button is doing, if anything.
    enum ConversionState: Equatable {
        case idle
        case running(Double)
        case finished(String)
        case failed(String)
    }

    var conversion: ConversionState = .idle
    private var cancellation: ConversionCancellation?
    private var conversionTask: Task<Void, Never>?

    /// Where a conversion of the open file writes: beside it, with a clear name.
    var suggestedOutput: URL? {
        guard let opened else {
            return nil
        }
        let base = opened.url.deletingPathExtension().lastPathComponent
        return opened.url.deletingLastPathComponent()
            .appendingPathComponent("\(base)-converted.mkv")
    }

    var selectedReference: Reference? {
        references.first { $0.id == selectedID }
    }

    init() {
        selectedID = references.first?.id
        connect()
        refresh()
    }

    func connect() {
        do {
            identity = try CoreClient().identity()
            connectionFailure = nil
        } catch let error as CoreError {
            identity = nil
            connectionFailure = error.code
        } catch {
            identity = nil
            connectionFailure = "ffi.unavailable"
        }
    }

    /// Reads one file with ffprobe and normalizes it through the shared core.
    ///
    /// ffprobe runs off the main actor because it blocks on the child process; the core
    /// call stays on the main actor, which is where every other core call happens.
    func open(url: URL) {
        probeError = nil
        Task {
            do {
                let document = try await Task.detached(priority: .userInitiated) {
                    try ProbeRunner.document(path: url.path)
                }.value
                opened = OpenedSource(url: url, summary: try CoreClient().summarize(probeDocument: document))
                preview = nil
                previewFailure = nil
                seek(to: 0)
            } catch {
                opened = nil
                probeError = error.localizedDescription
            }
        }
    }

    /// Asks for the frame at `seconds`; a newer request cancels the older one.
    ///
    /// The generator runs off the main actor, so dragging the playhead never stalls the window.
    func seek(to seconds: Double) {
        guard let opened else {
            return
        }
        playhead = min(max(seconds, 0), max(duration, 0))
        let target = playhead
        previewTask?.cancel()
        previewTask = Task {
            do {
                let image = try await PreviewFrames.image(path: opened.url.path, seconds: target)
                guard !Task.isCancelled else {
                    return
                }
                preview = image
                previewFailure = nil
            } catch {
                guard !Task.isCancelled else {
                    return
                }
                preview = nil
                previewFailure = error.localizedDescription
            }
        }
    }

    /// Converts the open file with the options the inspector shows.
    ///
    /// Probing, planning and encoding run off the main actor; only the state changes come
    /// back to it. FFmpeg's own progress stream drives the bar, so a stalled encode looks
    /// stalled instead of pretending to move.
    func startConversion() {
        guard let opened, let destination = suggestedOutput else {
            return
        }
        if case .running = conversion {
            return
        }
        guard let presets = PresetDocument.load() else {
            conversion = .failed("The preset table is unavailable.")
            return
        }

        let job = ConversionJob(
            input: opened.url.path,
            output: destination.path,
            presets: presets,
            purpose: options.purpose,
            codec: options.codec,
            outputKind: options.output,
            inputKind: "auto",
            transform: "auto",
            logProfile: options.profile ?? "",
            lutDirectory: PresetDocument.assetDirectory
        )
        let cancellation = ConversionCancellation()
        self.cancellation = cancellation
        conversion = .running(0)
        conversionTask = Task {
            do {
                try await Task.detached(priority: .userInitiated) {
                    let document = try ProbeRunner.document(path: opened.url.path, includeFirstFrame: true)
                    let probed = try CoreClient().probe(document: document)
                    guard let ffmpeg = FFmpegLocator.locate() else {
                        throw ConversionError.toolMissing("ffmpeg")
                    }
                    let plan = try ConversionPlanner(ffmpeg: ffmpeg).plan(job, probed: probed)
                    try ConversionRunner.run(plan, duration: probed.duration, cancellation: cancellation) { fraction in
                        Task { @MainActor in
                            if case .running = self.conversion {
                                self.conversion = .running(fraction)
                            }
                        }
                    }
                }.value
                conversion = .finished(destination.path)
            } catch ConversionError.cancelled {
                conversion = .idle
            } catch {
                conversion = .failed(error.localizedDescription)
            }
            self.cancellation = nil
            self.conversionTask = nil
        }
    }

    /// Asks the running conversion to stop; the state returns to idle once FFmpeg has exited.
    func cancelConversion() {
        cancellation?.cancel()
    }

    /// Re-runs describe and validate for the current selection and options.
    ///
    /// `describe` never rejects a request, so it is queried first and its result
    /// stays available even when `validate` explains why the combination is not
    /// deliverable.
    func refresh() {
        guard let reference = selectedReference else {
            description = nil
            resolved = nil
            rejection = nil
            return
        }

        let client = CoreClient()
        do {
            description = try client.describe(source: reference.source, options: options)
        } catch let error as CoreError {
            description = nil
            rejection = error.code
        } catch {
            description = nil
            rejection = "ffi.unavailable"
        }

        do {
            resolved = try client.validate(source: reference.source, options: options)
            rejection = nil
        } catch let error as CoreError {
            resolved = nil
            rejection = error.code
        } catch {
            resolved = nil
            rejection = "ffi.unavailable"
        }
    }

    /// Keeps sampling and depth selectable when the reference changes by
    /// clearing a value the new source cannot honour.
    func select(_ id: Reference.ID?) {
        selectedID = id
        refresh()
    }
}
