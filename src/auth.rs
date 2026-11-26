use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{FromRequestParts, State},
    http::{header, request::Parts, StatusCode},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{
    error::AppError,
    models::{User, UserWithHash},
    AppState,
};

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: i64,   // user id
    exp: usize, // expiry (unix seconds)
}

#[derive(Deserialize)]
pub struct RegisterInput {
    pub email: String,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    token: String,
    user: User,
}

// Argon2 is deliberately slow and CPU-heavy, so run it off the async threads.
async fn hash_password(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await
    .map_err(|_| AppError::Internal)?
    .map_err(|_| AppError::Internal)
}

async fn verify_password(password: String, hash: String) -> bool {
    tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash)
            .map(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

fn make_token(user_id: i64, secret: &str) -> Result<String, AppError> {
    let exp = (chrono::Utc::now() + chrono::Duration::days(7)).timestamp() as usize;
    encode(
        &Header::default(),
        &Claims { sub: user_id, exp },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AppError::Internal)
}

pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterInput>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    let email = input.email.trim().to_lowercase();
    let username = input.username.trim().to_string();

    if !email.contains('@') {
        return Err(AppError::BadRequest("invalid email".into()));
    }
    if username.len() < 3 {
        return Err(AppError::BadRequest(
            "username must be at least 3 characters".into(),
        ));
    }
    if input.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }

    let password_hash = hash_password(input.password).await?;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, username, password_hash)
         VALUES ($1, $2, $3)
         RETURNING id, email, username, created_at",
    )
    .bind(&email)
    .bind(&username)
    .bind(&password_hash)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db) = &e {
            if db.is_unique_violation() {
                return AppError::Conflict("email or username already taken".into());
            }
        }
        AppError::Db(e)
    })?;

    let token = make_token(user.id, &state.jwt_secret)?;
    Ok((StatusCode::CREATED, Json(AuthResponse { token, user })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> Result<Json<AuthResponse>, AppError> {
    let email = input.email.trim().to_lowercase();

    let row = sqlx::query_as::<_, UserWithHash>("SELECT * FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::Unauthorized)?; // same error for "no such user" and "wrong password"

    if !verify_password(input.password, row.password_hash).await {
        return Err(AppError::Unauthorized);
    }

    let token = make_token(row.id, &state.jwt_secret)?;
    let user = User {
        id: row.id,
        email: row.email,
        username: row.username,
        created_at: row.created_at,
    };
    Ok(Json(AuthResponse { token, user }))
}

// An "extractor": add `user: AuthUser` to any handler's parameters and that
// route now requires a valid token. Step B uses this on the post routes.
pub struct AuthUser {
    pub id: i64,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let header_value = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = header_value
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized)?;

        Ok(AuthUser {
            id: data.claims.sub,
        })
    }
}

pub async fn me(State(pool): State<PgPool>, user: AuthUser) -> Result<Json<User>, AppError> {
    sqlx::query_as::<_, User>("SELECT id, email, username, created_at FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_optional(&pool)
        .await?
        .map(Json)
        .ok_or(AppError::Unauthorized)
}
