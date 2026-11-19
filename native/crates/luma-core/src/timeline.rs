//! Frame-accurate timeline built from real presentation timestamps.
//!
//! Nothing here multiplies an average frame rate: VFR sources, B-frame
//! reordering, repeated or missing timestamps and MP4 edit lists all need the
//! timestamps ffprobe/AVFoundation actually report.
//!
//! Ranges are zero-based and end-exclusive; the GUI keeps its own one-based,
//! end-inclusive display and converts at the boundary.

use crate::error::{CoreError, CoreResult, codes};

/// One displayed frame: source ticks plus random-access evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramePoint {
    pub pts: i64,
    /// Declared duration in ticks; zero when the source omits it.
    pub duration: i64,
    pub key_frame: bool,
}

/// Zero-based, end-exclusive selection. `end = None` means "to the last frame".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRange {
    pub start: usize,
    pub end: Option<usize>,
}

impl FrameRange {
    /// Selects the whole source.
    pub const fn all() -> Self {
        Self {
            start: 0,
            end: None,
        }
    }

    /// Selects from `start` to the end.
    pub const fn from(start: usize) -> Self {
        Self { start, end: None }
    }

    /// Selects `[start, end)`.
    pub const fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end: Some(end),
        }
    }
}

/// A selection resolved to source seconds and a safe preceding seek point.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameWindow {
    pub start: usize,
    pub end: usize,
    pub seek_frame: usize,
    pub seek_seconds: f64,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

impl FrameWindow {
    pub const fn count(&self) -> usize {
        self.end - self.start
    }

    pub fn duration(&self) -> f64 {
        self.end_seconds - self.start_seconds
    }
}

/// A display-order frame index with its stream time base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameTimeline {
    frames: Vec<FramePoint>,
    numerator: i64,
    denominator: i64,
    packet_indexed: bool,
}

impl FrameTimeline {
    /// Builds a timeline. Packet-indexed timelines come from compressed packet
    /// timestamps; decoded timelines come from presentation-order frames.
    pub fn new(
        frames: Vec<FramePoint>,
        numerator: i64,
        denominator: i64,
        packet_indexed: bool,
    ) -> CoreResult<Self> {
        if numerator <= 0 || denominator <= 0 {
            return Err(CoreError::with_detail(
                codes::SOURCE_TIME_BASE,
                format!("{numerator}/{denominator}"),
            ));
        }
        if frames.is_empty() {
            return Err(CoreError::new(codes::SOURCE_FRAME_INDEX));
        }
        Ok(Self {
            frames,
            numerator,
            denominator,
            packet_indexed,
        })
    }

    pub fn frames(&self) -> &[FramePoint] {
        &self.frames
    }

    pub fn count(&self) -> usize {
        self.frames.len()
    }

    pub const fn time_base(&self) -> (i64, i64) {
        (self.numerator, self.denominator)
    }

    /// `true` when indices came from packet timestamps rather than decoding.
    pub const fn is_packet_indexed(&self) -> bool {
        self.packet_indexed
    }

    fn ticks_to_seconds(&self, ticks: i64) -> f64 {
        ticks as f64 * self.numerator as f64 / self.denominator as f64
    }

    /// Absolute source time of a frame.
    pub fn seconds(&self, frame: usize) -> f64 {
        self.ticks_to_seconds(self.frames[frame].pts)
    }

    /// Time relative to the first displayed frame.
    pub fn position(&self, frame: usize) -> f64 {
        self.seconds(frame) - self.seconds(0)
    }

    /// End boundary of a frame; `count` yields the end of the last frame.
    pub fn boundary_seconds(&self, frame: usize) -> f64 {
        if frame < self.count() {
            return self.seconds(frame);
        }
        let last = self.frames[self.count() - 1];
        let duration = if last.duration > 0 {
            last.duration
        } else if self.count() > 1 {
            (last.pts - self.frames[self.count() - 2].pts).max(1)
        } else {
            1
        };
        self.ticks_to_seconds(last.pts.saturating_add(duration))
    }

    /// Total displayed duration including the last frame's own duration.
    pub fn duration(&self) -> f64 {
        self.boundary_seconds(self.count()) - self.seconds(0)
    }

    /// First frame at or after a display position.
    pub fn at_or_after(&self, position: f64) -> usize {
        let mut lower = 0;
        let mut upper = self.count();
        while lower < upper {
            let middle = lower + (upper - lower) / 2;
            if self.position(middle) + 1e-9 < position {
                lower = middle + 1;
            } else {
                upper = middle;
            }
        }
        lower
    }

