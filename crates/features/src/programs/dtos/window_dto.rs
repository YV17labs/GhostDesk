use schemars::JsonSchema;
use serde::Serialize;

use crate::programs::running_window::RunningWindow;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WindowDto {
    pub app: String,
    pub title: String,
    pub pid: i64,
    pub focused: bool,
}

impl From<RunningWindow> for WindowDto {
    fn from(window: RunningWindow) -> Self {
        Self {
            app: window.app,
            title: window.title,
            pid: window.pid,
            focused: window.focused,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_published_window_shape_is_what_clients_already_read() {
        let window = WindowDto::from(RunningWindow {
            app: "firefox".into(),
            title: "Example".into(),
            pid: 2737,
            focused: true,
        });
        assert_eq!(
            serde_json::to_value(&window).unwrap(),
            serde_json::json!({
                "app": "firefox", "title": "Example", "pid": 2737, "focused": true,
            }),
        );
    }
}
