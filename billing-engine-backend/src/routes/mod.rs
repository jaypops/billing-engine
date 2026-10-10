use utoipa::OpenApi;

use crate::state::AppState;

pub mod health;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Billing Engine API",
        version = "0.1.0",
        description = "A billing engine API for handling payments, subscriptions, and invoicing."
    ),
    paths(health::health,),
    components(schemas())
)]
pub struct ApiDoc;

pub fn app() -> axum::Router<AppState> {
    use utoipa_axum::{router::OpenApiRouter, routes};
    use utoipa_scalar::{Scalar, Servable};

    let (router, api) = OpenApiRouter::<AppState>::with_openapi(ApiDoc::openapi())
        .routes(routes!(health::health))
        .split_for_parts();

    router.merge(Scalar::with_url("/docs", api))
}
