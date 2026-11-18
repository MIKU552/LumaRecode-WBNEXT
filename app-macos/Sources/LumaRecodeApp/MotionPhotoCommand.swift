import Foundation
import LumaKit

/// Takes the video out of a motion photo.
///
/// The camera already wrote both parts into one file, so this is a copy rather than a
/// conversion: it reads the layout, then copies the declared range.
enum MotionPhotoCommand {
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
            return fail("usage: LumaRecode motionphoto <photo.jpg> -o <video.mp4>")
        }

        do {
            let info = try MotionPhotoInspector.inspect(fileURL: URL(fileURLWithPath: input))
            let written = try MotionPhotoExtractor.extract(
                from: URL(fileURLWithPath: input),
                to: URL(fileURLWithPath: output)
            )
            let kind = info.isUltraHDR ? "Ultra HDR motion photo" : "motion photo"
            print("wrote \(output)")
            print("\(kind) · video \(written) bytes at offset \(info.videoOffset) · \(info.presentationTimestampUs) us")
            return 0
        } catch let error as MotionPhotoError {
            return fail(error.localizedDescription)
        } catch {
            return fail(error.localizedDescription)
        }
    }

    private static func fail(_ message: String) -> Int32 {
        FileHandle.standardError.write(Data("error: \(message)\n".utf8))
        return ConvertCommand.rejected
    }
}
