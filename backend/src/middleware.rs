use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use sqlx::SqlitePool;

use crate::auth::{jwt_secret, verify_access_token, CurrentUser};
use crate::errors::AppError;

pub async fn require_auth(
    State(pool): State<SqlitePool>,
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing authorization header".into()))?;

    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        AppError::Unauthorized("authorization header must be a bearer token".into())
    })?;

    let secret = jwt_secret()?;
    let claims = verify_access_token(token, &secret)?;

    let row =
        sqlx::query_as::<_, (String, String)>("SELECT username, role FROM users WHERE id = ?")
            .bind(&claims.sub)
            .fetch_optional(&pool)
            .await?;
    let (username, role) =
        row.ok_or_else(|| AppError::Unauthorized("user no longer exists".into()))?;

    req.extensions_mut().insert(CurrentUser {
        id: claims.sub,
        username,
        role,
    });

    Ok(next.run(req).await)
}
