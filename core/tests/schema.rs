//! Schema test: applies migrations against the dev DB and round-trips a row.
//! Requires DATABASE_URL (workspace `.env`); skips cleanly if unset.

use syle_core::db;
use uuid::Uuid;

#[tokio::test]
async fn migrations_apply_and_gallery_roundtrips() {
    dotenvy::from_path("../.env").ok();
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL unset — skipping DB test");
        return;
    };

    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO galleries (id, slug, title, position, published) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(format!("g-{id}"))
    .bind("Test")
    .bind(0_i32)
    .bind(true)
    .execute(&pool)
    .await
    .expect("insert");

    let (slug, published): (String, bool) =
        sqlx::query_as("SELECT slug, published FROM galleries WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("select");
    assert_eq!(slug, format!("g-{id}"));
    assert!(published);

    sqlx::query("DELETE FROM galleries WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("cleanup");
}
