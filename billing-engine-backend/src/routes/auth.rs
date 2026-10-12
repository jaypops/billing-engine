use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{CurrentUser, password::hash_password, session::create_session},
    error::AppError,
    state::AppState,
};

#[derive(Deserialize, ToSchema)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user: CurrentUser,
}

const MIN_PASSWORD_LEN: usize = 8;
const MAX_PASSWORD_LEN: usize = 128;

#[utoipa::path(
    post,
    path = "/auth/signup",
    tag = "auth",
    request_body = SignupRequest,
    responses(
        (status = 201, description = "Account created", body = AuthResponse),
        (status = 400, description = "Validation error"),
        (status = 409, description = "Email already registered"),
    )
)]
pub async fn signup(
    State(state): State<AppState>,
    Json(body): Json<SignupRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    let email = body.email.trim().to_lowercase();

    if email.is_empty() || !email.contains('@') {
        return Err(AppError::Validation("invalid email".into()));
    }

    let password_len = body.password.chars().count();
    if password_len < MIN_PASSWORD_LEN {
        return Err(AppError::Validation(format!(
            "password must be up to {} characters",
            MAX_PASSWORD_LEN
        )));
    }

    let password = body.password;
    let password_hash = tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;

    let user_id = Uuid::now_v7();

    let row = sqlx::query_as!(
        CurrentUser,
        r#"
        INSERT INTO users (id, email, password_hash, is_admin)
        VALUES ($1, $2, $3, FALSE)
        RETURNING id, email, is_admin
        "#,
        user_id,
        email,
        password_hash,
    )
    .fetch_one(&state.db)
    .await;

    let user = match row {
        Ok(u) => u,
        Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
            return Err(AppError::Conflict("email already registered".into()));
        }
        Err(e) => return Err(e.into()),
    };

    let token = create_session(&state.db, user.id).await?;

    Ok((StatusCode::CREATED, Json(AuthResponse { user, token })))
}
