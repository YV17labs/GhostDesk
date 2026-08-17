use std::any::TypeId;

use nest_rs::core::{ContainerBuilder, Module};
use platform::clipboard::Clipboard;
use platform::desktop::AppCatalog;
use platform::host;
use platform::input::InputBackend;
use platform::screen::ScreenBackend;
use platform::window::WindowManager;

/// Binds `platform::host`'s backends into the container.
///
/// Hand-written rather than `#[module]`: the backends are plain platform
/// types — `platform` deliberately knows nothing about NestRS, and wrapping
/// each backend in a delegating provider would be five structs of pure
/// ceremony. Every domain module that injects an `Arc<dyn …>` backend
/// imports this.
pub struct HostModule;

impl Module for HostModule {
    fn register(mut builder: ContainerBuilder) -> ContainerBuilder {
        if !builder.mark_registered(TypeId::of::<HostModule>()) {
            return builder;
        }
        builder
            .provide_dyn::<dyn InputBackend>(host::input())
            .provide_dyn::<dyn ScreenBackend>(host::screen())
            .provide_dyn::<dyn WindowManager>(host::windows())
            .provide_dyn::<dyn Clipboard>(host::clipboard())
            .provide_dyn::<dyn AppCatalog>(host::apps())
    }
}
