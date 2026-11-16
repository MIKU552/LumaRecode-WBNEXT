import CLumaCore
import Foundation

/// A stable error identity from the shared core, or a boundary failure.
public enum CoreError: Error, Equatable, Sendable {
    /// The core rejected the request; `code` is localizable, `detail` is a value.
    case rejected(code: String, detail: String?)
    /// The envelope could not be read at all.
    case malformedResponse

    /// Stable code, usable as a localization key.
    public var code: String {
        switch self {
        case let .rejected(code, _):
            return code
        case .malformedResponse:
            return "ffi.malformed_response"
        }
    }
}

/// Identity of the loaded core library.
public struct CoreIdentity: Equatable, Sendable {
    public let abi: UInt32
    public let version: String
    public let operations: [String]
}

/// A normalized source description, built by the platform media adapter.
///
/// macOS fills this from AVFoundation once P5 lands; until then the reference
/// profiles exercise the same rules the Windows adapter will use.
public struct SourceDescription: Equatable, Sendable {
    public var pixelFormat: String
    public var kind: String
    public var profile: String?
    public var colorRange: String?
    public var components: [Int]
    public var rgb = false
    public var palette = false
    public var bayer = false
    public var alpha = false
    public var monochrome = false
    public var raw = false
    public var displayMatrix = false
    public var chromaWidth: Int?
    public var chromaHeight: Int?
    public var bitsPerRawSample: Int?

    public init(
        pixelFormat: String,
        kind: String,
        components: [Int],
        profile: String? = nil,
        colorRange: String? = nil
    ) {
        self.pixelFormat = pixelFormat
        self.kind = kind
        self.components = components
        self.profile = profile
        self.colorRange = colorRange
    }

    var json: [String: Any] {
        var values: [String: Any] = [
            "pixel_format": pixelFormat,
            "kind": kind,
            "components": components,
            "rgb": rgb,
            "palette": palette,
            "bayer": bayer,
            "alpha": alpha,
            "monochrome": monochrome,
            "raw": raw,
            "display_matrix": displayMatrix,
        ]
        if let profile {
            values["profile"] = profile
        }
        if let colorRange {
            values["color_range"] = colorRange
        }
        if let chromaWidth {
            values["log2_chroma_w"] = chromaWidth
        }
        if let chromaHeight {
            values["log2_chroma_h"] = chromaHeight
        }
        if let bitsPerRawSample {
            values["bits_per_raw_sample"] = bitsPerRawSample
        }
        return values
    }
}

/// An encode request. Values match the CLI vocabulary exactly.
public struct OutputOptions: Equatable, Sendable {
    public var purpose = "share"
    public var codec = "hevc"
    public var output = "sdr"
    public var transform = "none"
    public var profile: String?
    public var sampling = "auto"
    public var bitDepth = "auto"
    public var colorRange = "auto"
    public var audio = "auto"
    public var container = "auto"
    public var dynamicHdr = "preserve"
    public var cpuDecode = false

    public init() {}

    var json: [String: Any] {
        var values: [String: Any] = [
            "purpose": purpose,
            "codec": codec,
            "output": output,
            "transform": transform,
            "sampling": sampling,
            "bit_depth": bitDepth,
            "color_range": colorRange,
            "audio": audio,
            "container": container,
            "dynamic_hdr": dynamicHdr,
            "cpu_decode": cpuDecode,
        ]
        if let profile {
            values["profile"] = profile
        }
        return values
    }
}

/// Source facts plus the option lists the UI may show.
public struct PolicyDescription: Decodable, Sendable {
    public struct Facts: Decodable, Sendable {
        public let sampling: String
        public let depth: Int
        public let range: String
        public let kind: String
        public let dynamicHdr: Bool

        enum CodingKeys: String, CodingKey {
            case sampling, depth, range, kind
            case dynamicHdr = "dynamic_hdr"
        }
    }

    public struct Choices: Decodable, Sendable {
        public let samplings: [String]
        public let depths: [Int]
        public let sampling: String
        public let depth: Int?
    }

    public struct Audio: Decodable, Sendable {
        public let flac: Bool
    }

    public let source: Facts
    public let choices: Choices
    public let audio: Audio
}

