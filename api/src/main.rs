//! syle.studio API — public read + CRM write (Cloudflare Tunnel for admin).

use axum::{routing::get, Json, Router};
use syle_types::Health;

async fn health() -> Json<Health> {
    Json(Health { ok: true })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new().route("/health", get(health));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("syle-api listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
