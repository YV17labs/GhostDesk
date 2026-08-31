//! The Windows [`InputBackend`](crate::input::InputBackend) — mouse and
//! keyboard through `SendInput`.
//!
//! Three files, mirroring the macOS side: [`win32`] answers the trait,
//! [`event`] holds every `SendInput` call, and [`chord`] plus [`keycode`] turn
//! a published chord name into the virtual keys it presses. There is no
//! connection and no keymap to upload — `SendInput` reaches the window manager
//! directly, and text is injected as Unicode — so the Linux side's
//! `connection`, `session`, `keysym` and `keymap` have no counterpart here.

mod chord;
mod event;
mod keycode;
mod win32;

pub use win32::{CONVENTIONS, Win32};
