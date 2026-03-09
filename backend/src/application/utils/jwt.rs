use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use chrono::{Utc, Duration};
use uuid::Uuid;
use std::env;
use crate::application::dto::{apperror::AppError, token_dto::Claims};

pub fn generate_token(user_id: Uuid, username: String) -> Result<String, AppError> {
    let jwt_secret = env::var("JWT_SECRET")
        .map_err(|_| AppError::InternalServerError("JWT_SECRET not found in environment".to_string()))?;


    let now = Utc::now();
    let expiration = now + Duration::hours(48);

    let claims = Claims {
        sub: user_id,
        username,
        exp: expiration.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref())
    )
    .map_err(|e| AppError::InternalServerError(e.to_string()))
}

pub fn verify_token(token: &str) -> Result<Claims, AppError> {
    let jwt_secret = env::var("JWT_SECRET")
        .map_err(|_| AppError::InternalServerError("JWT_SECRET not found in environment".to_string()))?;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default()
    )
    .map(|data| data.claims)
    .map_err(|e| AppError::InternalServerError(e.to_string()))
}