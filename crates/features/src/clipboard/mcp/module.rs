use nest_rs::core::module;

use super::tool::ClipboardTool;
use crate::clipboard::ClipboardModule;

#[module(
    imports = [ClipboardModule],
    providers = [ClipboardTool],
)]
pub struct ClipboardMcpModule;
