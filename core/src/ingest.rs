//! Pure-Rust ingest pipeline: decode → capped responsive derivatives
//! (AVIF via ravif, JPEG via jpeg-encoder) + ThumbHash placeholder.

use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, ResizeOptions, Resizer};
use image::{DynamicImage, ImageDecoder, ImageReader, Limits};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use syle_types::ImageFormat;

const AVIF_QUALITY: f32 = 68.0;
const AVIF_SPEED: u8 = 6;
const JPEG_QUALITY: u8 = 80;
/// ThumbHash wants a tiny image; longest side clamped to this.
const THUMB_MAX: u32 = 100;
/// Guard against compressed image bombs before allocating their decoded pixel
/// buffer. This still accommodates current high-resolution full-frame cameras.
pub const MAX_INPUT_PIXELS: u64 = 64_000_000;
/// A second, strict guard for pathological panoramas with a narrow pixel count.
pub const MAX_INPUT_DIMENSION: u32 = 16_384;
/// Upper bound passed to decoders for their own working/output allocations.
const MAX_DECODE_ALLOC_BYTES: u64 = 384 * 1024 * 1024;
/// Increment whenever encoding settings or pixel transformations change. Static
/// manifests use it to invalidate otherwise unchanged source images.
pub const INGEST_PIPELINE_VERSION: u32 = 2;

/// Stable SHA-256 content key for immutable derivative names and cache busting.
pub fn content_key(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// One encoded rendition ready to be written to the media store.
#[derive(Debug, Clone)]
pub struct Derivative {
    pub format: ImageFormat,
    pub width: u32,
    pub bytes: Vec<u8>,
}

/// Result of ingesting one source image.
#[derive(Debug, Clone)]
pub struct Ingested {
    pub width: u32,
    pub height: u32,
    /// Hex-encoded ThumbHash, stored verbatim in `photos.thumbhash`.
    pub thumbhash: String,
    pub derivatives: Vec<Derivative>,
}

/// Decode `input`, then for every target width that does not upscale the
/// source, emit an AVIF and a JPEG derivative plus a ThumbHash placeholder.
pub fn ingest(input: &[u8], target_widths: &[u32]) -> anyhow::Result<Ingested> {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_INPUT_DIMENSION);
    limits.max_image_height = Some(MAX_INPUT_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC_BYTES);

    let mut reader = ImageReader::new(Cursor::new(input)).with_guessed_format()?;
    reader.limits(limits);
    let mut decoder = reader.into_decoder()?;
    let (encoded_w, encoded_h) = decoder.dimensions();
    let pixels = u64::from(encoded_w)
        .checked_mul(u64::from(encoded_h))
        .ok_or_else(|| anyhow::anyhow!("image dimensions overflow"))?;
    anyhow::ensure!(
        pixels <= MAX_INPUT_PIXELS,
        "image exceeds decoded pixel limit ({pixels} > {MAX_INPUT_PIXELS})"
    );
    anyhow::ensure!(
        decoder.total_bytes() <= MAX_DECODE_ALLOC_BYTES,
        "image exceeds decoded memory limit"
    );

    // Cameras commonly store landscape pixels plus an EXIF rotation. Apply it
    // once before calculating responsive sizes; the pixel-only encoders below
    // intentionally carry no EXIF, XMP, IPTC, or location metadata forward.
    let orientation = decoder.orientation()?;
    let mut decoded = DynamicImage::from_decoder(decoder)?;
    decoded.apply_orientation(orientation);
    let img = decoded.to_rgba8();
    let (w, h) = img.dimensions();
    let rgba = img.into_raw();

    let mut widths: Vec<u32> = target_widths
        .iter()
        .copied()
        .filter(|&tw| tw > 0 && tw <= w)
        .collect();
    widths.sort_unstable();
    widths.dedup();
    // If every requested width would upscale the source, fall back to the
    // source's native width so callers always get at least one derivative.
    if widths.is_empty() {
        widths.push(w);
    }

    let mut derivatives = Vec::with_capacity(widths.len() * 2);
    for tw in widths {
        let th = ((tw as u64 * h as u64) / w as u64).max(1) as u32;
        let scaled = resize_rgba(&rgba, w, h, tw, th)?;
        derivatives.push(Derivative {
            format: ImageFormat::Avif,
            width: tw,
            bytes: encode_avif(&scaled, tw, th)?,
        });
        derivatives.push(Derivative {
            format: ImageFormat::Jpeg,
            width: tw,
            bytes: encode_jpeg(&scaled, tw, th)?,
        });
    }

    Ok(Ingested {
        width: w,
        height: h,
        thumbhash: thumbhash_hex(&rgba, w, h)?,
        derivatives,
    })
}

