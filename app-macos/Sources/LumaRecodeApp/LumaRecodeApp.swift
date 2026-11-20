import Foundation
import LumaKit
import SwiftUI

@main
struct LumaRecodeApp: App {
    init() {
        // The executable doubles as a terminal entry point, as the plan requires for
        // macOS. Keep the check ahead of any window or scene work so a headless run
        // never touches AppKit.
        let arguments = CommandLine.arguments
        if arguments.contains("--version") {
            print("LumaRecode 5.0.7 WBNEXT")
            do {
                print("core: shared \(try CoreClient().identity().version)")
            } catch {
                print("core: unavailable")
            }
            exit(0)
        }

        // The first real conversion path: `convert <input> -o <output>`. It runs and
        // exits before SwiftUI builds a scene, so CI can drive it on a headless runner.
        if arguments.count > 1, arguments[1] == "convert" {
            exit(ConvertCommand.run(Array(arguments.dropFirst(2))))
        }

        // `doctor` answers "why can't it convert": the same probes the pipeline runs,
        // printed one step at a time.
        if arguments.count > 1, arguments[1] == "doctor" {
            exit(DoctorCommand.run(Array(arguments.dropFirst(2))))
        }

        // The HDR10+ document a clip carries, in the shape x265 reads back.
        if arguments.count > 1, arguments[1] == "hdr10plus" {
            exit(HDR10PlusCommand.run(Array(arguments.dropFirst(2))))
        }

        // A motion photo already contains its video, so this is a copy, not a conversion.
        if arguments.count > 1, arguments[1] == "motionphoto" {
            exit(MotionPhotoCommand.run(Array(arguments.dropFirst(2))))
        }
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
