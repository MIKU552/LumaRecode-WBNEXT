//! Timeline tests: ticks, VFR spacing, keyframe seeks and range validation.

use luma_core::timeline::{
    FramePoint, FrameRange, FrameTimeline, parse_index_line, sorted_unique_packets,
};

fn point(pts: i64, duration: i64, key_frame: bool) -> FramePoint {
    FramePoint {
        pts,
        duration,
        key_frame,
    }
}

fn timeline(frames: Vec<FramePoint>) -> FrameTimeline {
    FrameTimeline::new(frames, 1, 1000, true).expect("valid timeline")
}

#[test]
fn positions_use_real_timestamps_not_an_average_rate() {
    // 1 s, 1 s, 0.5 s: a variable-rate source an average FPS would misplace.
    let line = timeline(vec![
        point(0, 1000, true),
        point(1000, 1000, false),
        point(2000, 500, false),
        point(2500, 0, false),
    ]);
    assert_eq!(line.count(), 4);
    assert!((line.position(1) - 1.0).abs() < 1e-9);
    assert!((line.position(2) - 2.0).abs() < 1e-9);
    assert!((line.position(3) - 2.5).abs() < 1e-9);
    // Last frame has no declared duration; the previous gap is the fallback.
    assert!((line.boundary_seconds(4) - 3.0).abs() < 1e-9);
    assert!((line.duration() - 3.0).abs() < 1e-9);
}

#[test]
fn nonzero_start_pts_still_reports_zero_position() {
    let line = timeline(vec![point(9000, 1000, true), point(10000, 1000, false)]);
    assert!((line.seconds(0) - 9.0).abs() < 1e-9);
    assert!((line.position(0)).abs() < 1e-9);
    assert!((line.position(1) - 1.0).abs() < 1e-9);
    // The end boundary is absolute, so the offset is preserved.
    assert!((line.boundary_seconds(2) - 11.0).abs() < 1e-9);
}

#[test]
fn negative_timestamps_are_not_sought_to() {
    let line = timeline(vec![
        point(-500, 500, true),
        point(0, 1000, false),
        point(1000, 1000, true),
    ]);
    let window = line.resolve(FrameRange::new(1, 2)).expect("valid range");
    assert_eq!(window.seek_frame, 0, "negative PTS cannot be a seek target");
    assert_eq!(window.seek_seconds, 0.0);
}

#[test]
fn at_or_after_finds_the_first_frame_at_a_position() {
    let line = timeline(vec![
        point(0, 1000, true),
        point(1000, 1000, false),
        point(3000, 1000, false),
    ]);
    assert_eq!(line.at_or_after(0.0), 0);
    assert_eq!(line.at_or_after(0.5), 1);
    assert_eq!(line.at_or_after(1.0), 1);
    assert_eq!(line.at_or_after(2.999), 2);
    assert_eq!(line.at_or_after(9.0), 3);
}

#[test]
fn resolve_seeks_to_the_nearest_preceding_keyframe() {
    let line = timeline(vec![
        point(0, 1000, true),
        point(1000, 1000, false),
        point(2000, 1000, true),
        point(3000, 1000, false),
    ]);
    let window = line.resolve(FrameRange::new(1, 3)).expect("valid range");
    assert_eq!(window.seek_frame, 0);
    assert_eq!(window.count(), 2);
    assert!((window.start_seconds - 1.0).abs() < 1e-9);
    assert!((window.end_seconds - 3.0).abs() < 1e-9);
    assert!((window.duration() - 2.0).abs() < 1e-9);

    let window = line.resolve(FrameRange::new(3, 4)).expect("valid range");
    assert_eq!(window.seek_frame, 2);
    assert!((window.seek_seconds - 2.0).abs() < 1e-9);
}

#[test]
fn decoded_timelines_always_decode_from_the_start() {
    let decoded = FrameTimeline::new(
        vec![point(2000, 1000, true), point(3000, 1000, true)],
        1,
        1000,
        false,
    )
    .expect("valid timeline");
    let window = decoded.resolve(FrameRange::new(1, 2)).expect("valid range");
    assert_eq!(window.seek_frame, 0);
    assert!(!decoded.is_packet_indexed());
}

#[test]
fn invalid_ranges_and_time_bases_are_rejected() {
    let line = timeline(vec![point(0, 1000, true), point(1000, 1000, false)]);
    assert_eq!(
        line.resolve(FrameRange::new(2, 2))
            .expect_err("empty range")
            .code(),
        "timeline.range"
    );
    assert_eq!(
        line.resolve(FrameRange::new(0, 5))
            .expect_err("past the end")
            .code(),
        "timeline.range"
    );
    assert_eq!(
        line.resolve(FrameRange::new(1, 1))
            .expect_err("end before start")
            .code(),
        "timeline.range"
    );

    assert_eq!(
        FrameTimeline::new(Vec::new(), 1, 1000, true)
            .expect_err("empty index")
            .code(),
        "source.frame_index"
    );
    assert_eq!(
        FrameTimeline::new(vec![point(0, 1, true)], 0, 1000, true)
            .expect_err("bad time base")
            .code(),
        "source.time_base"
    );
}

#[test]
fn packet_lines_expose_discarded_preroll_and_keyframes() {
    let line = parse_index_line("packet|pts=1024|duration=512|flags=KD|side_data=", true);
    assert!(line.has_timestamp);
    assert_eq!(line.point.pts, 1024);
    assert_eq!(line.point.duration, 512);
    assert!(line.point.key_frame);
    assert!(line.discarded);

    let line = parse_index_line("packet|duration=512|flags=_", true);
    assert!(!line.has_timestamp, "a missing PTS invalidates the index");
    assert!(!line.point.key_frame);

    let line = parse_index_line(
        "frame|best_effort_timestamp=2048|duration=512|key_frame=1",
        false,
    );
    assert!(line.has_timestamp);
    assert_eq!(line.point.pts, 2048);
    assert!(line.point.key_frame);
}

#[test]
fn duplicate_packet_timestamps_fall_back_to_decoded_frames() {
    let unique = sorted_unique_packets(vec![point(2000, 1, true), point(0, 1, true)]);
    let unique = unique.expect("distinct timestamps");
    assert_eq!(unique[0].pts, 0);
    assert_eq!(unique[1].pts, 2000);

    assert!(
        sorted_unique_packets(vec![point(0, 1, true), point(0, 1, false)]).is_none(),
        "a repeated PTS cannot describe display order"
    );
    assert!(sorted_unique_packets(Vec::new()).is_some());
}
