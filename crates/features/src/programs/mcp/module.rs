use nest_rs::core::module;

use super::tool::ProgramsTool;
use crate::programs::ProgramsModule;
use crate::screen::ScreenModule;

#[module(
    imports = [ProgramsModule, ScreenModule],
    providers = [ProgramsTool],
)]
pub struct ProgramsMcpModule;
