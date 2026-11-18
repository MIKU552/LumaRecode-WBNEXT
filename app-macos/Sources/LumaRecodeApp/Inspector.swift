import LumaKit
import SwiftUI

/// Output inspector: every choice comes from the shared core, and a rejected
/// combination is explained by its stable code rather than a translated guess.
struct Inspector: View {
    @Bindable var model: WorkspaceModel

    var body: some View {
        Form {
            Section("Output") {
                Picker("Purpose", selection: $model.options.purpose) {
                    Text("Quick share").tag("share")
                    Text("Software archive").tag("archive")
                }
                Picker("Codec", selection: $model.options.codec) {
                    Text("HEVC").tag("hevc")
                    Text("AV1").tag("av1")
                    Text("H.264").tag("avc")
                }
                Picker("Dynamic range", selection: $model.options.output) {
                    Text("SDR").tag("sdr")
                    Text("HDR").tag("hdr")
                }
                Picker("Container", selection: $model.options.container) {
                    Text("Automatic").tag("auto")
                    Text("MP4").tag("mp4")
                    Text("MKV").tag("mkv")
                }
            }

            Section("Color") {
                Picker("Transform", selection: $model.options.transform) {
                    Text("None").tag("none")
                    Text("Log → HDR").tag("log-hdr")
                    Text("Log → SDR").tag("log-sdr")
                    Text("HDR → SDR").tag("hdr-sdr")
                }
                if model.options.transform == "log-hdr" || model.options.transform == "log-sdr" {
                    Picker("Camera curve", selection: $model.options.profile) {
                        Text("Not selected").tag(String?.none)
                        Text("DJI D-Log M").tag(String?.some("pocket3"))
                        Text("Samsung Log").tag(String?.some("samsung"))
                        Text("Xiaomi Mi-Log").tag(String?.some("milog"))
                        Text("Apple Log").tag(String?.some("applelog"))
                        Text("Apple Log 2").tag(String?.some("applelog2"))
                    }
                }
                Picker("Chroma sampling", selection: $model.options.sampling) {
                    Text("Automatic").tag("auto")
                    Text("Keep source").tag("source")
                    ForEach(model.description?.choices.samplings ?? [], id: \.self) { value in
                        Text(samplingLabel(value)).tag(value)
                    }
                }
                Picker("Bit depth", selection: $model.options.bitDepth) {
                    Text("Automatic").tag("auto")
                    ForEach(model.description?.choices.depths ?? [], id: \.self) { depth in
                        Text("\(depth)-bit").tag(String(depth))
                    }
                }
                Picker("Sample range", selection: $model.options.colorRange) {
                    Text("Automatic").tag("auto")
                    Text("Keep source").tag("source")
                    Text("Limited").tag("limited")
                    Text("Full").tag("full")
                }
            }

            Section("Source") {
                if let facts = model.description?.source {
                    LabeledContent("Detected") { Text(facts.kind.uppercased()).monospaced() }
                    LabeledContent("Chroma") { Text(samplingLabel(facts.sampling)).monospaced() }
                    LabeledContent("Precision") { Text("\(facts.depth)-bit").monospaced() }
                    LabeledContent("Range") { Text(facts.range).monospaced() }
                    if facts.dynamicHdr {
                        LabeledContent("Dynamic HDR") { Text("present").monospaced() }
                    }
                } else {
                    Text("—").foregroundStyle(.secondary)
                }
                LabeledContent("Lossless audio") {
                    Text(model.description?.audio.flac == true ? "available" : "not available")
                        .monospaced()
                }
            }

            Section("Result") {
                if let resolved = model.resolved {
                    LabeledContent("Pixel format") { Text(resolved.pixelFormat).monospaced() }
                    LabeledContent("Range") { Text(resolved.rangeTag).monospaced() }
                    LabeledContent("File") { Text(resolved.suffix).monospaced() }
                }
                if let rejection = model.rejection {
                    VStack(alignment: .leading, spacing: 4) {
                        Label("Not deliverable", systemImage: "exclamationmark.triangle")
                            .foregroundStyle(.orange)
                        Text(rejection)
                            .font(.caption)
                            .monospaced()
                            .textSelection(.enabled)
                    }
                }
            }
        }
        .formStyle(.grouped)
        .onChange(of: model.options) {
            model.refresh()
        }
    }
}

/// Minimal settings pane: identifies the loaded core and its operations.
struct SettingsView: View {
    private let identity: CoreIdentity?
    private let failure: String?

    init() {
        do {
            identity = try CoreClient().identity()
            failure = nil
        } catch let error as CoreError {
            identity = nil
            failure = error.code
        } catch {
            identity = nil
            failure = "ffi.unavailable"
        }
    }

    var body: some View {
        Form {
            Section("Shared core") {
                LabeledContent("Version") { Text(identity?.version ?? "—").monospaced() }
                LabeledContent("ABI") { Text(identity.map { "\($0.abi)" } ?? "—").monospaced() }
                if let failure {
                    LabeledContent("Status") { Text(failure).monospaced().foregroundStyle(.orange) }
                }
            }
            Section("Operations") {
                ForEach(identity?.operations ?? [], id: \.self) { operation in
                    Text(operation).font(.caption).monospaced()
                }
            }
            Section("Distribution") {
                Text("Ad-hoc signed build. macOS verifies that the app is intact but not who built it.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .frame(width: 420, height: 460)
    }
}

private func samplingLabel(_ value: String) -> String {
    switch value {
    case "420": return "4:2:0"
    case "422": return "4:2:2"
    case "444": return "4:4:4"
    case "gray": return "Gray"
    case "source": return "Source"
    case "unknown": return "Unknown"
    default: return value
    }
}
