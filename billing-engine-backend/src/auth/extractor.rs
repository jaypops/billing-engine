use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};

use crate::{
    auth::{CurrentUser, session::find_user_by_session},
    error::AppError,
    state::AppState,
};

pub struct AuthUser {
    pub user: CurrentUser,
    pub token: String,
}

pub struct AdminUser(pub AuthUser);


impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .ok_or_else(|| AppError::Unauthorized("missing token".into()))?;

        let header = header
            .to_str()
            .map_err(|_| AppError::Unauthorized("invalid authorization header".into()))?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("invalid authorization scheme".into()))?;

        let user = find_user_by_session(&state.db, token)
            .await?
            .ok_or_else(|| AppError::Unauthorized("invalid or expired token".into()))?;

        Ok(AuthUser {
            user,
            token: token.to_string(),
        })
    }
}


impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;

        if !auth.user.is_admin {
            return Err(AppError::Forbidden("admin only".into()));
        }

        Ok(AdminUser(auth))
    }
}