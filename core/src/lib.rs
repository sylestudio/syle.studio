//! Domain logic + pure-Rust image pipeline
//! (fast_image_resize + ravif + webp + thumbhash).

pub mod db;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
