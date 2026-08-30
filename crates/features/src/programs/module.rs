use nest_rs::core::module;

use super::registry::LaunchRegistry;
use super::service::ProgramsService;
use crate::host::HostModule;

#[module(
    imports = [HostModule],
    providers = [LaunchRegistry, ProgramsService],
)]
pub struct ProgramsModule;
