import XCTest
@testable import LumaKit

final class TimelineGeometryTests: XCTestCase {
    func testThePlayheadSitsWhereTheTimeSays() {
        XCTAssertEqual(TimelineGeometry.fraction(time: 5, duration: 10), 0.5, accuracy: 0.0001)
        XCTAssertEqual(TimelineGeometry.fraction(time: -1, duration: 10), 0)
        XCTAssertEqual(TimelineGeometry.fraction(time: 20, duration: 10), 1)
    }

    func testAZeroLengthClipAnswersZeroInsteadOfDividing() {
        XCTAssertEqual(TimelineGeometry.fraction(time: 5, duration: 0), 0)
        XCTAssertEqual(TimelineGeometry.fraction(time: .infinity, duration: 10), 0)
    }

    func testAClickSelectsTheTimeUnderIt() {
        XCTAssertEqual(TimelineGeometry.time(x: 50, width: 100, duration: 10), 5, accuracy: 0.0001)
        XCTAssertEqual(TimelineGeometry.time(x: -10, width: 100, duration: 10), 0)
        XCTAssertEqual(TimelineGeometry.time(x: 110, width: 100, duration: 10), 10)
    }

    func testAZeroWidthStripSelectsNothing() {
        XCTAssertEqual(TimelineGeometry.time(x: 50, width: 0, duration: 10), 0)
    }
}
