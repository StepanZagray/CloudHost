use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub pwd_changed: i64, // Password changed timestamp
}

pub struct AuthState {
    pub secret: String,
    pub password: std::sync::Mutex<Option<String>>,
    pub password_changed_at: std::sync::Mutex<Option<chrono::DateTime<chrono::Utc>>>,
}

impl AuthState {
    pub fn new(
        secret: String,
        password: Option<String>,
        password_changed_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        Self {
            secret,
            password: std::sync::Mutex::new(password),
            password_changed_at: std::sync::Mutex::new(password_changed_at),
        }
    }

    pub fn verify_password(&self, password: &str) -> bool {
        if let Ok(stored_password) = self.password.lock() {
            if let Some(ref stored) = *stored_password {
                password == stored
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn generate_token(&self) -> Result<String, jsonwebtoken::errors::Error> {
        let pwd_changed_timestamp = self
            .password_changed_at
            .lock()
            .ok()
            .and_then(|dt| *dt)
            .map(|dt| dt.timestamp())
            .unwrap_or(0);

        let claims = Claims {
            sub: "admin".to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
            pwd_changed: pwd_changed_timestamp,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let validation = Validation::default();
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &validation,
        )?;

        let claims = token_data.claims;

        // Check if password was changed after token was issued
        if let Ok(current_pwd_changed) = self.password_changed_at.lock() {
            if let Some(current_pwd_changed) = *current_pwd_changed {
                let current_pwd_timestamp = current_pwd_changed.timestamp();
                if current_pwd_timestamp > claims.pwd_changed {
                    return Err(jsonwebtoken::errors::Error::from(
                        jsonwebtoken::errors::ErrorKind::InvalidToken,
                    ));
                }
            }
        }

        Ok(claims)
    }
}

pub async fn auth_middleware(
    State(auth_state): State<Arc<AuthState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check for Authorization header
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if auth_state.verify_token(token).is_ok() {
                    return Ok(next.run(request).await);
                }
            }
        }
    }

    // Check for password in query parameters (for simple auth)
    if let Some(password) = request.uri().query().and_then(|q| {
        q.split('&')
            .find(|param| param.starts_with("password="))
            .map(|param| &param[9..])
    }) {
        if auth_state.verify_password(password) {
            return Ok(next.run(request).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

pub async fn login(
    State(auth_state): State<Arc<AuthState>>,
    axum::Json(payload): axum::Json<LoginRequest>,
) -> Result<axum::Json<LoginResponse>, StatusCode> {
    if auth_state.verify_password(&payload.password) {
        if let Ok(token) = auth_state.generate_token() {
            return Ok(axum::Json(LoginResponse { token }));
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}
