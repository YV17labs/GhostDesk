use platform::window::WindowInfo;

/// One window open on the desktop, as this domain reports it.
///
/// The platform's `WindowInfo` carries an opaque `WindowId` that only the
/// backend that made it can use; dropping it here is deliberate — nothing in
/// this domain closes a window, so carrying an address it must not act on
/// would be an invitation.
#[derive(Debug, Clone)]
pub struct RunningWindow {
    pub app: String,
    pub title: String,
    pub pid: i64,
    pub focused: bool,
}

impl From<WindowInfo> for RunningWindow {
    fn from(window: WindowInfo) -> Self {
        Self {
            app: window.app,
            title: window.title,
            pid: window.pid,
            focused: window.focused,
        }
    }
}
