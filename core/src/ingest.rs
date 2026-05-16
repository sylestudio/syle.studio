//! Pure-Rust ingest pipeline: decode → capped responsive derivatives
//! (AVIF via ravif, JPEG via jpeg-encoder) + ThumbHash placeholder.

use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, ResizeOptions, Resizer};
use syle_types::ImageFormat;

const AVIF_QUALITY: f32 = 68.0;
const AVIF_SPEED: u8 = 6;
const JPEG_QUALITY: u8 = 80;
/// ThumbHash wants a tiny image; longest side clamped to this.
const THUMB_MAX: u32 = 100;

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
    let img = image::load_from_memory(input)?.to_rgba8();
    let (w, h) = img.dimensions();
    let rgba = img.into_raw();

    let mut widths: Vec<u32> = target_widths
        .iter()
        .copied()
        .filter(|&tw| tw > 0 && tw <= w)
        .collect();
    widths.sort_unstable();
    widths.dedup();

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

fn thumbhash_hex(rgba: &[u8], w: u32, h: u32) -> anyhow::Result<String> {
    let (tw, th) = if w >= h {
        (THUMB_MAX.min(w), ((THUMB_MAX as u64 * h as u64) / w as u64).max(1) as u32)
    } else {
        (((THUMB_MAX as u64 * w as u64) / h as u64).max(1) as u32, THUMB_MAX.min(h))
    };
    let small = resize_rgba(rgba, w, h, tw, th)?;
    let hash = thumbhash::rgba_to_thumb_hash(tw as usize, th as usize, &small);
    Ok(hash.iter().map(|b| format!("{b:02x}")).collect())
}
