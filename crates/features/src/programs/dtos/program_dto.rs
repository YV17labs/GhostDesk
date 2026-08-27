use platform::desktop::DesktopApp;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProgramDto {
    pub name: String,
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
