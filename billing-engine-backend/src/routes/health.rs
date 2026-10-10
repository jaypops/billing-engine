use crate::{error::AppError, state::AppState};
use axum::{Json, extract::State};

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    status: String,
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses(
        (status = 200, description = "Service and database are up", body = HealthResponse),
        (status = 500, description = "Database unreachable"),
    )
)]
#[axum::debug_handler]
pub async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, AppError> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    Ok(Json(HealthResponse {
        status: "ok".into(),
    }))
}
