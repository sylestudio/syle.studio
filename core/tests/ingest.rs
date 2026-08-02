//! Pipeline test: a synthetic image in → capped derivatives + thumbhash out.

use image::{ImageFormat as ImgFmt, RgbImage};
use std::io::Cursor;
use syle_core::ingest::{ingest, thumbhash_data_url, MAX_INPUT_DIMENSION};
use syle_types::ImageFormat;

fn synthetic_png(w: u32, h: u32) -> Vec<u8> {
    let img = RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
    });
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImgFmt::Png)
        .unwrap();
    buf
}

fn synthetic_webp(w: u32, h: u32) -> Vec<u8> {
    let img = RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 64])
    });
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImgFmt::WebP)
        .unwrap();
    buf
}

fn synthetic_jpeg(w: u32, h: u32) -> Vec<u8> {
    let img = RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x * 40) as u8, (y * 80) as u8, 120])
    });
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImgFmt::Jpeg)
        .unwrap();
    buf
}

/// Add a minimal EXIF APP1 segment containing only TIFF orientation.
fn with_exif_orientation(jpeg: &[u8], orientation: u16) -> Vec<u8> {
    assert_eq!(&jpeg[..2], &[0xff, 0xd8]);
    let mut tiff = vec![
        b'I',
        b'I',
        42,
        0, // little-endian TIFF header
        8,
        0,
        0,
        0, // first IFD offset
        1,
        0, // one directory entry
        0x12,
        0x01, // orientation tag
        3,
        0, // SHORT
        1,
        0,
        0,
        0, // one value
        orientation as u8,
        (orientation >> 8) as u8,
        0,
        0,
        0,
        0,
        0,
        0, // no next IFD
    ];
    let mut payload = b"Exif\0\0".to_vec();
    payload.append(&mut tiff);
    let segment_len = u16::try_from(payload.len() + 2).unwrap();

    let mut out = Vec::with_capacity(jpeg.len() + payload.len() + 4);
    out.extend_from_slice(&jpeg[..2]);
    out.extend_from_slice(&[0xff, 0xe1]);
    out.extend_from_slice(&segment_len.to_be_bytes());
    out.extend_from_slice(&payload);
    out.extend_from_slice(&jpeg[2..]);
    out
}

fn patch_jpeg_dimensions(jpeg: &mut [u8], width: u16, height: u16) {
    let sof = jpeg
        .windows(2)
        .position(|marker| marker == [0xff, 0xc0])
        .expect("baseline JPEG has SOF0");
    jpeg[sof + 5..sof + 7].copy_from_slice(&height.to_be_bytes());
    jpeg[sof + 7..sof + 9].copy_from_slice(&width.to_be_bytes());
}

#[test]
fn ingest_decodes_webp_input() {
    // Browsers and modern image exports routinely hand us WebP. The pipeline
    // must decode it (not reject it with a 400) just like JPEG/PNG.
    let src = synthetic_webp(640, 480);
    let out = ingest(&src, &[320, 640]).expect("ingest webp");
    assert_eq!((out.width, out.height), (640, 480));
    assert!(
        !out.derivatives.is_empty(),
        "produces derivatives from webp"
    );
    assert!(!out.thumbhash.is_empty());
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

#[test]
fn ingest_applies_exif_orientation_and_strips_metadata() {
    let src = with_exif_orientation(&synthetic_jpeg(3, 2), 6);
    assert!(src.windows(6).any(|bytes| bytes == b"Exif\0\0"));

    let out = ingest(&src, &[2]).expect("oriented ingest");
    assert_eq!((out.width, out.height), (2, 3));

    for derivative in &out.derivatives {
        assert!(
            !derivative
                .bytes
                .windows(6)
                .any(|bytes| bytes == b"Exif\0\0"),
            "derived image must not retain EXIF metadata"
        );
    }
}

#[test]
fn ingest_rejects_oversized_dimensions_before_pixel_decode() {
    let mut src = synthetic_jpeg(2, 2);
    let oversized = u16::try_from(MAX_INPUT_DIMENSION + 1).unwrap();
    patch_jpeg_dimensions(&mut src, oversized, oversized);

    let err = ingest(&src, &[480]).expect_err("oversized dimensions must fail");
    let message = format!("{err:#}");
    assert!(
        message.contains("image dimensions") || message.contains("limit"),
        "unexpected error: {message}"
    );
}
