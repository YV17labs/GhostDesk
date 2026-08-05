use nest_rs::core::module;

use super::service::ClipboardService;
use crate::host::HostModule;

#[module(
    imports = [HostModule],
    providers = [ClipboardService],
)]
pub struct ClipboardModule;
