use nest_rs::core::module;

use super::service::ScreenService;
use crate::host::HostModule;

#[module(
    imports = [HostModule],
    providers = [ScreenService],
)]
pub struct ScreenModule;
