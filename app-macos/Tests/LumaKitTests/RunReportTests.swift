import XCTest
@testable import LumaKit

final class RunReportTests: XCTestCase {
    func testTheReportDecodesWhatTheCoreSends() throws {
        let payload = """
        {"ok":true,"stopped_early":false,"outcomes":[{"ok":true,"exit_code":0,"stdout":"hello\\n"}]}
        """
        let report = try JSONDecoder().decode(RunReport.self, from: Data(payload.utf8))

        XCTAssertTrue(report.ok)
        XCTAssertFalse(report.stoppedEarly)
        XCTAssertEqual(report.outcomes.count, 1)
        XCTAssertEqual(report.outcomes[0].stdout, "hello\n")
        XCTAssertEqual(report.outcomes[0].exitCode, 0)
    }

    func testAFailedStepCarriesItsMessage() throws {
        let payload = """
        {"ok":false,"stopped_early":true,"outcomes":[{"ok":false,"exit_code":1,"message":"boom"}]}
        """
        let report = try JSONDecoder().decode(RunReport.self, from: Data(payload.utf8))

        XCTAssertFalse(report.ok)
        XCTAssertTrue(report.stoppedEarly)
        XCTAssertEqual(report.outcomes[0].message, "boom")
    }
}
