use nest_rs::config::var_name;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error(
        "{} is required when TLS is enabled — an HTTPS endpoint with no \
         bearer token is an open desktop",
        var_name("auth", "TOKEN")
    )]
    UnauthenticatedTls,
}