/// Concrete delivery format produced by resolve/validate.
public struct ResolvedFormat: Decodable, Sendable {
    public let sampling: String
    public let colorRange: String
    public let bitDepth: Int
    public let pixelFormat: String
    public let rangeTag: String
    public let sourceKind: String
    public let container: String
    public let suffix: String

    enum CodingKeys: String, CodingKey {
        case sampling, container, suffix
        case colorRange = "color_range"
        case bitDepth = "bit_depth"
        case pixelFormat = "pixel_format"
        case rangeTag = "range_tag"
        case sourceKind = "source_kind"
    }
}

/// Typed access to the C ABI.
///
/// The transport is a JSON envelope, which keeps a single entry point for every
/// operation while the typed structs for progress events are still being
/// designed. Nothing here throws on a rejected request: the core's stable code
/// is surfaced through `CoreError`.
/// Video and audio facts the core read from an ffprobe document.
public struct ProbeSummary: Decodable, Sendable, Equatable {
    public let width: Int
    public let height: Int
    public let rotation: Int
    public let kind: String?
    public let pixelFormat: String?
    public let depth: Int?
    public let dolbyVision: Int?
    public let hdr10Plus: Bool
    public let audio: Int

    enum CodingKeys: String, CodingKey {
        case width, height, rotation, kind, depth, audio
        case pixelFormat = "pixel_format"
        case dolbyVision = "dolby_vision"
        case hdr10Plus = "hdr10_plus"
    }
}

public struct CoreClient: Sendable {
    public init() {}

    /// ABI revision this client was compiled against.
    public static var abiVersion: UInt32 {
        luma_abi_version()
    }

    /// Version of the linked core, from static storage.
    public static var version: String {
        String(cString: luma_core_version())
    }

    /// Reads the operations this library implements.
    public func identity() throws -> CoreIdentity {
        let payload = try call("core.info")
        let info = try JSONDecoder().decode(Info.self, from: payload)
        return CoreIdentity(abi: info.abi, version: info.coreVersion, operations: info.operations)
    }

    public func describe(source: SourceDescription, options: OutputOptions) throws -> PolicyDescription {
        try decode("policy.describe", source: source, options: options)
    }

    public func resolve(source: SourceDescription, options: OutputOptions) throws -> ResolvedFormat {
        try decode("policy.resolve", source: source, options: options)
    }

    public func validate(source: SourceDescription, options: OutputOptions) throws -> ResolvedFormat {
        try decode("policy.validate", source: source, options: options)
    }

    /// Normalizes an ffprobe document through the shared probe reader.
    public func summarize(probeDocument: String) throws -> ProbeSummary {
        let payload = try call("probe.summarize", request: ["document": probeDocument])
        return try JSONDecoder().decode(ProbeSummary.self, from: payload)
    }

    private func decode<T: Decodable>(_ operation: String, source: SourceDescription, options: OutputOptions) throws -> T {
        let payload = try call(operation, request: ["source": source.json, "options": options.json])
        return try JSONDecoder().decode(T.self, from: payload)
    }

    private func call(_ operation: String, request: [String: Any]) throws -> Data {
        let body = try JSONSerialization.data(withJSONObject: request)
        guard let text = String(data: body, encoding: .utf8) else {
            throw CoreError.malformedResponse
        }
        return try call(operation, request: text)
    }

    private func call(_ operation: String, request: String = "{}") throws -> Data {
        let response = operation.withCString { name in
            request.withCString { body in
                luma_call_json(name, body)
            }
        }
        guard let response else {
            throw CoreError.malformedResponse
        }
        defer { luma_string_free(response) }

        let text = String(cString: response)
        guard let raw = text.data(using: .utf8),
              let parsed = try? JSONSerialization.jsonObject(with: raw),
              let envelope = parsed as? [String: Any]
        else {
            throw CoreError.malformedResponse
        }
        if envelope["ok"] as? Bool == true {
            let data = envelope["data"] ?? [String: Any]()
            return (try? JSONSerialization.data(withJSONObject: data)) ?? Data("{}".utf8)
        }
        let failure = envelope["error"] as? [String: Any]
        throw CoreError.rejected(
            code: failure?["code"] as? String ?? "ffi.error",
            detail: failure?["detail"] as? String
        )
    }

    private struct Info: Decodable {
        let abi: UInt32
        let coreVersion: String
        let operations: [String]

        enum CodingKeys: String, CodingKey {
            case abi, operations
            case coreVersion = "core_version"
        }
    }
}
