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
            } catch {
                opened = nil
                probeError = error.localizedDescription
            }
        }
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
