use std::sync::Arc;

use nest_rs::config::var_name;
use nest_rs::core::{hooks, injectable};
use nest_rs::http::HttpConfig;

use super::config::AuthConfig;
use super::error::AuthError;
use super::posture::Posture;

#[injectable]
pub struct AuthService {
    #[inject]
    config: Arc<AuthConfig>,
    #[inject]
    http: Arc<HttpConfig>,
}

impl AuthService {
    pub fn posture(&self) -> Posture {
        Posture::of(
            self.http.tls.is_some(),
            self.config.token.is_some(),
            &self.http.host,
        )
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
    use nest_rs::testing::LogCapture;

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

    /// The classification itself is `posture.rs`'s; what this pins is that the
    /// service hands it the three inputs in the order it expects — a swapped
    /// pair would read a certificate as a token and serve an open desktop.
    #[test]
    fn the_service_reads_its_posture_off_tls_token_and_bind_address() {
        assert_eq!(
            service(true, false, "127.0.0.1").posture(),
            Posture::UnauthenticatedTls,
        );
        assert_eq!(
            service(false, true, "0.0.0.0").posture(),
            Posture::CleartextToken,
        );
        assert_eq!(
            service(false, false, "0.0.0.0").posture(),
            Posture::ExposedOpen,
        );
        assert_eq!(service(true, true, "0.0.0.0").posture(), Posture::Secured);
    }

    #[tokio::test]
    async fn an_https_endpoint_with_no_token_refuses_to_boot() {
        assert!(
            service(true, false, "0.0.0.0")
                .announce_posture()
                .await
                .is_err(),
        );
    }

    #[tokio::test]
    async fn every_served_posture_boots_and_says_which_it_is() {
        let logs = LogCapture::install();
        for (tls, token, host) in [
            (true, true, "0.0.0.0"),
            (false, true, "0.0.0.0"),
            (false, false, "127.0.0.1"),
            (false, false, "0.0.0.0"),
        ] {
            service(tls, token, host)
                .announce_posture()
                .await
                .expect("served");
        }

        let announced: Vec<String> = logs
            .events()
            .into_iter()
            .filter_map(|event| event.field("posture"))
            .collect();
        assert_eq!(
            announced,
            [
                "secured",
                "cleartext_token",
                "loopback_open",
                "exposed_open",
            ],
            "each served posture announces itself by the name the trail carries",
        );
    }
}
