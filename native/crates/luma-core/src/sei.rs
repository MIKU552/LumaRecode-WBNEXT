//! HDR10+ dynamic metadata, read from the SEI payload a stream carries.
//!
//! The host extracts the payload — it owns the file and the demuxer — and the core turns
//! the bits into the same pipe-delimited line FFprobe writes, so both hosts build the
//! `dhdr10-info` document from one reader.

use crate::error::{CoreError, CoreResult, codes};

/// Bytes every HEVC HDR10+ payload starts with: the country code, terminal provider code,
/// provider oriented code and application identifier.
const T35_PREFIX: [u8; 6] = [0xb5, 0x00, 0x3c, 0x00, 0x01, 0x04];

/// One frame's metadata, spelled the way FFprobe's compact writer spells it.
pub fn t35_line(payload: &[u8]) -> CoreResult<String> {
    if !payload.starts_with(&T35_PREFIX) {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }

    let mut bits = Bits::new(&payload[T35_PREFIX.len()..]);
    let mut line = String::from("frame|side_data_type=HDR Dynamic Metadata SMPTE2094-40 (HDR10+)");
    let field = |line: &mut String, key: &str, value: u32, scale: Option<u32>| {
        line.push('|');
        line.push_str(key);
        line.push('=');
        line.push_str(&value.to_string());
        if let Some(scale) = scale {
            line.push('/');
            line.push_str(&scale.to_string());
        }
    };

    field(&mut line, "application version", bits.read(8)?, None);
    let windows = bits.read(2)?;
    if windows != 1 {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }
    field(&mut line, "num_windows", windows, None);
    field(
        &mut line,
        "targeted_system_display_maximum_luminance",
        bits.read(27)?,
        None,
    );
    if bits.read(1)? != 0 {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }
    for _ in 0..3 {
        field(&mut line, "maxscl", bits.read(17)?, Some(100_000));
    }
    field(&mut line, "average_maxrgb", bits.read(17)?, Some(100_000));
    let count = bits.read(4)?;
    field(
        &mut line,
        "num_distribution_maxrgb_percentiles",
        count,
        None,
    );
    for _ in 0..count {
        field(
            &mut line,
            "distribution_maxrgb_percentage",
            bits.read(7)?,
            None,
        );
        field(
            &mut line,
            "distribution_maxrgb_percentile",
            bits.read(17)?,
            Some(100_000),
        );
    }
    field(
        &mut line,
        "fraction_bright_pixels",
        bits.read(10)?,
        Some(1000),
    );
    if bits.read(1)? != 0 {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }
    if bits.read(1)? != 0 {
        field(&mut line, "knee_point_x", bits.read(12)?, Some(4095));
        field(&mut line, "knee_point_y", bits.read(12)?, Some(4095));
        let anchors = bits.read(4)?;
        field(&mut line, "num_bezier_curve_anchors", anchors, None);
        for _ in 0..anchors {
            field(
                &mut line,
                "bezier_curve_anchors",
                bits.read(10)?,
                Some(1023),
            );
        }
    }
    if bits.read(1)? != 0 {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }
    // Only the trailing-bit padding may be left, and it has to be zeros.
    while bits.remaining() > 0 {
        if bits.read(1)? != 0 {
            return Err(CoreError::new(codes::METADATA_HDR10PLUS));
        }
    }

    Ok(line)
}

/// MSB-first bit reader over the payload that follows the six fixed bytes.
struct Bits<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Bits<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() * 8 - self.position
    }

    fn read(&mut self, count: u32) -> CoreResult<u32> {
        if count as usize > self.remaining() {
            return Err(CoreError::new(codes::METADATA_HDR10PLUS));
        }
        let mut value = 0u32;
        for _ in 0..count {
            let byte = self.bytes[self.position / 8];
            value = (value << 1) | u32::from(byte >> (7 - self.position % 8) & 1);
            self.position += 1;
        }
        Ok(value)
    }
}
