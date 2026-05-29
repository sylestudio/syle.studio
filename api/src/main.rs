//! syle.studio API binary. `/api/admin/*` is exposed only via Cloudflare
//! Tunnel; `/api/public/*` is fronted by the CDN.

use std::path::PathBuf;
use syle_api::auth::build_webauthn;
use syle_api::{app, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL is required"))?;
    let media_dir = PathBuf::from(
        std::env::var("MEDIA_DIR").unwrap_or_else(|_| "./media".to_string()),
    );
    // Relying-party identity. RP_ID is the apex domain (e.g. `syle.studio`);
    // RP_ORIGIN is the admin origin (e.g. `https://admin.syle.studio`). Both are
    // required — a misconfigured RP silently breaks every passkey ceremony.
    let rp_id =
        std::env::var("RP_ID").map_err(|_| anyhow::anyhow!("RP_ID is required"))?;
    let rp_origin = std::env::var("RP_ORIGIN")
        .map_err(|_| anyhow::anyhow!("RP_ORIGIN is required"))?;
    let webauthn = build_webauthn(&rp_id, &rp_origin)?;

    let pool = syle_core::db::connect(&database_url).await?;
    syle_core::db::migrate(&pool).await?;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("syle-api listening on {}", listener.local_addr()?);
    axum::serve(listener, app(AppState::new(pool, media_dir, webauthn))).await?;
    Ok(())
}
