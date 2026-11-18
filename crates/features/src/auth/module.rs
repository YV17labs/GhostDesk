use nest_rs::core::module;
use nest_rs::http::HttpModule;

use super::guard::AuthnGuard;
use super::service::AuthService;
use super::strategy::TokenStrategy;

/// `HttpModule` is imported for the `HttpConfig` the posture is read off: the
/// bind address and the certificate decide whether a token is owed. Bare, so it
/// declares the dependency and pins nothing — the app's root pins the server.
#[module(
    imports = [HttpModule],
    providers = [AuthService, TokenStrategy, AuthnGuard],
)]
pub struct AuthModule;
