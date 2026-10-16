// src/routes/plans.rs

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    auth::extractor::{AdminUser, AuthUser},
    error::AppError,
    money::Money,
    state::AppState,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct Plan {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub amount_kobo: Money,
    pub currency: String,
    pub status: String,
    pub billing_interval: BillingInterval,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, ToSchema, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum BillingInterval {
    Monthly,
    Yearly,
}

impl BillingInterval {
    pub fn as_str(&self) -> &'static str {
        match self {
            BillingInterval::Monthly => "monthly",
            BillingInterval::Yearly => "yearly",
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePlanRequest {
    pub name: String,
    pub description: Option<String>,
    pub amount_kobo: Money,
    pub billing_interval: BillingInterval,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListPlansQuery {
    #[serde(default)]
    pub include_archived: bool,
}

#[utoipa::path(
    tag = "plans",
    post,
    path = "/plans",
    security(("bearer_auth" = [])),
    request_body = CreatePlanRequest,
    responses(
        (status = 201, description = "Plan created", body = Plan),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 422, description = "Unprocessable entity"),
    ),
)]
pub async fn create_plan(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreatePlanRequest>,
) -> Result<(StatusCode, Json<Plan>), AppError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::Validation("plan name must not be empty".into()));
    }

    let description = req
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let plan = sqlx::query_as!(
        Plan,
        r#"
        INSERT INTO plans (id, name, description, amount_kobo, billing_interval)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING
            id,
            name,
            description,
            amount_kobo AS "amount_kobo: Money",
            currency,
            status,
            billing_interval AS "billing_interval: BillingInterval",
            created_at
        "#,
        Uuid::now_v7(),
        name,
        description,
        req.amount_kobo.kobo(),
        req.billing_interval.as_str(),
    )
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(plan)))
}

#[utoipa::path(
    tag = "plans",
    get,
    path = "/plans",
    security(("bearer_auth" = [])),
    params(ListPlansQuery),
    responses(
        (status = 200, description = "List of plans", body = Vec<Plan>),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn list_plans(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListPlansQuery>,
) -> Result<(StatusCode, Json<Vec<Plan>>), AppError> {
    let include_archived = query.include_archived && auth.user.is_admin;

    let plans = sqlx::query_as!(
        Plan,
        r#"
        SELECT
            id,
            name,
            description,
            amount_kobo AS "amount_kobo: Money",
            currency,
            status,
            billing_interval AS "billing_interval: BillingInterval",
            created_at
        FROM plans
        WHERE ($1 OR status = 'active')
        ORDER BY created_at DESC
        "#,
        include_archived,
    )
    .fetch_all(&state.db)
    .await?;

    Ok((StatusCode::OK, Json(plans)))
}

#[utoipa::path(
    tag = "plans",
    get,
    path = "/plans/{id}",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Plan ID")),
    responses(
        (status = 200, description = "Plan", body = Plan),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Plan not found"),
    ),
)]
pub async fn get_plan(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<Plan>), AppError> {
    let plan = sqlx::query_as!(
        Plan,
        r#"
        SELECT
            id,
            name,
            description,
            amount_kobo AS "amount_kobo: Money",
            currency,
            status,
            billing_interval AS "billing_interval: BillingInterval",
            created_at
        FROM plans
        WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("plan not found".into()))?;

    Ok((StatusCode::OK, Json(plan)))
}

#[utoipa::path(
    tag = "plans",
    post,
    path = "/plans/{id}/archive",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Plan ID")),
    responses(
        (status = 200, description = "Plan archived", body = Plan),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Plan not found"),
    ),
)]
pub async fn archive_plan(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<Plan>), AppError> {
    let plan = sqlx::query_as!(
        Plan,
        r#"
        UPDATE plans
        SET status = 'archived'
        WHERE id = $1
        RETURNING
            id,
            name,
            description,
            amount_kobo AS "amount_kobo: Money",
            currency,
            status,
            billing_interval AS "billing_interval: BillingInterval",
            created_at
        "#,
        id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("plan not found".into()))?;

    Ok((StatusCode::OK, Json(plan)))
}