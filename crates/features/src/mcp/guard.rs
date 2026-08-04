//! The operation guard for `/mcp`.
//!
//! Auth ≡ TLS, exactly as in the Python original:
//!
//! * **Token configured** — every operation must carry
//!   `Authorization: Bearer <token>`, compared in constant time.
//! * **No token** — the endpoint is open. Shipping a static bearer token over
//!   cleartext would be security theatre (no rotation, no per-user identity),
//!   so the surface is deliberately left open and the operator is expected to
//!   either mount a cert or keep the port on a trusted loopback.
//!
//! The container entrypoint is what ties the two together: it refuses to boot
//! with TLS and no token, and drops a token supplied without TLS. This guard
//! enforces the same invariant a second time, at the one place it cannot be
//! bypassed — a bare `#[mcp]` endpoint is deny-all, so *something* has to
//! bind, and an open posture has to say out loud that it is open.

use std::sync::Arc;

use nest_rs::core::{hooks, injectable};
use nest_rs::http::HttpConfig;
use nest_rs::http::poem::http::{StatusCode, header};
use nest_rs::http::poem::{Error, Request, Response, Result};
use nest_rs::mcp::{BoxFuture, McpOperationGuard};
use subtle::ConstantTimeEq;

use crate::config::GhostdeskConfig;

#[injectable]
pub struct McpAuthGuard {
    #[inject]
    config: Arc<GhostdeskConfig>,
}

/// The boot-time half of the posture, split into its own provider on purpose.
///
/// `McpAuthGuard` is bound as `dyn McpOperationGuard`, and a provider reached
/// only through a trait object is not reachable *as itself* — the framework
/// skips its lifecycle hooks and says so at boot. Putting the check on a
/// plainly-registered provider is what makes it actually run.
#[injectable]
pub struct McpSecurityPosture {
    #[inject]
    config: Arc<GhostdeskConfig>,
    #[inject]
    http: Arc<HttpConfig>,
}

#[hooks]
impl McpSecurityPosture {
    /// Refuse to serve a half-configured security posture.
    ///
    /// Init hooks are strict, so this aborts the boot rather than leaving a
    /// TLS endpoint unauthenticated. The entrypoint already checks it for
    /// containerised runs; this covers every other way the binary starts.
    #[on_module_init]
    async fn check_posture(&self) -> anyhow::Result<()> {
        match (self.http.tls.is_some(), self.config.auth_token.is_some()) {
            (true, false) => anyhow::bail!(
                "NESTRS_GHOSTDESK__AUTH_TOKEN is required when TLS is enabled — \
                 an HTTPS endpoint with no bearer token is an open desktop",
            ),
            (true, true) => tracing::info!(
                target: "ghostdesk::mcp",
                "TLS enabled, bearer-token auth required",
            ),
            (false, true) => tracing::warn!(
                target: "ghostdesk::mcp",
                "bearer token configured without TLS — the token crosses the \
                 wire in cleartext; mount a cert or drop the token",
            ),
            (false, false) => tracing::warn!(
                target: "ghostdesk::mcp",
                "no TLS cert — serving plain HTTP with NO authentication. This \
                 is the intended dev posture; deploy behind a cert plus \
                 NESTRS_GHOSTDESK__AUTH_TOKEN for any non-loopback exposure.",
            ),
        }
        Ok(())
    }
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

impl McpOperationGuard for McpAuthGuard {
    fn before<'a>(&'a self, req: &'a mut Request) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let Some(expected) = self.config.auth_token.as_deref() else {
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
