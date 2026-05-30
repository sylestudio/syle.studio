//! Pure edit math for the CRM image editor (Fase A).
//!
//! Three concerns, each a small pure module with no platform dependencies so the
//! `cargo test` host loop covers them (the admin canvas glue is the only part
//! that touches `web-sys`):
//!
//! - [`filter`] — the CSS / `ctx.filter` string applied to both the live preview
//!   element and the bake canvas, so they match.
//! - [`crop`] — crop-rectangle geometry in source-pixel space (clamp, translate,
//!   aspect-ratio fit, grip dragging).
//! - [`geom`] — output dimensions after rotation and the bake-size cap.

pub mod crop;
pub mod filter;
pub mod geom;

pub use crop::{CropRect, Handle};
pub use filter::{css_filter, Adjust, Preset};
pub use geom::{bake_dims, rotated_dims, BAKE_MAX_LONG};
