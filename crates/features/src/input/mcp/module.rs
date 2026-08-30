use nest_rs::core::module;

use super::tool::InputTool;
use crate::input::InputModule;

#[module(
    imports = [InputModule],
    providers = [InputTool],
)]
pub struct InputMcpModule;
