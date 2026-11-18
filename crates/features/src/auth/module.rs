use nest_rs::core::module;

use super::guard::AuthnGuard;
use super::service::AuthService;
use super::strategy::TokenStrategy;

#[module(
    providers = [AuthService, TokenStrategy, AuthnGuard],
)]
pub struct AuthModule;
