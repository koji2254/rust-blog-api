use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;

use crate::{
    auth::AuthUser,
    error::AppError,
    models::{CreatePost, Post, UpdatePost},
};

const POST_COLUMNS: &str =
    "p.id, p.title, p.body, p.author_id, u.username AS author_username, p.created_at, p.updated_at";

// Returns Ok if the logged-in user owns the post, 404 if it doesn't exist, 403 if it's someone else's.
async fn ensure_owner(pool: &PgPool, post_id: i64, user_id: i64) -> Result<(), AppError> {
    let author = sqlx::query_scalar::<_, i64>("SELECT author_id FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(pool)
        .await?;

    match author {
        None => Err(AppError::NotFound),
        Some(a) if a == user_id => Ok(()),
        Some(_) => Err(AppError::Forbidden),
    }
}

// ---------- public ----------

pub async fn list(State(pool): State<PgPool>) -> Result<Json<Vec<Post>>, AppError> {
    let sql = format!(
        "SELECT {POST_COLUMNS} FROM posts p JOIN users u ON u.id = p.author_id ORDER BY p.id DESC"
    );
    let posts = sqlx::query_as::<_, Post>(&sql).fetch_all(&pool).await?;
    Ok(Json(posts))
}

pub async fn get_one(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<Post>, AppError> {
    let sql = format!(
        "SELECT {POST_COLUMNS} FROM posts p JOIN users u ON u.id = p.author_id WHERE p.id = $1"
    );
    sqlx::query_as::<_, Post>(&sql)
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

// ---------- login required ----------

pub async fn create(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(input): Json<CreatePost>,
) -> Result<(StatusCode, Json<Post>), AppError> {
    let title = input.title.trim();
    let body = input.body.trim();
    if title.is_empty() || body.is_empty() {
        return Err(AppError::BadRequest("title and body are required".into()));
    }

    let sql = format!(
        "WITH p AS (
             INSERT INTO posts (title, body, author_id) VALUES ($1, $2, $3) RETURNING *
         )
         SELECT {POST_COLUMNS} FROM p JOIN users u ON u.id = p.author_id"
    );
    let post = sqlx::query_as::<_, Post>(&sql)
        .bind(title)
        .bind(body)
        .bind(user.id)
        .fetch_one(&pool)
        .await?;
    Ok((StatusCode::CREATED, Json(post)))
}

pub async fn update(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    user: AuthUser,
    Json(input): Json<UpdatePost>,
) -> Result<Json<Post>, AppError> {
    if input.title.as_deref().is_some_and(|t| t.trim().is_empty())
        || input.body.as_deref().is_some_and(|b| b.trim().is_empty())
    {
        return Err(AppError::BadRequest(
            "title and body cannot be empty".into(),
        ));
    }

    ensure_owner(&pool, id, user.id).await?;

    let sql = format!(
        "WITH p AS (
             UPDATE posts
             SET title = COALESCE($1, title),
                 body = COALESCE($2, body),
                 updated_at = now()
             WHERE id = $3 AND author_id = $4
             RETURNING *
         )
         SELECT {POST_COLUMNS} FROM p JOIN users u ON u.id = p.author_id"
    );
    let post = sqlx::query_as::<_, Post>(&sql)
        .bind(input.title.map(|t| t.trim().to_string()))
        .bind(input.body.map(|b| b.trim().to_string()))
        .bind(id)
        .bind(user.id)
        .fetch_one(&pool)
        .await?;
    Ok(Json(post))
}

pub async fn delete(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    user: AuthUser,
) -> Result<StatusCode, AppError> {
    ensure_owner(&pool, id, user.id).await?;

    sqlx::query("DELETE FROM posts WHERE id = $1 AND author_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
