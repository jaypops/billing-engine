use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::extractor::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct Subscription {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub plan_id: Uuid,
    pub status: String,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub canceled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSubscription {
    pub customer_id: Uuid,
    pub plan_id: Uuid,
}

#[utoipa::path(
    post,
    path = "/subscriptions",
    tag = "subscriptions",
    security(("bearer_auth" = [])),
    request_body = CreateSubscription,
    responses(
        (status = 201, description = "Subscription created in pending status", body = Subscription),
        (status = 400, description = "Customer or plan is not active"),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Customer or plan not found, or not yours"),
        (status = 409, description = "Customer already has a live subscription to this plan"),
        (status = 422, description = "Body failed to deserialize (missing or malformed UUID)"),
    ),
)]
pub async fn create_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateSubscription>,
) -> Result<(StatusCode, Json<Subscription>), AppError> {
    let customer = sqlx::query!(
        r#"
        SELECT id, user_id, status
        FROM customers
        WHERE id = $1
        "#,
        body.customer_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("customer not found".into()))?;

    if !auth.user.is_admin && customer.user_id != Some(auth.user.id) {
        return Err(AppError::NotFound("customer not found".into()));
    }

    if customer.status != "active" {
        return Err(AppError::Validation("customer is not active".into()));
    }

    let plan = sqlx::query!(
        r#"
        SELECT id, status
        FROM plans
        WHERE id = $1
        "#,
        body.plan_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("plan not found".into()))?;

    if plan.status != "active" {
        return Err(AppError::Validation("plan is not active".into()));
    }

    let subscription = sqlx::query_as!(
        Subscription,
        r#"
        INSERT INTO subscriptions (id, customer_id, plan_id)
        VALUES ($1, $2, $3)
        RETURNING
            id,
            customer_id,
            plan_id,
            status,
            current_period_start,
            current_period_end,
            cancel_at_period_end,
            trial_ends_at,
            canceled_at,
            created_at
        "#,
        Uuid::now_v7(),
        body.customer_id,
        body.plan_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            AppError::Conflict("customer is already subscribed to this plan".into())
        }
        other => AppError::from(other),
    })?;

    Ok((StatusCode::CREATED, Json(subscription)))
}

#[utoipa::path(
    get,
    path = "/subscriptions/{id}",
    tag = "subscriptions",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Subscription id")),
    responses(
        (status = 200, description = "Subscription", body = Subscription),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Subscription not found, or not yours"),
    ),
)]
pub async fn get_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Subscription>, AppError> {
    let subscription = sqlx::query_as!(
        Subscription,
        r#"
        SELECT
            s.id,
            s.customer_id,
            s.plan_id,
            s.status,
            s.current_period_start,
            s.current_period_end,
            s.cancel_at_period_end,
            s.trial_ends_at,
            s.canceled_at,
            s.created_at
        FROM subscriptions s
        JOIN customers c ON c.id = s.customer_id
        WHERE s.id = $1
          AND ($2 OR c.user_id = $3)
        "#,
        id,
        auth.user.is_admin,
        auth.user.id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("subscription not found".into()))?;

    Ok(Json(subscription))
}

#[utoipa::path(
    get,
    path = "/subscriptions",
    tag = "subscriptions",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Subscriptions visible to the caller", body = [Subscription]),
        (status = 401, description = "Missing or invalid token"),
    ),
)]
pub async fn list_subscriptions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Subscription>>, AppError> {
    let subscriptions = sqlx::query_as!(
        Subscription,
        r#"
        SELECT
            s.id,
            s.customer_id,
            s.plan_id,
            s.status,
            s.current_period_start,
            s.current_period_end,
            s.cancel_at_period_end,
            s.trial_ends_at,
            s.canceled_at,
            s.created_at
        FROM subscriptions s
        JOIN customers c ON c.id = s.customer_id
        WHERE ($1 OR c.user_id = $2)
        ORDER BY s.created_at DESC
        "#,
        auth.user.is_admin,
        auth.user.id,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(subscriptions))
}
