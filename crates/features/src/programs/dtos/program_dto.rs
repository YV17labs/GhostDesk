use platform::desktop::DesktopApp;
use schemars::JsonSchema;
use serde::Serialize;

/// One installed GUI program, as `app_list` reports it.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProgramDto {
    /// Human-readable application name.
    pub name: String,
    /// The string to pass to `app_launch`.
    pub exec: String,
}

impl From<DesktopApp> for ProgramDto {
    fn from(app: DesktopApp) -> Self {
        Self {
            name: app.name,
            exec: app.exec,
        }
    }
}
