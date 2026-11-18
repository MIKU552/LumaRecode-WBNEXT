//! Developing a camera RAW frame into HLG.
//!
//! The host demuxes the RawVideo packet and hands the geometry, the white balance matrix
//! and the calibration commands; the chain below is the rule, spelled once so the two
//! hosts cannot develop the same shot differently.

use crate::domain::RawGeometry;

/// Working scale for the calibration matrix, kept integral so the `sendcmd` values stay exact.
const MATRIX_SCALE: u32 = 1024;

/// The `-vf` chain that turns a Bayer frame into scene-linear BT.2020 and then HLG.
pub fn development_filter(geometry: &RawGeometry, commands: &str, preview_size: u32) -> String {
    let crop = format!(
        "crop={}:{}:{}:{}:exact=1",
        geometry
            .width
            .saturating_sub(geometry.crop[0] + geometry.crop[1]),
        geometry
            .height
            .saturating_sub(geometry.crop[2] + geometry.crop[3]),
        geometry.crop[0],
        geometry.crop[2]
    );
    let mut filters = vec!["format=gbrapf32le".to_owned()];
    if preview_size > 0 {
        // Demosaic first: cropping the Bayer mosaic would change its phase. Calibration and
        // highlight rendering then operate on the viewport.
        filters.push(crop.clone());
        filters.push(scale_filter(preview_size));
    }
    filters.push(format!("sendcmd=f={commands}"));
    filters.push(format!(
        "colorchannelmixer@raw={}:format=gbrpf32le",
        coefficients(&geometry.matrix).join(":")
    ));
    filters.push(format!(
        "geq=r={}:g={}:b={}",
        render(0),
        render(1),
        render(2)
    ));
    if preview_size == 0 {
        filters.push(crop);
    }
    filters.push(
        "setparams=range=full:color_primaries=bt2020:color_trc=arib-std-b67:colorspace=gbr"
            .to_owned(),
    );
    filters.push("zscale=matrix=bt2020nc:range=limited,format=yuv444p16le".to_owned());
    filters.join(",")
}

/// Fits the frame inside a square viewport without changing its aspect or parity.
fn scale_filter(maximum: u32) -> String {
    format!(
        "scale=w='min(iw,{maximum})':h='min(ih,{maximum})':force_original_aspect_ratio=decrease:force_divisible_by=2"
    )
}

/// Names the white balance matrix the way `colorchannelmixer` expects it.
fn coefficients(matrix: &[f64; 12]) -> Vec<String> {
    let mut pairs = Vec::with_capacity(12);
    for row in 0..3 {
        for column in 0..4 {
            let channel = ["r", "g", "b"][row];
            let target = ["r", "g", "b", "a"][column];
            pairs.push(format!(
                "{channel}{target}={}",
                number(matrix[row * 4 + column])
            ));
        }
    }
    pairs
}

/// Trims the exponent notation FFmpeg's duration parser rejects.
fn number(value: f64) -> String {
    let text = format!("{value:.17}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() || trimmed == "-" {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Scene-linear BT.2020 with negatives compressed to neutral, then rolled into HLG.
fn render(channel: u32) -> String {
    let common = format!(
        "st(0,r(X,Y)*{MATRIX_SCALE});st(1,g(X,Y)*{MATRIX_SCALE});st(2,b(X,Y)*{MATRIX_SCALE});\
         st(3,max(0,0.2627*ld(0)+0.6780*ld(1)+0.0593*ld(2)));\
         st(4,min(ld(0),min(ld(1),ld(2))));\
         st(5,if(lt(ld(4),0),if(gt(ld(3),0),ld(3)/(ld(3)-ld(4)),0),1));\
         st(0,max(0,ld(3)+ld(5)*(ld(0)-ld(3)))*0.2674074074074074);\
         st(1,max(0,ld(3)+ld(5)*(ld(1)-ld(3)))*0.2674074074074074);\
         st(2,max(0,ld(3)+ld(5)*(ld(2)-ld(3)))*0.2674074074074074);\
         st(3,max(ld(0),max(ld(1),ld(2))));\
         st(4,if(lte(ld(3),0.45),1,(0.45+0.55*(1-exp(-(ld(3)-0.45)/0.55)))/ld(3)));\
         st(5,ld({channel})*ld(4));"
    );
    format!(
        "'{common}if(lte(ld(5),1/12),sqrt(3*ld(5)),0.17883277*log(12*ld(5)-0.28466892)+0.55991073)'"
    )
}
