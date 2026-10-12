use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{auth::CurrentUser, error::AppError};

pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    hex::encode(bytes)
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn create_session(pool: &PgPool, user_id: Uuid) -> Result<String, AppError> {
    let token = generate_token();
    let token_hash = sha256_hex(&token);

    let expires_at = Utc::now() + Duration::days(30);

    sqlx::query(
        r#"
        INSERT INTO sessions (id, user_id, token_hash, expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(token)
}

pub async fn find_user_by_session(
    pool: &PgPool,
    token: &str,
) -> Result<Option<CurrentUser>, AppError> {
    let token_hash = sha256_hex(token);

    let user = sqlx::query_as!(
        CurrentUser,
        r#"
        SELECT u.id, u.email, u.is_admin
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.token_hash = $1 AND s.expires_at > NOW()
        "#,
        token_hash
    )
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn delete_session(pool: &PgPool, token: &str) -> Result<(), AppError> {
    let token_hash = sha256_hex(token);

    sqlx::query!(
        r#"
        DELETE FROM sessions
        WHERE token_hash = $1
        "#,
        token_hash
    )
    .execute(pool)
    .await?;

    Ok(())
}