/// Box-fit resize of an RGBA8 buffer to exactly `dw x dh`.
fn resize_rgba(rgba: &[u8], w: u32, h: u32, dw: u32, dh: u32) -> anyhow::Result<Vec<u8>> {
    let src = Image::from_vec_u8(w, h, rgba.to_vec(), PixelType::U8x4)?;
    let mut dst = Image::new(dw, dh, PixelType::U8x4);
    Resizer::new().resize(&src, &mut dst, &ResizeOptions::new())?;
    Ok(dst.into_vec())
}

fn encode_avif(rgba: &[u8], w: u32, h: u32) -> anyhow::Result<Vec<u8>> {
    let pixels: Vec<ravif::RGBA8> = rgba
        .chunks_exact(4)
        .map(|p| ravif::RGBA8::new(p[0], p[1], p[2], p[3]))
        .collect();
    let encoded = ravif::Encoder::new()
        .with_quality(AVIF_QUALITY)
        .with_speed(AVIF_SPEED)
        .encode_rgba(ravif::Img::new(pixels.as_slice(), w as usize, h as usize))?;
    Ok(encoded.avif_file)
}

fn encode_jpeg(rgba: &[u8], w: u32, h: u32) -> anyhow::Result<Vec<u8>> {
    let rgb: Vec<u8> = rgba
        .chunks_exact(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    let mut buf = Vec::new();
    jpeg_encoder::Encoder::new(&mut buf, JPEG_QUALITY).encode(
        &rgb,
        w as u16,
        h as u16,
        jpeg_encoder::ColorType::Rgb,
    )?;
    Ok(buf)
}

/// Decode a stored ThumbHash hex string into a tiny PNG `data:` URL — the
/// blur-up placeholder painted before the full image loads. Galleries compute
/// this in Astro (`thumbHashToDataURL`); blog images render through the shared
/// pure renderer, so it's precomputed here (where the image deps live) and
/// embedded as a plain string, keeping `syle-render` dependency-free.
pub fn thumbhash_data_url(hash_hex: &str) -> anyhow::Result<String> {
    use base64::Engine;
    let bytes = hex_to_bytes(hash_hex).ok_or_else(|| anyhow::anyhow!("invalid thumbhash hex"))?;
    let (w, h, rgba) = thumbhash::thumb_hash_to_rgba(&bytes)
        .map_err(|_| anyhow::anyhow!("thumbhash decode failed"))?;
    let img = image::RgbaImage::from_raw(w as u32, h as u32, rgba)
        .ok_or_else(|| anyhow::anyhow!("thumbhash rgba size mismatch"))?;
    let mut png = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    Ok(format!("data:image/png;base64,{b64}"))
}

/// Parse an even-length lowercase/uppercase hex string into bytes.
fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok())
        .collect()
}

fn thumbhash_hex(rgba: &[u8], w: u32, h: u32) -> anyhow::Result<String> {
    let (tw, th) = if w >= h {
        (
            THUMB_MAX.min(w),
            ((THUMB_MAX as u64 * h as u64) / w as u64).max(1) as u32,
        )
    } else {
        (
            ((THUMB_MAX as u64 * w as u64) / h as u64).max(1) as u32,
            THUMB_MAX.min(h),
        )
    };
    let small = resize_rgba(rgba, w, h, tw, th)?;
    let hash = thumbhash::rgba_to_thumb_hash(tw as usize, th as usize, &small);
    Ok(hash.iter().map(|b| format!("{b:02x}")).collect())
}
