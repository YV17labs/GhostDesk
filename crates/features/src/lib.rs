//! Two files sit outside a module, and the list is closed.
//!
//! [`blame`] is a posture — a decision every adapter has to render identically,
//! where a per-adapter copy that forgets the refusal branch still compiles,
//! still answers, and leaks. `testing` is the doubles, one per platform seam.
//! Anything else landing here is a module that was never drawn.

pub mod auth;
pub(crate) mod blame;
pub mod clipboard;
pub(crate) mod host;
pub mod idle;
pub mod input;
pub mod programs;
pub mod screen;

#[cfg(test)]
mod testing;
