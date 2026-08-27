use std::sync::Arc;

use nest_rs::authn::{AuthError, Strategy, bearer_token};
use nest_rs::core::{async_trait, injectable};
use nest_rs::http::poem::Request;
use subtle::ConstantTimeEq;

use super::config::AuthConfig;
use super::principal::DeskOperator;

#[injectable]
pub struct TokenStrategy {
    #[inject]
    config: Arc<AuthConfig>,
}

#[async_trait]
impl Strategy for TokenStrategy {
    type Principal = DeskOperator;

    async fn authenticate(&self, req: &mut Request) -> Result<DeskOperator, AuthError> {
        let Some(expected) = self.config.token.as_deref() else {
            return Ok(DeskOperator::anonymous());
        };

        let provided = bearer_token(req).ok_or(AuthError::MissingCredentials)?;

        bool::from(provided.as_bytes().ct_eq(expected.as_bytes()))
            .then(DeskOperator::authenticated)
            .ok_or_else(|| AuthError::Failed("bearer token mismatch".into()))
    }
}

#[cfg(test)]
mod tests {
    use nest_rs::authn::PrincipalIdentity;

    use super::*;

    fn strategy(token: Option<&str>) -> TokenStrategy {
        TokenStrategy {
            config: Arc::new(AuthConfig {
                token: token.map(str::to_owned),
            }),
        }
    }

    async fn authenticate(
        token: Option<&str>,
        header: Option<&str>,
    ) -> Result<DeskOperator, AuthError> {
        let mut req = Request::default();
        if let Some(header) = header {
            req.headers_mut().insert(
                nest_rs::http::poem::http::header::AUTHORIZATION,
                header.parse().expect("a header value"),
            );
        }
        strategy(token).authenticate(&mut req).await
    }

    #[tokio::test]
    async fn the_exact_token_authenticates_the_operator() {
        let principal = authenticate(Some("s3cret"), Some("Bearer s3cret")).await;
        assert_eq!(
            PrincipalIdentity::actor_id(&principal.expect("authenticated")),
            Some("desk-operator".to_owned()),
        );
    }

    #[tokio::test]
    async fn the_scheme_is_matched_without_regard_to_case() {
        assert!(
            authenticate(Some("s3cret"), Some("bearer s3cret"))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn a_wrong_or_missing_token_is_refused() {
        assert!(matches!(
            authenticate(Some("s3cret"), Some("Bearer nope")).await,
            Err(AuthError::Failed(_)),
        ));
        assert!(matches!(
            authenticate(Some("s3cret"), Some("Bearer s3cre")).await,
            Err(AuthError::Failed(_)),
        ));
        assert!(matches!(
            authenticate(Some("s3cret"), None).await,
            Err(AuthError::MissingCredentials),
        ));
    }

    #[tokio::test]
    async fn an_unconfigured_token_admits_an_anonymous_caller() {
        let principal = authenticate(None, None).await.expect("admitted");
        assert_eq!(PrincipalIdentity::actor_id(&principal), None);
    }
}
