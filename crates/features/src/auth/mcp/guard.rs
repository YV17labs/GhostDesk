//! The per-operation bearer check on `/mcp`.
//!
//! A bare `#[mcp]` endpoint is deny-all, so *something* has to bind here — and
//! an open posture has to say out loud that it is open, which is
//! [`AuthService`](crate::auth::service::AuthService)'s half of the story.

use std::sync::Arc;

use nest_rs::core::injectable;
use nest_rs::http::poem::http::{StatusCode, header};
use nest_rs::http::poem::{Error, Request, Response, Result};
use nest_rs::mcp::{BoxFuture, McpOperationGuard};
use subtle::ConstantTimeEq;

use crate::auth::config::AuthConfig;

#[injectable]
pub struct AuthGuard {
    #[inject]
    config: Arc<AuthConfig>,
}

fn unauthorized() -> Error {
    Error::from_response(
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(header::WWW_AUTHENTICATE, r#"Bearer realm="ghostdesk""#)
            .content_type("text/plain; charset=utf-8")
            .body("unauthorized\n"),
    )
}

/// Constant-time comparison — a plain `==` returns early on the first
/// mismatching byte, which leaks both the token's length and how much of a
/// guess was right.
fn token_matches(provided: &str, expected: &str) -> bool {
    provided.as_bytes().ct_eq(expected.as_bytes()).into()
}

impl McpOperationGuard for AuthGuard {
    fn before<'a>(&'a self, req: &'a mut Request) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let Some(expected) = self.config.token.as_deref() else {
                return Ok(());
            };

            let provided = req
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .map(str::trim);

            match provided {
                Some(token) if token_matches(token, expected) => Ok(()),
                _ => Err(unauthorized()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_exact_token_matches() {
        assert!(token_matches("s3cret", "s3cret"));
        assert!(!token_matches("s3cret", "s3cre"));
        assert!(!token_matches("s3cre", "s3cret"));
        assert!(!token_matches("", "s3cret"));
        assert!(!token_matches("S3CRET", "s3cret"));
    }
}
