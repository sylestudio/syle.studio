//! Pipeline test: a synthetic image in → capped derivatives + thumbhash out.

use image::{ImageFormat as ImgFmt, RgbImage};
use std::io::Cursor;
use syle_core::ingest::ingest;
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
