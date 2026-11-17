import XCTest
@testable import LumaKit

final class ProgressSampleTests: XCTestCase {
    func testItReadsThePresentationTime() {
        XCTAssertEqual(ProgressSample.microseconds("out_time_us=1500000"), 1_500_000)
        XCTAssertEqual(ProgressSample.microseconds("out_time_us=0"), 0)
    }

    func testItIgnoresEverythingElse() {
        XCTAssertNil(ProgressSample.microseconds("frame=42"))
        XCTAssertNil(ProgressSample.microseconds("out_time_us=N/A"))
        XCTAssertNil(ProgressSample.microseconds("out_time_us=-1"))
        XCTAssertNil(ProgressSample.microseconds(""))
    }
}
