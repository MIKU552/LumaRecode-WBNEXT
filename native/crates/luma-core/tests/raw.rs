//! The RAW chain is checked as a string: the two hosts must develop the same shot the same
//! way, and a filter that moves or loses a denominator would show up here.

use luma_core::domain::RawGeometry;
use luma_core::raw::development_filter;

fn geometry() -> RawGeometry {
    RawGeometry {
        width: 1000,
        height: 800,
        crop: [10, 6, 4, 2],
        matrix: [
            1.5, 0.0, -0.5, 0.0, //
            0.0, 1.25, -0.25, 0.0, //
            -0.125, 0.0, 1.125, 0.0,
        ],
    }
}

#[test]
fn the_chain_demosaics_balances_and_rolls_into_hlg() {
    let chain = development_filter(&geometry(), "0.5 setconfig rbright", 0);

    assert!(chain.starts_with("format=gbrapf32le,"));
    assert!(chain.contains("sendcmd=f=0.5 setconfig rbright"));
    assert!(chain.contains("colorchannelmixer@raw=rr=1.5:rg=0:rb=-0.5:ra=0:gr=0:gg=1.25:gb=-0.25:ga=0:br=-0.125:bg=0:bb=1.125:ba=0"));
    assert!(chain.contains("geq=r='st(0,r(X,Y)*1024)"));
    // The crop follows the render when there is no preview to fit.
    assert!(chain.contains(",crop=984:794:10:4:exact=1,"));
    assert!(chain.ends_with(
        "setparams=range=full:color_primaries=bt2020:color_trc=arib-std-b67:colorspace=gbr,\
         zscale=matrix=bt2020nc:range=limited,format=yuv444p16le"
    ));
}

#[test]
fn a_preview_crops_before_scaling_so_the_mosaic_keeps_its_phase() {
    let chain = development_filter(&geometry(), "0 setconfig rbright", 1024);

    let crop = chain.find("crop=984:794:10:4:exact=1").expect("crop");
    let scale = chain.find("scale=w='min(iw,1024)'").expect("scale");
    assert!(crop < scale, "demosaic first, then fit the viewport");
    assert!(chain.ends_with("format=yuv444p16le"));
    // Only the preview path puts the crop in front of the render.
    assert_eq!(chain.matches("crop=").count(), 1);
}
