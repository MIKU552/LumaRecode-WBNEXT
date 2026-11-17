import Foundation

/// Maps between a timeline strip's width and the asset's time.
///
/// Kept out of the view so the clamping rules can be tested without a window: a click past
/// either end has to land on the end, and a zero-length strip or clip has to answer zero
/// rather than divide by it.
public enum TimelineGeometry {
    /// Where the playhead sits, as a fraction of the strip.
    public static func fraction(time: Double, duration: Double) -> Double {
        guard duration > 0, time.isFinite else {
            return 0
        }
        return min(max(time / duration, 0), 1)
    }

    /// The time a click or drag at `x` selects.
    public static func time(x: Double, width: Double, duration: Double) -> Double {
        guard width > 0, duration > 0, x.isFinite else {
            return 0
        }
        return min(max(x / width, 0), 1) * duration
    }
}
