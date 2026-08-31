//! The Windows backends — Win32, one module per seam.
//!
//! Each module here answers the same-named contract in the crate root:
//! `windows::input` implements `input::InputBackend`, `windows::window`
//! implements `window::WindowManager`, and so on. Everything Win32-specific
//! lives below this point; `host` is the only module that reaches in.
//!
//! Nothing here needs a helper binary or a second process. Where Linux shells
//! out to `grim` and `wl-paste` and macOS to `screencapture` and `pbcopy`,
//! Windows ships no such tools — GDI, the clipboard API and the shell's own
//! `IShellLink` are called in process instead, which is why this is the one
//! backend with no [`cmd`](crate::cmd) call in it.
//!
//! There is no permission to grant, and that is the substantive difference
//! from macOS: Windows gates nothing behind a privacy setting. What it gates
//! is *integrity* — a process may only send input to windows at or below its
//! own level — so an unelevated server drives every ordinary application and
//! silently reaches none of an elevated one. [`input`] states that at boot
//! rather than letting a click disappear.
//!
//! Two spellings collide in this directory and both are deliberate. `windows`
//! is the target's own name, so the directory takes it exactly as `linux` and
//! `macos` do; the crate of Win32 bindings is also called `windows`, so every
//! file below reaches it as `::windows::…`. The leading `::` is what tells the
//! compiler which of the two is meant, and a reader the same thing.

pub mod clipboard;
pub mod desktop;
pub mod input;
pub mod process;
pub mod screen;
pub mod window;

mod display;
