use std::sync::Arc;

use nest_rs::authn::{AuthError, Strategy, bearer_token};
use nest_rs::core::{Secret, async_trait, injectable};
use nest_rs::http::Request;

use super::config::AuthConfig;
use super::desk_operator::DeskOperator;

#[injectable]
pub struct TokenStrategy {
    #[inject]
    config: Arc<AuthConfig>,
}

#[async_trait]
impl Strategy for TokenStrategy {
    type Principal = DeskOperator;

    async fn authenticate(&self, req: &mut Request) -> Result<DeskOperator, AuthError> {
        let Some(expected) = self.config.token.as_ref() else {
            return Ok(DeskOperator::anonymous());
        };

        let provided = bearer_token(req.headers()).ok_or(AuthError::MissingCredentials)?;

        // `Secret`'s own equality, which is constant-time over the bytes: a
        // plain `==` on the strings would say, by how long it took, how much of
        // the token a caller already has right. Both sides borrowed, so no
        // request copies the token to compare it.
        (Secret::new(provided) == Secret::new(expected.expose_secret().as_str()))
            .then(DeskOperator::authenticated)
            .ok_or_else(|| AuthError::failed("bearer token mismatch"))
    }
}

#[cfg(test)]
mod tests {
    use nest_rs::authn::PrincipalIdentity;
    use nest_rs::http::header::AUTHORIZATION;

    use super::*;

    fn strategy(token: Option<&str>) -> TokenStrategy {
        TokenStrategy {
            config: Arc::new(AuthConfig {
                token: token.map(Into::into),
            }),
        }
    }

    async fn authenticate(
        token: Option<&str>,
        header: Option<&str>,
    ) -> Result<DeskOperator, AuthError> {
        let mut req = Request::builder();
        if let Some(header) = header {
            req = req.header(AUTHORIZATION, header.parse().expect("a header value"));
        }
        strategy(token).authenticate(&mut req.finish()).await
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
            Err(AuthError::Failed { .. }),
        ));
        assert!(matches!(
            authenticate(Some("s3cret"), Some("Bearer s3cre")).await,
            Err(AuthError::Failed { .. }),
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
