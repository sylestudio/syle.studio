//! The CSS / `ctx.filter` string for the editor. The SAME string drives the live
//! preview element's `style.filter` and the bake canvas's `ctx.filter`, so what
//! the user sees is what gets baked — for everything CSS filters can express.

/// Per-channel adjustments as percentages where 100 = neutral, matching the
/// `brightness()` / `contrast()` / `saturate()` CSS functions 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adjust {
    pub brightness: u16,
    pub contrast: u16,
    pub saturation: u16,
}

impl Default for Adjust {
    fn default() -> Self {
        Self { brightness: 100, contrast: 100, saturation: 100 }
    }
}

impl Adjust {
    /// True when nothing would change (so callers can emit `filter: none`).
    pub fn is_neutral(&self) -> bool {
        *self == Self::default()
    }
}

/// Brand-aligned presets (gothic-punk DNA). `Bw`/`Fade` are pure CSS-filter
/// expressions; `Grain`/`Vignette` need a compositing pass at bake time (done in
/// the admin canvas glue) and contribute no CSS fragment here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Bw,
    Fade,
    Grain,
    Vignette,
}

impl Preset {
    /// CSS-filter fragment for presets expressible as filters; empty string for
    /// the compositing-only presets.
    pub fn css_fragment(self) -> &'static str {
        match self {
            Preset::Bw => "grayscale(1) contrast(140%)",
            Preset::Fade => "saturate(70%) brightness(105%) contrast(90%)",
            Preset::Grain | Preset::Vignette => "",
        }
    }

    /// Whether the preset requires a pixel/compositing pass beyond CSS filters.
    pub fn needs_compositing(self) -> bool {
        matches!(self, Preset::Grain | Preset::Vignette)
    }
}

/// Build the combined filter string: adjustment functions first, then the preset
/// fragment. Returns `""` when there is nothing to apply.
pub fn css_filter(adjust: &Adjust, preset: Option<Preset>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if adjust.brightness != 100 {
        parts.push(format!("brightness({}%)", adjust.brightness));
    }
    if adjust.contrast != 100 {
        parts.push(format!("contrast({}%)", adjust.contrast));
    }
    if adjust.saturation != 100 {
        parts.push(format!("saturate({}%)", adjust.saturation));
    }
    if let Some(p) = preset {
        let frag = p.css_fragment();
        if !frag.is_empty() {
            parts.push(frag.to_string());
        }
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_adjust_no_preset_is_empty() {
        assert_eq!(css_filter(&Adjust::default(), None), "");
        assert!(Adjust::default().is_neutral());
    }

    #[test]
    fn single_adjustment_renders_one_function() {
        let a = Adjust { brightness: 110, ..Default::default() };
        assert_eq!(css_filter(&a, None), "brightness(110%)");
        assert!(!a.is_neutral());
    }

    #[test]
    fn all_adjustments_render_in_stable_order() {
        let a = Adjust { brightness: 90, contrast: 120, saturation: 80 };
        assert_eq!(css_filter(&a, None), "brightness(90%) contrast(120%) saturate(80%)");
    }

    #[test]
    fn bw_preset_appends_its_fragment() {
        assert_eq!(css_filter(&Adjust::default(), Some(Preset::Bw)), "grayscale(1) contrast(140%)");
        let a = Adjust { brightness: 105, ..Default::default() };
        assert_eq!(
            css_filter(&a, Some(Preset::Bw)),
            "brightness(105%) grayscale(1) contrast(140%)"
        );
    }

    #[test]
    fn fade_preset_fragment() {
        assert_eq!(
            css_filter(&Adjust::default(), Some(Preset::Fade)),
            "saturate(70%) brightness(105%) contrast(90%)"
        );
    }

    #[test]
    fn compositing_presets_contribute_no_css() {
        assert_eq!(css_filter(&Adjust::default(), Some(Preset::Grain)), "");
        assert_eq!(css_filter(&Adjust::default(), Some(Preset::Vignette)), "");
        assert!(Preset::Grain.needs_compositing());
        assert!(Preset::Vignette.needs_compositing());
        assert!(!Preset::Bw.needs_compositing());
        assert!(!Preset::Fade.needs_compositing());
    }

    #[test]
    fn adjustments_kept_even_with_compositing_preset() {
        let a = Adjust { contrast: 130, ..Default::default() };
        assert_eq!(css_filter(&a, Some(Preset::Grain)), "contrast(130%)");
    }
}