    /// Resolves a range to times and a keyframe seek target.
    ///
    /// Decoded timelines have no random-access evidence, so they always seek to
    /// the start of the file and decode forward.
    pub fn resolve(&self, range: FrameRange) -> CoreResult<FrameWindow> {
        let end = range.end.unwrap_or(self.count());
        if range.start >= end || end > self.count() {
            return Err(CoreError::with_detail(
                codes::TIMELINE_RANGE,
                format!("{}-{end}/{}", range.start, self.count()),
            ));
        }
        let mut seek = 0;
        if self.packet_indexed {
            for index in (0..=range.start).rev() {
                // A negative PTS cannot be sought to; keep looking backwards.
                if self.frames[index].key_frame && self.seconds(index) >= 0.0 {
                    seek = index;
                    break;
                }
            }
        }
        Ok(FrameWindow {
            start: range.start,
            end,
            seek_frame: seek,
            seek_seconds: if seek == 0 { 0.0 } else { self.seconds(seek) },
            start_seconds: self.seconds(range.start),
            end_seconds: self.boundary_seconds(end),
        })
    }
}

/// One parsed line of `ffprobe -show_packets`/`-show_frames` compact output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexLine {
    pub point: FramePoint,
    /// `false` when the timestamp field was missing or unparsable.
    pub has_timestamp: bool,
    /// Packet marked discarded (MP4 preroll); excluded from the index.
    pub discarded: bool,
}

/// Parses `key=value|key=value` without allocating a map per frame.
pub fn parse_index_line(line: &str, packets: bool) -> IndexLine {
    let mut pts = 0;
    let mut duration = 0;
    let mut has_timestamp = false;
    let mut key_frame = false;
    let mut discarded = false;
    for field in line.split('|') {
        let Some(equals) = field.find('=') else {
            continue;
        };
        if equals == 0 {
            continue;
        }
        let (name, raw) = field.split_at(equals);
        let value = &raw[1..];
        if name
            == if packets {
                "pts"
            } else {
                "best_effort_timestamp"
            }
        {
            if let Ok(parsed) = value.trim().parse::<i64>() {
                pts = parsed;
                has_timestamp = true;
            }
        } else if name == "duration" {
            duration = value.trim().parse::<i64>().unwrap_or(0);
        } else if packets && name == "flags" {
            discarded = value.contains('D');
            key_frame = value.contains('K');
        } else if !packets && name == "key_frame" {
            key_frame = value == "1";
        }
    }
    IndexLine {
        point: FramePoint {
            pts,
            duration,
            key_frame,
        },
        has_timestamp,
        discarded,
    }
}

/// Sorts packet timestamps and rejects ambiguous duplicates.
///
/// A repeated PTS means a packet index cannot describe display order, so the
/// caller must fall back to decoded frames.
pub fn sorted_unique_packets(mut points: Vec<FramePoint>) -> Option<Vec<FramePoint>> {
    points.sort_by_key(|point| point.pts);
    points
        .windows(2)
        .all(|pair| pair[0].pts != pair[1].pts)
        .then_some(points)
}

/// Reads a time the user typed, as `h:mm:ss`, `h:mm:ss.ff` or decimal seconds.
///
/// The group after the dot counts frames when the text is clock-shaped and seconds when it
/// is not, which is how people write both. A bare number is not a time — the frame box
/// treats it as a frame number — so it returns `None` here.
pub fn parse_time_input(text: &str) -> Option<(f64, u32)> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !trimmed.contains(':') {
        let seconds: f64 = trimmed.parse().ok()?;
        return (seconds.is_finite() && seconds >= 0.0).then_some((seconds, 0));
    }

    let (clock, frames) = match trimmed.rsplit_once('.') {
        Some((clock, frames)) => (clock, Some(frames)),
        None => (trimmed, None),
    };
    let mut seconds = 0.0;
    let mut groups = 0;
    for group in clock.split(':') {
        let value: f64 = group.parse().ok()?;
        if !(value.is_finite() && value >= 0.0) {
            return None;
        }
        seconds = seconds * 60.0 + value;
        groups += 1;
    }
    if groups < 2 {
        return None;
    }
    let offset = match frames {
        None => 0.0,
        Some(frames) if !frames.is_empty() && frames.chars().all(|c| c.is_ascii_digit()) => {
            frames.parse().ok()?
        }
        Some(_) => return None,
    };
    Some((seconds, offset as u32))
}
