use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{
        CurrentUser,
        extractor::AuthUser,
        password::{hash_password, verify_password},
        session::{create_session, delete_session},
    },
    error::AppError,
    state::AppState,
};

#[derive(Deserialize, ToSchema)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
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

#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, body = AuthResponse),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let email = req.email.trim().to_lowercase();

    if req.password.chars().count() > MAX_PASSWORD_LEN {
        return Err(AppError::Unauthorized("invalid credentials".into()));
    }

    let row = sqlx::query!(
        r#"
        SELECT id, email, is_admin, password_hash
        FROM users
        WHERE email = $1
        "#,
        email
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(user) = row else {
        return Err(AppError::Unauthorized("invalid credentials".into()));
    };

    let password = req.password;
    let hash = user.password_hash;

    let valid = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;

    if !valid {
        return Err(AppError::Unauthorized("invalid credentials".into()));
    }

    let token = create_session(&state.db, user.id).await?;

    let user = CurrentUser {
        id: user.id,
        email: user.email,
        is_admin: user.is_admin,
    };

    Ok(Json(AuthResponse { token, user }))
}

#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    responses(
        (status = 204, description = "Logged out"),
        (status = 401, description = "Missing, invalid or expired token"),
    )
)]
pub async fn logout(State(state): State<AppState>, auth: AuthUser) -> Result<StatusCode, AppError> {
    delete_session(&state.db, &auth.token).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "Current user data", body = CurrentUser),
        (status = 401, description = "Missing, invalid or expired token"),
    )
)]
pub async fn me(auth: AuthUser) -> Json<CurrentUser> {
    Json(auth.user)
}
