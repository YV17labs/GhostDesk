//! What the endpoint demands of a caller, decided once at boot.
//!
//! Auth ≡ TLS, exactly as in the Python original:
//!
//! * **Token configured** — every operation must carry
//!   `Authorization: Bearer <token>`, which is the MCP adapter's guard's job.
//! * **No token** — the endpoint is open. Shipping a static bearer token over
//!   cleartext would be security theatre (no rotation, no per-user identity),
//!   so the surface is deliberately left open and the operator is expected to
//!   either mount a cert or keep the port on a trusted loopback.
//!
//! The container entrypoint is what ties the two together: it refuses to boot
//! with TLS and no token, and drops a token supplied without TLS. This service
//! enforces the same invariant a second time, on every other way the binary
//! starts.
//!
//! Not folded into [`AuthGuard`]: the guard is bound as `dyn
//! McpOperationGuard`, and a provider reached only through a trait object is
//! not reachable *as itself* — the framework skips its lifecycle hooks and
//! says so at boot. The rule has to hang off a plainly-registered provider to
//! run at all.
//!
//! [`AuthGuard`]: super::mcp::guard::AuthGuard

use std::sync::Arc;

use nest_rs::config::var_name;
use nest_rs::core::{hooks, injectable};
use nest_rs::http::HttpConfig;

use super::config::AuthConfig;
use super::error::AuthError;

/// The security posture the endpoint is about to serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Posture {
    /// TLS and a bearer token. The only posture fit for a non-loopback port.
    Secured,
    /// TLS with no token — an open desktop over HTTPS. Refused.
    UnauthenticatedTls,
    /// A token that would cross the wire in cleartext. Served, loudly.
    CleartextToken,
    /// Plain HTTP, no auth. The intended dev posture.
    Open,
}

#[injectable]
pub struct AuthService {
    #[inject]
    config: Arc<AuthConfig>,
    #[inject]
    http: Arc<HttpConfig>,
}

impl AuthService {
    /// Read the posture from the two settings that decide it.
    ///
    /// A plain function over `(tls, token)` so the four cases are reachable
    /// from a test. They used to exist only inside a boot hook, which made the
    /// one security rule the server enforces about *itself* the one thing
    /// nothing could exercise.
    pub fn posture(&self) -> Posture {
        match (self.http.tls.is_some(), self.config.token.is_some()) {
            (true, true) => Posture::Secured,
            (true, false) => Posture::UnauthenticatedTls,
            (false, true) => Posture::CleartextToken,
            (false, false) => Posture::Open,
        }
    }
}

#[hooks]
impl AuthService {
    /// Refuse to serve a half-configured security posture.
    ///
    /// Init hooks are strict, so this aborts the boot rather than leaving a
    /// TLS endpoint unauthenticated.
    #[on_module_init]
    async fn announce_posture(&self) -> anyhow::Result<()> {
        match self.posture() {
            // The one place `anyhow` is right in this crate: a boot hook is
            // the binary's entry point by another name, and nothing above it
            // can branch on the reason.
            Posture::UnauthenticatedTls => return Err(AuthError::UnauthenticatedTls.into()),
            Posture::Secured => tracing::info!(
                target: "features::auth",
                "TLS enabled, bearer-token auth required",
            ),
            Posture::CleartextToken => tracing::warn!(
                target: "features::auth",
                "bearer token configured without TLS — the token crosses the \
                 wire in cleartext; mount a cert or drop the token",
            ),
            Posture::Open => tracing::warn!(
                target: "features::auth",
                remedy = %format!(
                    "deploy behind a cert plus {} for any non-loopback exposure",
                    var_name("auth", "TOKEN"),
                ),
                "no TLS cert — serving plain HTTP with NO authentication. This \
                 is the intended dev posture.",
            ),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use nest_rs::http::TlsConfig;

    use super::*;

    fn service(tls: bool, token: bool) -> AuthService {
        AuthService {
            config: Arc::new(AuthConfig {
                token: token.then(|| "s3cret".to_string()),
            }),
            http: Arc::new(HttpConfig {
                tls: tls.then(|| TlsConfig::new("cert-pem", "key-pem")),
                ..Default::default()
            }),
        }
    }

    #[test]
    fn tls_without_a_token_is_the_one_posture_that_is_refused() {
        assert_eq!(service(true, false).posture(), Posture::UnauthenticatedTls);
    }

    #[test]
    fn the_other_three_postures_are_served() {
        assert_eq!(service(true, true).posture(), Posture::Secured);
        assert_eq!(service(false, true).posture(), Posture::CleartextToken);
        assert_eq!(service(false, false).posture(), Posture::Open);
    }
}
