mod error;
mod handlers;
mod models;

use axum::{routing::get, Router};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("failed to connect to database");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("migrations failed");

    let app = Router::new()
        .route("/posts", get(handlers::list).post(handlers::create))
        .route(
            "/posts/{id}",
            get(handlers::get_one)
                .patch(handlers::update)
                .delete(handlers::delete),
        )
        .with_state(pool)
        .layer(CorsLayer::permissive());

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    println!("listening on {port}");
    axum::serve(listener, app).await.unwrap();
}
