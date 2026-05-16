//! Domain logic + pure-Rust image pipeline
//! (fast_image_resize + ravif + jpeg-encoder + thumbhash).

pub mod db;
pub mod ingest;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
