use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

use crate::state::AppState;

pub mod auth;
pub mod customers;
pub mod health;
pub mod plans;

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_default();
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("opaque")
                    .build(),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(info(
    title = "Billing Engine API",
    version = "0.1.0",
    description = "A billing engine API for handling payments, subscriptions, and invoicing."
))]
pub struct ApiDoc;

pub fn app() -> axum::Router<AppState> {
    use utoipa_axum::{router::OpenApiRouter, routes};
    use utoipa_scalar::{Scalar, Servable};

    let (router, api) = OpenApiRouter::<AppState>::with_openapi(ApiDoc::openapi())
        .routes(routes!(health::health))
        .routes(routes!(auth::signup))
        .routes(routes!(auth::login))
        .routes(routes!(auth::me))
        .routes(routes!(auth::logout))
        .routes(routes!(
            customers::create_customer,
            customers::list_customers
        ))
        .routes(routes!(customers::get_customer))
        .routes(routes!(plans::create_plan, plans::list_plans))
        .routes(routes!(plans::get_plan))
        .routes(routes!(plans::archive_plan))
        .split_for_parts();

    router.merge(Scalar::with_url("/docs", api))
}
