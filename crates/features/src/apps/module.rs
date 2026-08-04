use nest_rs::core::module;

use super::service::AppsService;

#[module(providers = [AppsService])]
pub struct AppsModule;
