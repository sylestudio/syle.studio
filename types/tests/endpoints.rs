//! The CRM and the public site must hit the exact paths the API serves.
//! Centralizing them here keeps both ends from drifting.

use syle_types::endpoints as ep;

#[test]
fn auth_paths() {
    assert_eq!(ep::LOGIN, "/api/admin/login");
    assert_eq!(ep::LOGOUT, "/api/admin/logout");
    assert_eq!(ep::ME, "/api/admin/me");
}

#[test]
fn write_paths() {
    assert_eq!(ep::ADMIN_GALLERIES, "/api/admin/galleries");
    assert_eq!(ep::ADMIN_POSTS, "/api/admin/posts");
    assert_eq!(ep::ADMIN_PHOTOS, "/api/admin/photos");
}

#[test]
fn public_read_paths() {
    assert_eq!(ep::PUBLIC_GALLERIES, "/api/public/galleries");
    assert_eq!(ep::public_gallery("a-slug"), "/api/public/galleries/a-slug");
    assert_eq!(ep::PUBLIC_POSTS, "/api/public/posts");
    assert_eq!(ep::public_post("hello"), "/api/public/posts/hello");
}
