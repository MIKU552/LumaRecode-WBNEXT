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
                        TimelineStrip()
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
            if let opened = model.opened {
                VStack(spacing: 8) {
                    Image(systemName: "film")
                        .font(.system(size: 40, weight: .light))
                        .foregroundStyle(.tertiary)
                    Text(opened.url.lastPathComponent)
                        .font(.title3)
                    Text("\(opened.summary.width) × \(opened.summary.height) · \(opened.summary.kind ?? "unknown") · \(opened.summary.depth ?? 0)-bit")
                        .font(.callout)
                        .monospaced()
                    Text("\(opened.summary.audio) audio track(s)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            } else {
                VStack(spacing: 10) {
                    Image(systemName: "film.stack")
                        .font(.system(size: 40, weight: .light))
                        .foregroundStyle(.tertiary)
                    Text(model.selectedReference?.title ?? "No source")
                        .font(.title3)
                    Text("Preview, frame stepping and trimming arrive with the AVFoundation adapter.")
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

/// Empty timeline with a neutral ruler, so the layout is final even before
/// frame data exists.
private struct TimelineStrip: View {
    var body: some View {
        VStack(spacing: 0) {
            Divider()
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
            }
            .frame(height: 56)
        }
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
