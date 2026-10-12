use serde::Serialize;
use uuid::Uuid;
use utoipa::ToSchema;

pub mod extractor;
pub mod password;
pub mod session;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CurrentUser {
    pub id: Uuid,
    pub email: String,
    pub is_admin: bool,
}
