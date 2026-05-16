//! Thin fetch client for the syle API. Cookies are sent with every request
//! (`credentials: include`) so the HttpOnly session works behind the Tunnel.

use gloo_net::http::Request;
use serde::de::DeserializeOwned;
use serde::Serialize;
use syle_types::{
    endpoints as ep, BlogPost, Gallery, LoginRequest, NewGallery, NewPost, Photo, User,
};
use web_sys::{FormData, RequestCredentials};

#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    Unauthorized,
    Status(u16),
    Network,
}

fn classify(status: u16) -> ApiError {
    if status == 401 {
        ApiError::Unauthorized
    } else {
        ApiError::Status(status)
    }
}

async fn get_json<T: DeserializeOwned>(url: &str) -> Result<T, ApiError> {
    let resp = Request::get(url)
        .credentials(RequestCredentials::Include)
        .send()
        .await
        .map_err(|_| ApiError::Network)?;
    if resp.status() == 200 {
        resp.json().await.map_err(|_| ApiError::Network)
    } else {
        Err(classify(resp.status()))
    }
}

async fn post_json<B: Serialize, T: DeserializeOwned>(
    url: &str,
    body: &B,
) -> Result<T, ApiError> {
    let resp = Request::post(url)
        .credentials(RequestCredentials::Include)
        .json(body)
        .map_err(|_| ApiError::Network)?
        .send()
        .await
        .map_err(|_| ApiError::Network)?;
    if resp.status() == 200 {
        resp.json().await.map_err(|_| ApiError::Network)
    } else {
        Err(classify(resp.status()))
    }
}

pub async fn me() -> Result<User, ApiError> {
    get_json(ep::ME).await
}

pub async fn login(req: &LoginRequest) -> Result<User, ApiError> {
    post_json(ep::LOGIN, req).await
}

pub async fn logout() {
    let _ = Request::post(ep::LOGOUT)
        .credentials(RequestCredentials::Include)
        .send()
        .await;
}

pub async fn list_galleries() -> Result<Vec<Gallery>, ApiError> {
    get_json(ep::ADMIN_GALLERIES).await
}

pub async fn create_gallery(n: &NewGallery) -> Result<Gallery, ApiError> {
    post_json(ep::ADMIN_GALLERIES, n).await
}

pub async fn list_posts() -> Result<Vec<BlogPost>, ApiError> {
    get_json(ep::ADMIN_POSTS).await
}

pub async fn create_post(n: &NewPost) -> Result<BlogPost, ApiError> {
    post_json(ep::ADMIN_POSTS, n).await
}

pub async fn upload_photo(form: FormData) -> Result<Photo, ApiError> {
    let resp = Request::post(ep::ADMIN_PHOTOS)
        .credentials(RequestCredentials::Include)
        .body(form)
        .map_err(|_| ApiError::Network)?
        .send()
        .await
        .map_err(|_| ApiError::Network)?;
    if resp.status() == 200 {
        resp.json().await.map_err(|_| ApiError::Network)
    } else {
        Err(classify(resp.status()))
    }
}
