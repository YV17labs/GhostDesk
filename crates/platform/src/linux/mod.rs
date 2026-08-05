//! The Linux backends — Wayland/Sway in a container, one module per seam.
//!
//! Each module here answers the same-named contract in the crate root:
//! `linux::input` implements `input::InputBackend`, `linux::window`
//! implements `window::WindowManager`, and so on. Everything protocol- or
//! distro-specific lives below this point; `host` is the only module that
//! reaches in.

pub mod clipboard;
pub mod desktop;
pub mod input;
pub mod screen;
pub mod window;
