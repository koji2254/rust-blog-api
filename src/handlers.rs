use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;

use crate::{
    error::AppError,
    models::{CreatePost, Post, UpdatePost},
};

pub async fn list(State(pool): State<PgPool>) -> Result<Json<Vec<Post>>, AppError> {
    let posts = sqlx::query_as::<_, Post>("SELECT * FROM posts ORDER BY id DESC")
        .fetch_all(&pool)
        .await?;
    Ok(Json(posts))
}

pub async fn get_one(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<Post>, AppError> {
    sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

pub async fn create(
    State(pool): State<PgPool>,
    Json(input): Json<CreatePost>,
) -> Result<(StatusCode, Json<Post>), AppError> {
    let post =
        sqlx::query_as::<_, Post>("INSERT INTO posts (title, body) VALUES ($1, $2) RETURNING *")
            .bind(input.title)
            .bind(input.body)
            .fetch_one(&pool)
            .await?;
    Ok((StatusCode::CREATED, Json(post)))
}

pub async fn update(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(input): Json<UpdatePost>,
) -> Result<Json<Post>, AppError> {
    sqlx::query_as::<_, Post>(
        "UPDATE posts
         SET title = COALESCE($1, title),
             body = COALESCE($2, body),
             updated_at = now()
         WHERE id = $3 RETURNING *",
    )
    .bind(input.title)
    .bind(input.body)
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .map(Json)
    .ok_or(AppError::NotFound)
}

pub async fn delete(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM posts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
