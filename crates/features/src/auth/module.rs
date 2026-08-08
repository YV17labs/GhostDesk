use nest_rs::config::ConfigModule;
use nest_rs::core::module;

use super::config::AuthConfig;
use super::service::AuthService;

/// The rule about the endpoint's security posture, and nothing about the
/// transport that happens to enforce it — that is
/// [`AuthMcpModule`](super::mcp::AuthMcpModule)'s.
#[module(
    imports = [ConfigModule::for_feature::<AuthConfig>()],
    providers = [AuthService],
)]
pub struct AuthModule;
