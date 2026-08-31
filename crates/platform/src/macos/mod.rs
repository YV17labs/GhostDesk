//! The macOS backends — one module per seam, each answering the same-named
//! contract in the crate root.
//!
//! Everything framework-specific lives below this point: Quartz Event
//! Services for input, Accessibility for windows, `screencapture` and the
//! pasteboard tools for the rest. [`display`] is the shared piece the seams
//! disagree about — Quartz points versus captured pixels.
//!
//! Two of these need permissions the user grants by hand, per binary, in
//! System Settings: Accessibility for input and windows, Screen Recording for
//! capture. Neither can be requested from code, so each backend fails with
//! the setting to open rather than appearing to work.

pub mod clipboard;
pub mod desktop;
pub mod input;
pub mod process;
pub mod screen;
pub mod window;

mod display;
