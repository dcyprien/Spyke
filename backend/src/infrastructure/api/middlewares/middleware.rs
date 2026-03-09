use axum::{ extract::{Request, State}, http::{HeaderMap, StatusCode}, middleware::Next, response::{Response}, Json,};
use serde_json::json;
use crate::application::utils::jwt::verify_token;
use crate::AppState; // Import AppState
use crate::domain::models::refresh_token; 
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};

pub async fn auth_middleware(
    State(state): State<AppState>, // 👈 On injecte l'accès à la DB ici
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // 1. Vérifications basiques (Header présent ?)
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Missing Authorization header"}))
        ))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid Authorization header format"}))
        ))?;

    // 2. Crypto : Le token est-il valide mathématiquement ?
    let claims = verify_token(token)
        .map_err(|e| (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": e})) // ou e.to_string() selon ton AppError
        ))?;

    // 3. DB : Le token existe-t-il encore en base ? (Gestion Logout)
    let session_exists = refresh_token::Entity::find()
        .filter(refresh_token::Column::Token.eq(token))
        .one(state.db.as_ref())
        .await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error checking session"}))
        ))?;

    if session_exists.is_none() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Session expired or logged out"}))
        ));
    }

    // 4. Tout est bon, on passe les infos
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}