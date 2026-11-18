//! The HDR10+ reader is checked against payloads composed bit by bit, so a field that
//! moves by one bit fails here instead of producing a wrong `dhdr10-info` document.

use luma_core::sei::t35_line;

/// MSB-first writer that mirrors the reader.
#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    used: u32,
}

impl Bits {
    fn write(&mut self, value: u32, count: u32) -> &mut Self {
        for bit in (0..count).rev() {
            if self.used % 8 == 0 {
                self.bytes.push(0);
            }
            let one = (value >> bit) & 1 == 1;
            if one {
                let index = self.bytes.len() - 1;
                self.bytes[index] |= 0x80 >> (self.used % 8);
            }
            self.used += 1;
        }
        self
    }

    /// Pads to the byte boundary with the zeros the standard requires.
    fn pad(&mut self) -> &mut Self {
        while self.used % 8 != 0 {
            self.write(0, 1);
        }
        self
    }
}

fn sample_payload() -> Vec<u8> {
    let mut bits = Bits::default();
    bits.write(4, 8) // application version
        .write(1, 2) // one window
        .write(1000, 27) // targeted system display maximum luminance
        .write(0, 1)
        .write(1000, 17)
        .write(2000, 17)
        .write(3000, 17)
        .write(500, 17) // average maxrgb
        .write(0, 4) // no percentiles
        .write(100, 10) // fraction bright pixels
        .write(0, 1)
        .write(0, 1) // no bezier curve
        .write(0, 1)
        .pad();

    let mut payload = vec![0xb5, 0x00, 0x3c, 0x00, 0x01, 0x04];
    payload.extend_from_slice(&bits.bytes);
    payload
}

#[test]
fn every_field_lands_where_ffprobe_puts_it() {
    let line = t35_line(&sample_payload()).expect("payload parses");
    assert_eq!(
        line,
        "frame|side_data_type=HDR Dynamic Metadata SMPTE2094-40 (HDR10+)\
         |application version=4|num_windows=1|targeted_system_display_maximum_luminance=1000\
         |maxscl=1000/100000|maxscl=2000/100000|maxscl=3000/100000\
         |average_maxrgb=500/100000|num_distribution_maxrgb_percentiles=0\
         |fraction_bright_pixels=100/1000"
    );
}

#[test]
fn a_payload_with_the_wrong_header_is_rejected() {
    let mut payload = sample_payload();
    payload[5] = 0x08;
    assert_eq!(
        t35_line(&payload).expect_err("wrong application id").code(),
        "metadata.hdr10plus"
    );
}

#[test]
fn a_truncated_payload_is_rejected() {
    let payload = sample_payload();
    assert_eq!(
        t35_line(&payload[..payload.len() - 4])
            .expect_err("truncated")
            .code(),
        "metadata.hdr10plus"
    );
}

#[test]
fn two_windows_are_rejected() {
    let mut bits = Bits::default();
    bits.write(4, 8).write(2, 2).pad();
    let mut payload = vec![0xb5, 0x00, 0x3c, 0x00, 0x01, 0x04];
    payload.extend_from_slice(&bits.bytes);
    assert_eq!(
        t35_line(&payload).expect_err("one window only").code(),
        "metadata.hdr10plus"
    );
}
