//! Provision (or reset) a CRM operator. There is no public sign-up by design,
//! so this is how accounts are created — in dev and in prod.
//!
//! Usage: DATABASE_URL=... cargo run -p syle-api --example seed-admin -- <email> <password>

use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL is required"))?;
    let email = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "admin@syle.studio".to_string());
    let password = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "changeme123".to_string());

    let pool = PgPoolOptions::new().connect(&url).await?;
    syle_core::db::migrate(&pool).await?;
    let hash = syle_api::auth::hash_password(&password)?;

    sqlx::query(
        "INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3) \
         ON CONFLICT (email) DO UPDATE SET password_hash = EXCLUDED.password_hash",
    )
    .bind(Uuid::new_v4())
    .bind(&email)
    .bind(&hash)
    .execute(&pool)
    .await?;

    println!("seeded operator: {email}");
    Ok(())
}
