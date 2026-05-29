//! Pipeline test: a synthetic image in → capped derivatives + thumbhash out.

use image::{ImageFormat as ImgFmt, RgbImage};
use std::io::Cursor;
use syle_core::ingest::{ingest, thumbhash_data_url};
use syle_types::ImageFormat;

fn synthetic_png(w: u32, h: u32) -> Vec<u8> {
    let img = RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
    });
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImgFmt::Png).unwrap();
    buf
}

#[test]
fn ingest_produces_capped_derivatives_and_thumbhash() {
    let src = synthetic_png(1200, 800);

    // 2000 is wider than the source: it must be capped out, not upscaled.
    let out = ingest(&src, &[480, 960, 2000]).expect("ingest");

    assert_eq!((out.width, out.height), (1200, 800));
    assert!(!out.thumbhash.is_empty());

    // widths {480, 960} (2000 dropped) × formats {avif, jpeg} = 4
    assert_eq!(out.derivatives.len(), 4);

    for d in &out.derivatives {
        assert!(d.width <= 960, "no upscaling beyond source");
        assert!(!d.bytes.is_empty());
        match d.format {
            ImageFormat::Jpeg => assert_eq!(&d.bytes[0..2], &[0xFF, 0xD8]),
            ImageFormat::Avif => assert_eq!(&d.bytes[4..8], b"ftyp"),
        }
    }

    let avif_widths: Vec<u32> = out
        .derivatives
        .iter()
        .filter(|d| d.format == ImageFormat::Avif)
        .map(|d| d.width)
        .collect();
    assert_eq!(avif_widths, vec![480, 960]);
}

#[test]
fn thumbhash_data_url_decodes_to_a_png_data_uri() {
    let src = synthetic_png(1200, 800);
    let out = ingest(&src, &[480]).expect("ingest");

    let url = thumbhash_data_url(&out.thumbhash).expect("placeholder");
    assert!(url.starts_with("data:image/png;base64,"));
    // Non-trivial payload (a real encoded PNG, not an empty string).
    assert!(url.len() > "data:image/png;base64,".len() + 40);

    // Malformed hex is rejected rather than panicking.
    assert!(thumbhash_data_url("nothex").is_err());
}

#[test]
fn ingest_falls_back_to_source_width_when_all_targets_upscale() {
    let src = synthetic_png(300, 200);
    // Every requested width exceeds the 300px source; rather than emit nothing,
    // the pipeline produces the native size (still no upscaling).
    let out = ingest(&src, &[800, 1600]).expect("ingest");
    assert!(!out.derivatives.is_empty());
    assert!(out.derivatives.iter().all(|d| d.width == 300));
}
