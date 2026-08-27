use std::net::IpAddr;
use std::sync::Arc;

use nest_rs::config::var_name;
use nest_rs::core::{hooks, injectable};
use nest_rs::http::HttpConfig;

use super::config::AuthConfig;
use super::error::AuthError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Posture {
    Secured,
    UnauthenticatedTls,
    CleartextToken,
    LoopbackOpen,
    ExposedOpen,
}

impl Posture {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Secured => "secured",
            Self::UnauthenticatedTls => "unauthenticated_tls",
            Self::CleartextToken => "cleartext_token",
            Self::LoopbackOpen => "loopback_open",
            Self::ExposedOpen => "exposed_open",
        }
    }
}

fn is_loopback(host: &str) -> bool {
    host.parse::<IpAddr>()
        .map(|address| address.is_loopback())
        .unwrap_or_else(|_| host.eq_ignore_ascii_case("localhost"))
}

#[injectable]
pub struct AuthService {
    #[inject]
    config: Arc<AuthConfig>,
    #[inject]
    http: Arc<HttpConfig>,
}

impl AuthService {
    pub fn posture(&self) -> Posture {
        match (self.http.tls.is_some(), self.config.token.is_some()) {
            (true, true) => Posture::Secured,
            (true, false) => Posture::UnauthenticatedTls,
            (false, true) => Posture::CleartextToken,
            (false, false) if is_loopback(&self.http.host) => Posture::LoopbackOpen,
            (false, false) => Posture::ExposedOpen,
        }
    }
}

#[hooks]
impl AuthService {
    #[on_module_init]
    async fn announce_posture(&self) -> anyhow::Result<()> {
        let posture = self.posture();
        match posture {
            Posture::UnauthenticatedTls => return Err(AuthError::UnauthenticatedTls.into()),
            Posture::ExposedOpen => tracing::warn!(
                target: "features::auth",
                posture = posture.as_str(),
                address = %self.http.host,
                remedy = %format!(
                    "set {} before publishing this port anywhere",
                    var_name("auth", "TOKEN"),
                ),
                "serving a routable address with NO authentication — anything \
                 that can reach this port can drive the desktop",
            ),
            Posture::Secured => tracing::info!(
                target: "features::auth",
                posture = posture.as_str(),
                "TLS enabled, bearer-token auth required",
            ),
            Posture::CleartextToken => tracing::warn!(
                target: "features::auth",
                posture = posture.as_str(),
                "bearer token configured without TLS — the token crosses the \
                 wire in cleartext; mount a cert, or terminate TLS in front",
            ),
            Posture::LoopbackOpen => tracing::warn!(
                target: "features::auth",
                posture = posture.as_str(),
                address = %self.http.host,
                remedy = %format!(
                    "set {} before binding anything but loopback",
                    var_name("auth", "TOKEN"),
                ),
                "loopback only, NO authentication. This is the intended dev \
                 posture; a routable bind without a token is refused at boot.",
            ),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use nest_rs::http::TlsConfig;

    use super::*;

    fn service(tls: bool, token: bool, host: &str) -> AuthService {
        AuthService {
            config: Arc::new(AuthConfig {
                token: token.then(|| "s3cret".to_string()),
            }),
            http: Arc::new(HttpConfig {
                tls: tls.then(|| TlsConfig::new("cert-pem", "key-pem")),
                host: host.to_string(),
                ..Default::default()
            }),
        }
    }

    #[test]
    fn tls_without_a_token_is_the_one_posture_that_is_refused() {
        assert_eq!(
            service(true, false, "127.0.0.1").posture(),
            Posture::UnauthenticatedTls,
        );
    }

    #[test]
    fn a_routable_bind_without_a_token_is_served_and_named() {
        assert_eq!(
            service(false, false, "0.0.0.0").posture(),
            Posture::ExposedOpen
        );
    }

    #[test]
    fn the_served_postures() {
        assert_eq!(service(true, true, "0.0.0.0").posture(), Posture::Secured);
        assert_eq!(
            service(false, true, "0.0.0.0").posture(),
            Posture::CleartextToken,
        );
        assert_eq!(
            service(false, false, "127.0.0.1").posture(),
            Posture::LoopbackOpen,
        );
    }

    #[test]
    fn a_token_makes_a_routable_bind_servable() {
        assert_ne!(
            service(false, true, "0.0.0.0").posture(),
            Posture::ExposedOpen
        );
    }

    #[test]
    fn loopback_is_recognised_by_address_and_by_name_and_nothing_else_is() {
        assert!(is_loopback("127.0.0.1"));
        assert!(is_loopback("127.0.0.7"));
        assert!(is_loopback("::1"));
        assert!(is_loopback("localhost"));
        assert!(is_loopback("LOCALHOST"));
        assert!(!is_loopback("0.0.0.0"));
        assert!(!is_loopback("192.168.1.10"));
        assert!(!is_loopback("::"));
        assert!(
            !is_loopback("desktop.internal"),
            "a name this process cannot resolve is routable until proven otherwise",
        );
    }
}
