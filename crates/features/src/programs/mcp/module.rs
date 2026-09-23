use nest_rs::core::module;

use super::tool::ProgramsTool;
use crate::programs::ProgramsModule;

#[module(
    imports = [ProgramsModule],
    providers = [ProgramsTool],
)]
pub struct ProgramsMcpModule;
