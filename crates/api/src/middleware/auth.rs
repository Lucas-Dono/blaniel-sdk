use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, state::AppState};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub exp: usize,
    pub iat: usize,
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header[7..],
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    let claims = verify_jwt_token(token, &state.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(claims.user_id);

    Ok(next.run(req).await)
}

fn verify_jwt_token(token: &str, secret: &str) -> Result<Claims, ApiError> {
    let validation = Validation::default();
    let key = DecodingKey::from_secret(secret.as_bytes());

    let token_data = decode::<Claims>(token, &key, &validation)
        .map_err(|_| ApiError::Unauthorized)?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    #[test]
    fn test_verify_valid_token() {
        let secret = "test_secret";
        let claims = Claims {
            user_id: "user123".to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iat: chrono::Utc::now().timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let result = verify_jwt_token(&token, secret);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_invalid_token() {
        let result = verify_jwt_token("invalid_token", "secret");
        assert!(result.is_err());
    }
}
