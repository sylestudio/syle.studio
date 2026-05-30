//! Crop-rectangle geometry in source-image pixel space. Pure: the admin canvas
//! glue maps display ↔ source coordinates and the crop overlay calls into these.

/// Smallest allowed crop side, in source pixels.
pub const MIN: f64 = 16.0;

/// A crop rectangle in source pixels. After [`CropRect::clamp`] it is guaranteed
/// to lie fully inside the image and be at least [`MIN`] on each side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Which grip is being dragged. `Move` translates the whole rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    N,
    S,
    E,
    W,
    Ne,
    Nw,
    Se,
    Sw,
    Move,
}

impl CropRect {
    /// The whole image as a crop.
    pub fn full(w: f64, h: f64) -> Self {
        Self { x: 0.0, y: 0.0, w, h }
    }

    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    /// Clamp size into `[MIN, img]` then position so the rect lies fully inside
    /// `0..img_w × 0..img_h`.
    pub fn clamp(self, img_w: f64, img_h: f64) -> Self {
        let w = self.w.clamp(MIN, img_w);
        let h = self.h.clamp(MIN, img_h);
        Self {
            x: self.x.clamp(0.0, img_w - w),
            y: self.y.clamp(0.0, img_h - h),
            w,
            h,
        }
    }

    /// Translate by `(dx, dy)`, keeping size, clamped inside the image.
    pub fn translate(self, dx: f64, dy: f64, img_w: f64, img_h: f64) -> Self {
        Self { x: self.x + dx, y: self.y + dy, ..self }.clamp(img_w, img_h)
    }

    /// Fit aspect ratio `w/h` inside the current rect, keeping the center. `None`
    /// leaves the rect unchanged. Result is clamped to the image.
    pub fn with_ratio(self, ratio: Option<f64>, img_w: f64, img_h: f64) -> Self {
        let Some(ratio) = ratio else { return self };
        let (cx, cy) = (self.x + self.w / 2.0, self.y + self.h / 2.0);
        let (w, h) = if self.w / self.h > ratio {
            (self.h * ratio, self.h) // too wide → trim width
        } else {
            (self.w, self.w / ratio) // too tall → trim height
        };
        Self { x: cx - w / 2.0, y: cy - h / 2.0, w, h }.clamp(img_w, img_h)
    }

    /// Resize by dragging `handle` by `(dx, dy)` source px. Edges/corners move the
    /// corresponding sides; `Move` translates. `ratio` (w/h) locks aspect when set
    /// — corners and the E/W edges are width-primary, the N/S edges height-primary.
    /// Result is min-size enforced and clamped to the image.
    pub fn drag(
        self,
        handle: Handle,
        dx: f64,
        dy: f64,
        ratio: Option<f64>,
        img_w: f64,
        img_h: f64,
    ) -> Self {
        use Handle::*;
        if handle == Move {
            return self.translate(dx, dy, img_w, img_h);
        }
        let (mut l, mut t, mut r, mut b) = (self.x, self.y, self.right(), self.bottom());
        if matches!(handle, W | Nw | Sw) {
            l = (l + dx).clamp(0.0, r - MIN);
        }
        if matches!(handle, E | Ne | Se) {
            r = (r + dx).clamp(l + MIN, img_w);
        }
        if matches!(handle, N | Nw | Ne) {
            t = (t + dy).clamp(0.0, b - MIN);
        }
        if matches!(handle, S | Sw | Se) {
            b = (b + dy).clamp(t + MIN, img_h);
        }
        let free = Self { x: l, y: t, w: r - l, h: b - t };
        match ratio {
            Some(ratio) => free.enforce_ratio(handle, ratio).clamp(img_w, img_h),
            None => free.clamp(img_w, img_h),
        }
    }

