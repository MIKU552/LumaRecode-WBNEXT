import SwiftUI

@main
struct LumaRecodeApp: App {
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
