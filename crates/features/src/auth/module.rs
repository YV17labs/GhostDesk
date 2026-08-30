use nest_rs::config::ConfigModule;
use nest_rs::core::module;

use super::config::AuthConfig;
use super::guard::AuthnGuard;
use super::service::AuthService;
use super::strategy::TokenStrategy;

#[module(
    imports = [ConfigModule::for_feature::<AuthConfig>()],
    providers = [AuthService, TokenStrategy, AuthnGuard],
)]
pub struct AuthModule;
