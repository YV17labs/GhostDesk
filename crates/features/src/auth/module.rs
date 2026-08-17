use nest_rs::config::ConfigModule;
use nest_rs::core::module;

use super::config::AuthConfig;
use super::service::AuthService;

#[module(
    imports = [ConfigModule::for_feature::<AuthConfig>()],
    providers = [AuthService],
)]
pub struct AuthModule;
