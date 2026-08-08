use nest_rs::core::module;

use super::service::ProgramsService;
use crate::host::HostModule;

#[module(
    imports = [HostModule],
    providers = [ProgramsService],
)]
pub struct ProgramsModule;
