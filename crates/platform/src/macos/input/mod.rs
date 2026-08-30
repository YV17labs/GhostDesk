//! The macOS [`InputBackend`](crate::input::InputBackend) — mouse and
//! keyboard through Quartz Event Services.
//!
//! Three files, mirroring the Linux side: [`quartz`] answers the trait,
//! [`event`] holds every `CGEvent` call, and [`chord`] plus [`keycode`] turn
//! a published chord name into what Quartz wants. There is no connection and
//! no keymap to upload — `CGEventPost` reaches the window server directly,
//! and text is injected as Unicode — so the Linux side's `connection`,
//! `session`, `keysym` and `keymap` have no counterpart here.

mod chord;
mod event;
mod keycode;
mod quartz;

pub use quartz::{CONVENTIONS, Quartz};
