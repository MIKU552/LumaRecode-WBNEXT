//! The frame box accepts a frame number or a time; the grammar is checked here so both
//! hosts read `00:01:23.12` the same way.

use luma_core::timeline::parse_time_input;

#[test]
fn a_clock_time_reads_as_seconds() {
    assert_eq!(parse_time_input("00:01:23"), Some((83.0, 0)));
    assert_eq!(parse_time_input("1:05"), Some((65.0, 0)));
}

#[test]
fn the_group_after_the_dot_counts_frames() {
    assert_eq!(parse_time_input("00:01:23.12"), Some((83.0, 12)));
    assert_eq!(parse_time_input("0:00:02.0"), Some((2.0, 0)));
}

#[test]
fn bare_seconds_keep_their_decimal() {
    assert_eq!(parse_time_input("12.5"), Some((12.5, 0)));
}

#[test]
fn a_bare_number_is_a_frame_not_a_time() {
    assert_eq!(parse_time_input("120"), None);
    assert_eq!(parse_time_input("  7  "), None);
}

#[test]
fn nonsense_is_refused() {
    assert_eq!(parse_time_input(""), None);
    assert_eq!(parse_time_input("abc"), None);
    assert_eq!(parse_time_input("1:xx"), None);
    assert_eq!(parse_time_input("00:01:23.1a"), None);
    assert_eq!(parse_time_input("-5"), None);
}
