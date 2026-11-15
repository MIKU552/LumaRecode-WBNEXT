import Foundation
import LumaKit
import SwiftUI

@main
struct LumaRecodeApp: App {
    init() {
        // The executable doubles as a terminal entry point, as the plan requires for
        // macOS. Keep the check ahead of any window or scene work so a headless run
        // never touches AppKit.
        guard CommandLine.arguments.contains("--version") else {
            return
        }
        print("LumaRecode 4.4.3 WBNEXT")
        do {
            print("core: shared \(try CoreClient().identity().version)")
        } catch {
            print("core: unavailable")
        }
        exit(0)
    }

    var body: some Scene {
        WindowGroup {
            WorkspaceView()
        }
        .defaultSize(width: 1180, height: 760)
        .commands {
            // No document model yet; the new-item command would do nothing.
            CommandGroup(replacing: .newItem) {}
        }

        Settings {
            SettingsView()
        }
    }
}
