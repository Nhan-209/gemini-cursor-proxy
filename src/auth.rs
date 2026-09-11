use crate::error::GatewayError;

pub fn verify_bearer_token(auth_header: Option<&str>, expected_token: &str) -> Result<(), GatewayError> {
    let header = match auth_header {
        Some(h) if !h.trim().is_empty() => h.trim(),
        _ => return Err(GatewayError::Unauthorized("Missing Authorization header".to_string())),
    };

    let token = if let Some(stripped) = header.strip_prefix("Bearer ") {
        stripped.trim()
    } else if let Some(stripped) = header.strip_prefix("bearer ") {
        stripped.trim()
    } else {
        return Err(GatewayError::Unauthorized("Authorization scheme must be Bearer".to_string()));
    };

    if constant_time_eq(token.as_bytes(), expected_token.as_bytes()) {
        Ok(())
    } else {
        Err(GatewayError::Unauthorized("Invalid proxy token".to_string()))
    }
}

/// Constant-time slice comparison to prevent timing attacks on token verification.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
