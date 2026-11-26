use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Serialize, FromRow)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub author_id: i64,
    pub author_username: String,
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

#[derive(Serialize, FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub username: String,
    pub created_at: DateTime<Utc>,
}

// Only used internally for login. Not Serialize, so the hash can never leak in a response.
#[derive(FromRow)]
pub struct UserWithHash {
    pub id: i64,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}
