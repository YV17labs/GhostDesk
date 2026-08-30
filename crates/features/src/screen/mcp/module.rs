use nest_rs::core::module;

use super::tool::ScreenTool;
use crate::screen::ScreenModule;

#[module(
    imports = [ScreenModule],
    providers = [ScreenTool],
)]
pub struct ScreenMcpModule;
