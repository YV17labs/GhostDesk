use nest_rs::core::module;

use super::service::ClipboardService;

#[module(providers = [ClipboardService])]
pub struct ClipboardModule;
