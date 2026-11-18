import Foundation
import LumaKit

/// Writes the HDR10+ document a clip carries, ready to hand to x265.
enum HDR10PlusCommand {
    static func run(_ arguments: [String]) -> Int32 {
        var input: String?
        var output: String?
        var index = 0
        while index < arguments.count {
            let argument = arguments[index]
            if argument == "-o" || argument == "--output" {
                guard index + 1 < arguments.count else {
                    return fail("missing value for \(argument)")
                }
                output = arguments[index + 1]
                index += 2
            } else {
                input = argument
                index += 1
            }
        }

        guard let input, let output else {
            return fail("usage: LumaRecode hdr10plus <input> -o <document.json>")
        }
        guard let ffprobe = FFmpegLocator.locateProbe() else {
            return fail("ffprobe was not found beside ffmpeg")
        }

        do {
            let (frames, document) = try HDR10PlusScanner.document(
                ffprobe: ffprobe,
                path: input
            )
            try document.write(to: URL(fileURLWithPath: output))
            print("wrote \(output)")
            print("\(frames) frame(s) with HDR10+ metadata")
            return 0
        } catch {
            return fail(error.localizedDescription)
        }
    }

    private static func fail(_ message: String) -> Int32 {
        FileHandle.standardError.write(Data("error: \(message)\n".utf8))
        return ConvertCommand.rejected
    }
}
