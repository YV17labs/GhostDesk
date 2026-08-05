use nest_rs::core::module;

use super::service::AppsService;
use crate::host::HostModule;

#[module(
    imports = [HostModule],
    providers = [AppsService],
)]
pub struct AppsModule;
