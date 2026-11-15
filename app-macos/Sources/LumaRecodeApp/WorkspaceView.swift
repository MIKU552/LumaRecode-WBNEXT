import AppKit
import LumaKit
import SwiftUI

/// Main window: sources on the left, preview and timeline in the middle,
/// output inspector on the right.
struct WorkspaceView: View {
    @State private var model = WorkspaceModel()

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
                Divider()
                StatusBar(model: model)
            }
            .frame(minWidth: 820, minHeight: 540)
        }
        .navigationTitle("LumaRecode")
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
                Label("Open a clip to inspect it", systemImage: "film")
                    .foregroundStyle(.secondary)
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
            .background(Color(nsColor: .underPageBackgroundColor))
        }
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
            Spacer()
            Text(model.resolved?.pixelFormat ?? "—")
                .monospaced()
                .foregroundStyle(.secondary)
        }
        .font(.caption)
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(.bar)
    }
}
