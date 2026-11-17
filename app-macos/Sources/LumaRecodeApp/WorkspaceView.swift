import AppKit
import LumaKit
import SwiftUI
import UniformTypeIdentifiers

/// Main window: sources on the left, preview and timeline in the middle,
/// output inspector on the right.
struct WorkspaceView: View {
    @State private var model = WorkspaceModel()
    @State private var importing = false

    var body: some View {
        NavigationSplitView {
            Sidebar(model: model)
                .navigationSplitViewColumnWidth(min: 220, ideal: 260, max: 320)
        } detail: {
            VStack(spacing: 0) {
                HStack(spacing: 0) {
                    VStack(spacing: 0) {
                        PreviewCanvas(model: model)
                        TimelineStrip(model: model)
                    }
                    Divider()
                    Inspector(model: model)
                        .frame(width: 320)
                }
                if model.opened != nil {
                    Divider()
                    ConversionPanel(model: model)
                }
                Divider()
                StatusBar(model: model)
            }
            .frame(minWidth: 820, minHeight: 540)
        }
        .navigationTitle("LumaRecode")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button("Open…") { importing = true }
            }
        }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.movie], allowsMultipleSelection: false) { result in
            guard case .success(let urls) = result, let url = urls.first else { return }
            Task { @MainActor in
                model.open(url: url)
            }
        }
    }
}

private struct Sidebar: View {
    @Bindable var model: WorkspaceModel

    var body: some View {
        List(selection: $model.selectedID) {
            Section("Reference profiles") {
                ForEach(model.references) { reference in
                    VStack(alignment: .leading, spacing: 2) {
                        Text(reference.title)
                            .font(.callout)
                        Text(reference.subtitle)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    .padding(.vertical, 2)
                    .tag(reference.id)
                }
            }

            Section("Media") {
                if let opened = model.opened {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(opened.url.lastPathComponent)
                            .font(.callout)
                        Text("\(opened.summary.width) × \(opened.summary.height)")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    .padding(.vertical, 2)
                } else {
                    Label("Open a clip to inspect it", systemImage: "film")
                        .foregroundStyle(.secondary)
                }
            }
        }
        .listStyle(.sidebar)
        .onChange(of: model.selectedID) {
            model.refresh()
        }
    }
}

private struct PreviewCanvas: View {
    let model: WorkspaceModel

    var body: some View {
        ZStack {
            Color(nsColor: .underPageBackgroundColor)
            if let preview = model.preview {
                Image(nsImage: preview)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .padding(12)
            } else if let opened = model.opened {
                VStack(spacing: 8) {
                    Image(systemName: "film")
                        .font(.system(size: 40, weight: .light))
                        .foregroundStyle(.tertiary)
                    Text(opened.url.lastPathComponent)
                        .font(.title3)
                    Text("\(opened.summary.width) × \(opened.summary.height) · \(opened.summary.kind ?? "unknown") · \(opened.summary.depth ?? 0)-bit")
                        .font(.callout)
                        .monospaced()
                    Text(model.previewFailure ?? "\(opened.summary.audio) audio track(s)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                }
            } else {
                VStack(spacing: 10) {
                    Image(systemName: "film.stack")
                        .font(.system(size: 40, weight: .light))
                        .foregroundStyle(.tertiary)
                    Text(model.selectedReference?.title ?? "No source")
                        .font(.title3)
                    Text("Open a movie to see its first frame and scrub its timeline.")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                        .frame(maxWidth: 380)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

/// A ruler with a playhead: dragging it asks for the frame under the pointer.
private struct TimelineStrip: View {
    @Bindable var model: WorkspaceModel

    var body: some View {
        VStack(spacing: 2) {
            Divider()
            GeometryReader { geometry in
                Canvas { context, size in
                    let spacing: CGFloat = 64
                    var offset: CGFloat = 0
                    while offset < size.width {
                        var tick = Path()
                        tick.move(to: CGPoint(x: offset, y: 0))
                        tick.addLine(to: CGPoint(x: offset, y: 7))
                        context.stroke(tick, with: .color(.secondary.opacity(0.4)), lineWidth: 1)
                        offset += spacing
                    }

                    let x = size.width * TimelineGeometry.fraction(time: model.playhead, duration: model.duration)
                    var head = Path()
                    head.move(to: CGPoint(x: x, y: 0))
                    head.addLine(to: CGPoint(x: x, y: 16))
                    context.stroke(head, with: .color(.accentColor), lineWidth: 2)
                }
                .contentShape(Rectangle())
                .gesture(
                    DragGesture(minimumDistance: 0)
                        .onChanged { value in
                            model.playhead = TimelineGeometry.time(
                                x: value.location.x,
                                width: geometry.size.width,
                                duration: model.duration
                            )
                        }
                        .onEnded { value in
                            model.seek(to: TimelineGeometry.time(
                                x: value.location.x,
                                width: geometry.size.width,
                                duration: model.duration
                            ))
                        }
                )
            }
            .frame(height: 18)
            HStack {
                Text(timecode(model.playhead))
                Spacer()
                Text(timecode(model.duration))
            }
            .font(.caption2)
            .monospacedDigit()
            .foregroundStyle(.secondary)
            .padding(.horizontal, 8)
            .padding(.bottom, 4)
        }
    }

    /// `m:ss.t`, the shape a video tool puts under a timeline.
    private func timecode(_ seconds: Double) -> String {
        guard seconds.isFinite, seconds >= 0 else {
            return "0:00.0"
        }
        let minutes = Int(seconds) / 60
        return String(format: "%d:%04.1f", minutes, seconds - Double(minutes * 60))
    }
}

/// Convert controls for the open file: run, watch, cancel, read the result.
private struct ConversionPanel: View {
    @Bindable var model: WorkspaceModel

    var body: some View {
        HStack(spacing: 10) {
            switch model.conversion {
            case .idle:
                Button("Convert") { model.startConversion() }
            case let .running(progress):
                ProgressView(value: progress)
                    .frame(width: 140)
                Text("\(Int((progress * 100).rounded()))%")
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
                Button("Cancel") { model.cancelConversion() }
            case let .finished(path):
                Image(systemName: "checkmark.circle.fill")
                    .foregroundStyle(.green)
                Text(path)
                    .lineLimit(1)
                    .truncationMode(.middle)
                Button("Again") { model.startConversion() }
            case let .failed(message):
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundStyle(.orange)
                Text(message)
                    .lineLimit(2)
                    .foregroundStyle(.secondary)
                Button("Retry") { model.startConversion() }
            }
            Spacer()
            if let destination = model.suggestedOutput {
                Text(destination.lastPathComponent)
                    .monospaced()
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .font(.callout)
    }
}

private struct StatusBar: View {
    let model: WorkspaceModel

    var body: some View {
        HStack(spacing: 8) {
            Circle()
                .fill(model.identity == nil ? Color.orange : Color.green)
                .frame(width: 7, height: 7)
            if let identity = model.identity {
                Text("Shared core \(identity.version) · ABI \(identity.abi)")
            } else {
                Text("Shared core unavailable")
            }
            if let failure = model.connectionFailure {
                Text(failure)
                    .monospaced()
                    .foregroundStyle(.secondary)
            }
            if let probeError = model.probeError {
                Text(probeError)
                    .monospaced()
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            Spacer()
            Text(model.resolved?.pixelFormat ?? "—")
                .monospaced()
                .foregroundStyle(.secondary)
        }
        .font(.caption)
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
    }
}