    /// Re-fit `self` to aspect ratio `w/h` for a ratio-locked `handle`: the N/S
    /// edges are height-primary (width re-centered), the E/W edges and all corners
    /// are width-primary. Corners anchor the corner opposite the dragged one.
    fn enforce_ratio(self, handle: Handle, ratio: f64) -> Self {
        use Handle::*;
        match handle {
            N | S => {
                let w = self.h * ratio;
                let cx = self.x + self.w / 2.0;
                Self { x: cx - w / 2.0, y: self.y, w, h: self.h }
            }
            E | W => {
                let h = self.w / ratio;
                let cy = self.y + self.h / 2.0;
                Self { x: self.x, y: cy - h / 2.0, w: self.w, h }
            }
            Ne | Nw | Se | Sw => {
                let w = self.w;
                let h = w / ratio;
                let (r, b) = (self.right(), self.bottom());
                let (x, y) = match handle {
                    Se => (self.x, self.y),
                    Sw => (r - w, self.y),
                    Ne => (self.x, b - h),
                    _ => (r - w, b - h), // Nw
                };
                Self { x, y, w, h }
            }
            Move => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f64, y: f64, w: f64, h: f64) -> CropRect {
        CropRect { x, y, w, h }
    }

    // --- clamp ---------------------------------------------------------------

    #[test]
    fn clamp_pulls_negative_origin_inside() {
        assert_eq!(r(-10.0, -10.0, 50.0, 50.0).clamp(100.0, 100.0), r(0.0, 0.0, 50.0, 50.0));
    }

    #[test]
    fn clamp_shrinks_oversized_width_to_image() {
        assert_eq!(r(0.0, 0.0, 200.0, 50.0).clamp(100.0, 100.0), r(0.0, 0.0, 100.0, 50.0));
    }

    #[test]
    fn clamp_shifts_rect_left_when_it_overflows_right() {
        assert_eq!(r(90.0, 0.0, 50.0, 50.0).clamp(100.0, 100.0), r(50.0, 0.0, 50.0, 50.0));
    }

    #[test]
    fn clamp_enforces_minimum_side() {
        assert_eq!(r(0.0, 0.0, 4.0, 4.0).clamp(100.0, 100.0), r(0.0, 0.0, MIN, MIN));
    }

    // --- translate -----------------------------------------------------------

    #[test]
    fn translate_moves_and_keeps_size() {
        assert_eq!(r(10.0, 10.0, 20.0, 20.0).translate(5.0, -5.0, 100.0, 100.0), r(15.0, 5.0, 20.0, 20.0));
    }

    #[test]
    fn translate_clamps_at_far_edge() {
        assert_eq!(r(90.0, 0.0, 20.0, 20.0).translate(20.0, 0.0, 100.0, 100.0), r(80.0, 0.0, 20.0, 20.0));
    }

    // --- with_ratio ----------------------------------------------------------

    #[test]
    fn with_ratio_none_is_identity() {
        assert_eq!(r(3.0, 4.0, 5.0, 6.0).with_ratio(None, 100.0, 100.0), r(3.0, 4.0, 5.0, 6.0));
    }

    #[test]
    fn with_ratio_reduces_height_for_wide_target() {
        // square → 2:1, center preserved, width kept, height halved.
        assert_eq!(r(0.0, 0.0, 100.0, 100.0).with_ratio(Some(2.0), 1000.0, 1000.0), r(0.0, 25.0, 100.0, 50.0));
    }

    #[test]
    fn with_ratio_reduces_width_for_square_target() {
        assert_eq!(r(0.0, 0.0, 100.0, 50.0).with_ratio(Some(1.0), 1000.0, 1000.0), r(25.0, 0.0, 50.0, 50.0));
    }

    // --- drag (free) ---------------------------------------------------------

    #[test]
    fn drag_corner_grows_both_sides() {
        assert_eq!(r(0.0, 0.0, 50.0, 50.0).drag(Handle::Se, 10.0, 20.0, None, 100.0, 100.0), r(0.0, 0.0, 60.0, 70.0));
    }

    #[test]
    fn drag_west_edge_moves_left_side() {
        assert_eq!(r(0.0, 0.0, 50.0, 50.0).drag(Handle::W, 10.0, 0.0, None, 100.0, 100.0), r(10.0, 0.0, 40.0, 50.0));
    }

    #[test]
    fn drag_north_edge_moves_top_side() {
        assert_eq!(r(0.0, 20.0, 50.0, 50.0).drag(Handle::N, 0.0, -10.0, None, 100.0, 100.0), r(0.0, 10.0, 50.0, 60.0));
    }

    #[test]
    fn drag_collapses_to_minimum() {
        assert_eq!(r(0.0, 0.0, 50.0, 50.0).drag(Handle::Se, -100.0, -100.0, None, 100.0, 100.0), r(0.0, 0.0, MIN, MIN));
    }

    #[test]
    fn drag_east_edge_clamps_to_image() {
        assert_eq!(r(0.0, 0.0, 50.0, 50.0).drag(Handle::E, 100.0, 0.0, None, 100.0, 100.0), r(0.0, 0.0, 100.0, 50.0));
    }

    #[test]
    fn drag_move_translates() {
        assert_eq!(r(10.0, 10.0, 20.0, 20.0).drag(Handle::Move, 5.0, 5.0, None, 100.0, 100.0), r(15.0, 15.0, 20.0, 20.0));
    }

    // --- drag (ratio-locked) -------------------------------------------------

    #[test]
    fn drag_corner_se_locks_ratio_anchored_topleft() {
        assert_eq!(r(0.0, 0.0, 50.0, 50.0).drag(Handle::Se, 20.0, 0.0, Some(1.0), 100.0, 100.0), r(0.0, 0.0, 70.0, 70.0));
    }

    #[test]
    fn drag_corner_nw_locks_ratio_anchored_bottomright() {
        assert_eq!(r(30.0, 30.0, 40.0, 40.0).drag(Handle::Nw, -10.0, 0.0, Some(1.0), 200.0, 200.0), r(20.0, 20.0, 50.0, 50.0));
    }

    #[test]
    fn drag_east_edge_locks_ratio_height_centered() {
        assert_eq!(r(30.0, 30.0, 40.0, 40.0).drag(Handle::E, 10.0, 0.0, Some(1.0), 200.0, 200.0), r(30.0, 25.0, 50.0, 50.0));
    }

    #[test]
    fn drag_north_edge_locks_ratio_width_centered() {
        assert_eq!(r(30.0, 30.0, 40.0, 40.0).drag(Handle::N, 0.0, -10.0, Some(1.0), 200.0, 200.0), r(25.0, 20.0, 50.0, 50.0));
    }
}
