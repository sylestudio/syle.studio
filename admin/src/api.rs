//! Thin fetch client for the syle API. Cookies are sent with every request
//! (`credentials: include`) so the HttpOnly session works behind the Tunnel.

use gloo_net::http::Request;
use serde::de::DeserializeOwned;
use serde::Serialize;
use syle_types::{
    endpoints as ep, BlogPost, Gallery, GalleryDetail, LoginRequest, NewGallery, NewPost,
    Photo, Reorder, UpdateGallery, UpdatePhoto, UpdatePost, User,
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

async fn patch_json<B: Serialize, T: DeserializeOwned>(
    url: &str,
    body: &B,
) -> Result<T, ApiError> {
    let resp = Request::patch(url)
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

async fn send_empty(req: gloo_net::http::RequestBuilder) -> Result<(), ApiError> {
    let resp = req
        .credentials(RequestCredentials::Include)
        .send()
        .await
        .map_err(|_| ApiError::Network)?;
    if resp.status() == 200 {
        Ok(())
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

pub async fn gallery_detail(id: &str) -> Result<GalleryDetail, ApiError> {
    get_json(&ep::admin_gallery(id)).await
}

pub async fn update_gallery(
    id: &str,
    body: &UpdateGallery,
) -> Result<Gallery, ApiError> {
    patch_json(&ep::admin_gallery(id), body).await
}

pub async fn delete_gallery(id: &str) -> Result<(), ApiError> {
    send_empty(Request::delete(&ep::admin_gallery(id))).await
}

pub async fn reorder_photos(
    gallery_id: &str,
    body: &Reorder,
) -> Result<(), ApiError> {
    // 200 with empty body.
    let resp = Request::patch(&ep::admin_gallery_order(gallery_id))
        .credentials(RequestCredentials::Include)
        .json(body)
        .map_err(|_| ApiError::Network)?
        .send()
        .await
        .map_err(|_| ApiError::Network)?;
    if resp.status() == 200 {
        Ok(())
    } else {
        Err(classify(resp.status()))
    }
}

pub async fn update_photo(
    id: &str,
    body: &UpdatePhoto,
) -> Result<Photo, ApiError> {
    patch_json(&ep::admin_photo(id), body).await
}

pub async fn delete_photo(id: &str) -> Result<(), ApiError> {
    send_empty(Request::delete(&ep::admin_photo(id))).await
}

pub async fn get_post(id: &str) -> Result<BlogPost, ApiError> {
    get_json(&ep::admin_post(id)).await
}

pub async fn update_post(
    id: &str,
    body: &UpdatePost,
) -> Result<BlogPost, ApiError> {
    patch_json(&ep::admin_post(id), body).await
}

pub async fn delete_post(id: &str) -> Result<(), ApiError> {
    send_empty(Request::delete(&ep::admin_post(id))).await
}

/// Multipart upload over raw `XMLHttpRequest` so `upload.onprogress` can drive
/// a real per-file progress bar — `gloo_net`/`fetch` exposes no upload
/// progress. `on_progress` receives a 0.0..=1.0 fraction. Handler closures are
/// `forget()`-leaked (3 tiny closures per upload; uploads are rare admin
/// actions) — the idiomatic xhr-in-wasm tradeoff vs. an Rc/RefCell dance.
pub async fn upload_photo(
    form: FormData,
    on_progress: impl Fn(f64) + Clone + 'static,
) -> Result<Photo, ApiError> {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsValue};
    use web_sys::{ProgressEvent, XmlHttpRequest};

    let xhr = XmlHttpRequest::new().map_err(|_| ApiError::Network)?;
    xhr.open_with_async("POST", ep::ADMIN_PHOTOS, true)
        .map_err(|_| ApiError::Network)?;
    xhr.set_with_credentials(true);

    if let Ok(up) = xhr.upload() {
        let prog = on_progress.clone();
        let cb = Closure::<dyn FnMut(ProgressEvent)>::new(move |e: ProgressEvent| {
            if e.length_computable() && e.total() > 0.0 {
                prog((e.loaded() / e.total()).clamp(0.0, 1.0));
            }
        });
        up.set_onprogress(Some(cb.as_ref().unchecked_ref()));
        cb.forget();

        // Bytes fully sent: the server now ingests synchronously (decode +
        // encode derivatives). Emit a definitive 1.0 so the UI can flip to a
        // "processing" state rather than sit at <100% until the response.
        let done = on_progress.clone();
        let on_sent = Closure::<dyn FnMut()>::new(move || done(1.0));
        up.set_onload(Some(on_sent.as_ref().unchecked_ref()));
        on_sent.forget();
    }

    let done = xhr.clone();
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let on_load = Closure::<dyn FnMut()>::new(move || {
            let _ = resolve.call0(&JsValue::NULL);
        });
        let on_err = Closure::<dyn FnMut()>::new(move || {
            let _ = reject.call0(&JsValue::NULL);
        });
        done.set_onload(Some(on_load.as_ref().unchecked_ref()));
        done.set_onerror(Some(on_err.as_ref().unchecked_ref()));
        on_load.forget();
        on_err.forget();
    });

    xhr.send_with_opt_form_data(Some(&form))
        .map_err(|_| ApiError::Network)?;
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|_| ApiError::Network)?;

    let status = xhr.status().map_err(|_| ApiError::Network)?;
    if status != 200 {
        return Err(classify(status));
    }
    let text = xhr
        .response_text()
        .map_err(|_| ApiError::Network)?
        .ok_or(ApiError::Network)?;
    serde_json::from_str(&text).map_err(|_| ApiError::Network)
}
