//! Canonical API paths, shared by the API router, the Leptos CRM and the
//! Astro site so no end can drift from another.

pub const LOGIN: &str = "/api/admin/login";
pub const LOGOUT: &str = "/api/admin/logout";
pub const ME: &str = "/api/admin/me";

pub const ADMIN_GALLERIES: &str = "/api/admin/galleries";
pub const ADMIN_POSTS: &str = "/api/admin/posts";
pub const ADMIN_PHOTOS: &str = "/api/admin/photos";

pub const PUBLIC_GALLERIES: &str = "/api/public/galleries";
pub const PUBLIC_POSTS: &str = "/api/public/posts";

pub fn admin_gallery(id: &str) -> String {
    format!("{ADMIN_GALLERIES}/{id}")
}

pub fn admin_gallery_order(id: &str) -> String {
    format!("{ADMIN_GALLERIES}/{id}/photos/order")
}

pub fn admin_photo(id: &str) -> String {
    format!("{ADMIN_PHOTOS}/{id}")
}

pub fn admin_post(id: &str) -> String {
    format!("{ADMIN_POSTS}/{id}")
}

pub fn public_gallery(slug: &str) -> String {
    format!("{PUBLIC_GALLERIES}/{slug}")
}

pub fn public_post(slug: &str) -> String {
    format!("{PUBLIC_POSTS}/{slug}")
}
