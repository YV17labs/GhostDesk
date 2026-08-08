use schemars::JsonSchema;
use serde::Serialize;

use super::WindowDto;
use crate::programs::service::Launched;

/// What `app_launch` answers with.
///
/// `window` and `window_wait_ms` are filled in by the tool, not by the
/// service: waiting for a window is a composition of two domain calls, and
/// the shape that carries both back is the wire's, not the domain's.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct LaunchedDto {
    pub pid: u32,
    pub log_file: String,
    pub action: String,
    /// The window the launch produced, when the call waited for one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowDto>,
    /// How long that window took to appear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_wait_ms: Option<u64>,
}

impl From<Launched> for LaunchedDto {
    fn from(launched: Launched) -> Self {
        Self {
            pid: launched.pid,
            log_file: launched.log_file,
            action: launched.action,
            window: None,
            window_wait_ms: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::programs::service::RunningWindow;

    fn launched() -> Launched {
        Launched {
            pid: 2737,
            log_file: "/tmp/ghostdesk/proc-2737.log".into(),
            action: "Launched: firefox".into(),
        }
    }

    #[test]
    fn a_launch_that_did_not_wait_omits_both_window_fields_entirely() {
        assert_eq!(
            serde_json::to_value(LaunchedDto::from(launched())).unwrap(),
            serde_json::json!({
                "pid": 2737,
                "log_file": "/tmp/ghostdesk/proc-2737.log",
                "action": "Launched: firefox",
            }),
        );
    }

    #[test]
    fn a_launch_that_waited_carries_its_window_inline() {
        let mut launch = LaunchedDto::from(launched());
        launch.window = Some(WindowDto::from(RunningWindow {
            app: "firefox".into(),
            title: "Example".into(),
            pid: 2737,
            focused: true,
        }));
        launch.window_wait_ms = Some(1200);

        let json = serde_json::to_value(&launch).unwrap();
        assert_eq!(json["window"]["title"], "Example");
        assert_eq!(json["window_wait_ms"], 1200);
    }
}
