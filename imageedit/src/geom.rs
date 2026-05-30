//! Output-dimension math: rotation swaps, and the bake-size cap.

/// Longest side (px) of the baked output. Slightly above the pipeline's widest
/// target width (2400) for headroom; baking larger is wasted work since the
/// server re-derives ≤2400-wide renditions anyway, and it bounds memory on huge
/// source images.
pub const BAKE_MAX_LONG: u32 = 2560;

/// Output dimensions after `quarters` 90° rotations — odd counts swap width and
/// height.
pub fn rotated_dims(w: u32, h: u32, quarters: u8) -> (u32, u32) {
    if quarters % 2 == 1 {
        (h, w)
    } else {
        (w, h)
    }
}

/// Cap the longest side to `max_long`, preserving aspect ratio; never upscale.
/// Rounds to whole pixels, minimum 1.
pub fn bake_dims(w: f64, h: f64, max_long: u32) -> (u32, u32) {
    let long = w.max(h);
    let scale = if long > max_long as f64 {
        max_long as f64 / long
    } else {
        1.0
    };
    let px = |v: f64| ((v * scale).round() as u32).max(1);
    (px(w), px(h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_swaps_on_odd_quarters() {
        assert_eq!(rotated_dims(800, 600, 0), (800, 600));
        assert_eq!(rotated_dims(800, 600, 1), (600, 800));
        assert_eq!(rotated_dims(800, 600, 2), (800, 600));
        assert_eq!(rotated_dims(800, 600, 3), (600, 800));
    }

    #[test]
    fn bake_caps_longest_side_preserving_ratio() {
        assert_eq!(bake_dims(4000.0, 3000.0, 2560), (2560, 1920));
        assert_eq!(bake_dims(3000.0, 4000.0, 2560), (1920, 2560));
    }

    #[test]
    fn bake_never_upscales() {
        assert_eq!(bake_dims(800.0, 600.0, 2560), (800, 600));
        assert_eq!(bake_dims(2560.0, 2560.0, 2560), (2560, 2560));
    }

    #[test]
    fn bake_rounds_and_clamps_to_min_one() {
        assert_eq!(bake_dims(0.4, 0.4, 2560), (1, 1));
    }
}
