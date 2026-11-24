use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Serialize, FromRow)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreatePost {
    pub title: String,
    pub body: String,
}

#[derive(Deserialize)]
pub struct UpdatePost {
    pub title: Option<String>,
    pub body: Option<String>,
}
