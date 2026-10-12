use utoipa::OpenApi;

use crate::state::AppState;

pub mod auth;
pub mod health;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Billing Engine API",
        version = "0.1.0",
        description = "A billing engine API for handling payments, subscriptions, and invoicing."
    ),
)]
pub struct ApiDoc;

pub fn app() -> axum::Router<AppState> {
    use utoipa_axum::{router::OpenApiRouter, routes};
    use utoipa_scalar::{Scalar, Servable};

    let (router, api) = OpenApiRouter::<AppState>::with_openapi(ApiDoc::openapi())
        .routes(routes!(health::health))
        .routes(routes!(auth::signup))
        .split_for_parts();

    router.merge(Scalar::with_url("/docs", api))
}
