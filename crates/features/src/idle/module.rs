use nest_rs::core::module;

use super::service::IdleService;
use crate::host::HostModule;

#[module(
    imports = [HostModule],
    providers = [IdleService],
)]
pub struct IdleModule;
