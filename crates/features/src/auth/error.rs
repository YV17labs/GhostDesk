//! The one thing auth refuses outright.
//!
//! The variable is named by building it, never by spelling it: the prefix is
//! set on the process and renames every framework variable at once, so a name
//! typed by hand points at nothing the day it changes and the compiler never
//! notices.

use nest_rs::config::var_name;

/// A refusal from the auth domain.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// TLS with no token — an open desktop over HTTPS. Boot-fatal.
    #[error(
        "{} is required when TLS is enabled — an HTTPS endpoint with no \
         bearer token is an open desktop",
        var_name("auth", "TOKEN")
    )]
    UnauthenticatedTls,
}
