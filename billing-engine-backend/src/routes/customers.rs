use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;


use crate::{auth::extractor::AuthUser, error::AppError, state::AppState};

#[derive(Serialize, ToSchema)]
pub struct Customer {
    id: Uuid,
    user_id: Option<Uuid>,
    name: String,
    email: String,
    phone: Option<String>,
    paystack_customer_code: Option<String>,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateCustomerRequest {
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListCustomersQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}



#[utoipa::path(
    post,
    path = "/customers",
    tag = "customers",
    request_body = CreateCustomerRequest,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 201, description = "Customer created", body = Customer),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Conflict"),
    )
)]
pub async fn create_customer(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateCustomerRequest>,
) -> Result<(StatusCode, Json<Customer>), AppError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::Validation("name must not be empty".into()));
    }

    let email = body.email.trim();
    if !email.contains('@') {
        return Err(AppError::Validation("invalid email".into()));
    }
    let email = email.to_lowercase();

    let phone = body
        .phone
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty());

    let user_id = if auth.user.is_admin {
        None
    } else {
        Some(auth.user.id)
    };

    let customer = sqlx::query_as!(
        Customer,
        r#"
          INSERT INTO customers (id, user_id, name, email, phone)
          VALUES ($1, $2, $3, $4, $5)
          RETURNING id, user_id, name, email, phone, paystack_customer_code, status, created_at
        "#,
        Uuid::now_v7(),
        user_id,
        name,
        email,
        phone,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            AppError::Conflict("email or customer already exists".into())
        }
        _ => AppError::from(e),
    })?;

    Ok((StatusCode::CREATED, Json(customer)))
}

#[utoipa::path(
    get,
    path = "/customers/{id}",
    tag = "customers",
    params(
        ("id" = Uuid, Path, description = "Customer ID"),
    ),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Customer found", body = Customer),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Customer not found"),
    )
)]


pub async fn get_customer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Customer>, AppError> {
    let customer = sqlx::query_as!(
        Customer,
        r#"
          SELECT id, user_id, name, email, phone, paystack_customer_code, status, created_at
          FROM customers
          WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("customer not found".into()))?;

    if !auth.user.is_admin && customer.user_id != Some(auth.user.id) {
        return Err(AppError::NotFound("customer not found".into()));
    }

    Ok(Json(customer))
}

#[utoipa::path(
    get,
    path = "/customers",
    tag = "customers",
    params(ListCustomersQuery),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "List of customers", body = [Customer]),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn list_customers(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListCustomersQuery>,
) -> Result<Json<Vec<Customer>>, AppError> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let customers = if auth.user.is_admin {
        sqlx::query_as!(
            Customer,
            r#"
              SELECT id, user_id, name, email, phone, paystack_customer_code, status, created_at
              FROM customers
              ORDER BY created_at DESC, id DESC
              LIMIT $1 OFFSET $2
            "#,
            limit,
            offset,
        )
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as!(
            Customer,
            r#"
              SELECT id, user_id, name, email, phone, paystack_customer_code, status, created_at
              FROM customers
              WHERE user_id = $1
              ORDER BY created_at DESC, id DESC
              LIMIT $2 OFFSET $3
            "#,
            auth.user.id,
            limit,
            offset,
        )
        .fetch_all(&state.db)
        .await?
    };

    Ok(Json(customers))
}
